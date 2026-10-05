use super::skin::{floats, skin_mask, skin_mask_passes, supplied_skin_mask, supplied_skin_mask_aux};
use crate::common::*;

use abra_core::{
  IntoNumber,
  image::gpu::{GpuPass, GpuProcessor},
};
use adjustments::levels::{Exposure, exposure};
use mask::Mask;
use options::Effect;

/// Lightens or darkens existing skin tones without adding a tint. Create one with [`adjust_skin_tone`].
/// Uses the shared skin detection and feathering, or a supplied segmentation mask, on both CPU and GPU.
#[derive(Clone)]
pub struct SkinTone {
  amount: f64,
  feather: f32,
  mask: Option<Mask>,
  options: Options,
}

impl SkinTone {
  /// Replaces color-based skin detection with a supplied grayscale mask, resized and feathered to fit the image.
  pub fn with_mask(mut self, p_mask: Mask) -> Self {
    self.mask = Some(p_mask);
    self
  }

  /// Sets the edge fade as a multiple of the standard skin fade. Defaults to `3.0`.
  pub fn with_feather(mut self, p_feather: impl IntoNumber) -> Self {
    self.feather = p_feather.into::<f32>().max(0.0);
    self
  }

  fn adjustment(&self) -> Exposure {
    // Adjust midtones with the existing gamma curve: black and white stay fixed, so lightening does not clip highlights.
    exposure(0.0).with_gamma(1.4_f64.powf(-self.amount / 100.0))
  }
}

impl Effect for SkinTone {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn positional(&self) -> bool {
    true
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    if self.amount == 0.0 {
      return;
    }
    let mask = self
      .mask
      .as_ref()
      .map(|mask| supplied_skin_mask(p_image, mask, self.feather))
      .unwrap_or_else(|| skin_mask(p_image, self.feather));
    let mut adjusted = p_image.clone();
    self.adjustment().apply_on_cpu(&mut adjusted);
    let mut pixels = p_image.to_rgba_vec();
    pixels.par_chunks_exact_mut(4).zip(adjusted.rgba().par_chunks_exact(4)).zip(mask.par_iter()).for_each(
      |((pixel, toned), skin)| {
        for channel in 0..3 {
          let original = pixel[channel] as f32;
          pixel[channel] = (original + (toned[channel] as f32 - original) * skin).round().clamp(0.0, 255.0) as u8;
        }
      },
    );
    p_image.set_rgba(pixels);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for SkinTone {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    if self.amount == 0.0 {
      return Vec::new();
    }
    let mut passes = if self.mask.is_some() { Vec::new() } else { skin_mask_passes(p_width, p_height, self.feather) };
    // Exposure preserves the alpha channel carrying the detected skin mask.
    passes.extend(self.adjustment().passes(p_width, p_height));
    let blend = if let Some(mask) = &self.mask {
      GpuPass::new(include_str!("./skin_blend_mask.wgsl"), floats(&[1.0]))
        .with_aux(supplied_skin_mask_aux(mask, p_width, p_height, self.feather))
        .with_base()
    } else {
      GpuPass::new(include_str!("./skin_blend.wgsl"), floats(&[1.0])).with_base()
    };
    passes.push(blend);
    passes
  }
}

/// Adjusts skin relative to its original tone, preserving texture and alpha.
/// - `p_amount`: Adjustment strength: negative lightens, positive darkens, and zero changes nothing.
///   The amount is not range-limited; callers choose and enforce their own control range.
///
/// Uses the existing exposure effect's gamma curve with no color tint. Skin detection and mask feathering match
/// [`super::tan_skin`]. Black and white stay fixed, retaining highlight detail.
pub fn skin_tone(p_amount: impl IntoNumber) -> SkinTone {
  SkinTone {
    amount: p_amount.into::<f64>(),
    feather: 3.0,
    mask: None,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Color, Image};

  #[test]
  fn zero_is_an_exact_no_op_with_no_gpu_passes() {
    let mut image = Image::new_from_color(32, 32, Color::from_rgba(180, 140, 100, 123));
    let before = image.to_rgba_vec();
    skin_tone(0).apply_on_cpu(&mut image);
    assert_eq!(image.rgba(), before);
    assert!(skin_tone(0).passes(32, 32).is_empty());
  }

  #[test]
  fn signed_amounts_change_skin_but_preserve_blue_and_alpha() {
    let mut source = Image::new_from_color(64, 48, Color::from_rgba(180, 140, 100, 123));
    for y in 0..48 {
      for x in 32..64 {
        source.set_pixel(x, y, (60, 100, 200, 77));
      }
    }
    let render = |amount| {
      let mut image = source.clone();
      skin_tone(amount).apply_on_cpu(&mut image);
      image
    };
    let light = render(-100);
    let dark = render(100);
    let original = source.get_pixel(12, 24).unwrap();
    let lighter = light.get_pixel(12, 24).unwrap();
    let darker = dark.get_pixel(12, 24).unwrap();
    assert!(lighter.0 > original.0 && lighter.1 > original.1 && lighter.2 > original.2);
    assert!(darker.0 < original.0 && darker.1 < original.1 && darker.2 < original.2);
    for image in [&light, &dark] {
      assert_eq!(image.get_pixel(52, 24), source.get_pixel(52, 24));
      assert!(image.rgba().chunks_exact(4).zip(source.rgba().chunks_exact(4)).all(|(a, b)| a[3] == b[3]));
    }
    assert!(render(-200).get_pixel(12, 24).unwrap().0 > lighter.0, "core allows stronger lightening");
    assert!(render(200).get_pixel(12, 24).unwrap().0 < darker.0, "core allows stronger darkening");
  }

  #[test]
  fn supplied_masks_limit_the_effect_and_keep_shading_and_undertone() {
    let mut source = Image::new_from_color(64, 48, Color::from_rgb(120, 90, 60));
    source.set_pixel(12, 24, (150, 112, 75, 255));
    let white = Mask::from_image(Image::new_from_color(64, 48, Color::white()));
    let black = Mask::from_image(Image::new_from_color(64, 48, Color::black()));
    for amount in [-100, 100] {
      let mut unchanged = source.clone();
      skin_tone(amount).with_mask(black.clone()).apply_on_cpu(&mut unchanged);
      assert_eq!(unchanged.rgba(), source.rgba());
      let mut toned = source.clone();
      skin_tone(amount).with_mask(white.clone()).apply_on_cpu(&mut toned);
      let shadow = toned.get_pixel(20, 24).unwrap();
      let highlight = toned.get_pixel(12, 24).unwrap();
      assert!(highlight.0 > shadow.0 && highlight.1 > shadow.1 && highlight.2 > shadow.2);
      assert!(shadow.0 > shadow.1 && shadow.1 > shadow.2, "the warm undertone remains");
    }
  }

  #[test]
  fn lightening_retains_highlight_detail_instead_of_clipping_to_white() {
    let mask = Mask::from_image(Image::new_from_color(32, 32, Color::white()));
    let mut image = Image::new_from_color(32, 32, Color::from_rgb(230, 220, 210));
    image.set_pixel(16, 16, (250, 240, 230, 255));
    skin_tone(-100).with_mask(mask).apply_on_cpu(&mut image);
    let shadow = image.get_pixel(12, 16).unwrap();
    let highlight = image.get_pixel(16, 16).unwrap();
    assert!(highlight.0 > shadow.0 && highlight.1 > shadow.1 && highlight.2 > shadow.2);
    assert!(highlight.0 < 255 && highlight.1 < 255 && highlight.2 < 255);
  }
}
