use super::skin::{floats, ints, skin_mask, skin_mask_passes, supplied_skin_mask, supplied_skin_mask_aux};
use crate::common::*;

use crate::blur::surface_blur;
use abra_core::{
  IntoNumber,
  image::gpu::{GpuPass, GpuProcessor},
};
use mask::Mask;
use options::Effect;

// The smoothing is a surface blur, which blurs flat areas and stays sharp at edges. Its radius is a fraction of the long
// side of the photo (at least `RADIUS_MIN_PX` before the amount strengthens it), so a photo and a smaller copy of it
// get the same look. The blur looks at
// at most `SAMPLE_RADIUS_MAX` pixels on each side of a pixel, so its cost does not grow with the photo: past that the
// pixels it looks at are spaced out, which reaches as far for the same cost. `SURFACE_THRESHOLD` is how different two
// colors can be (0-255) and still be mixed.
const RADIUS_FRACTION: f32 = 0.005;
const RADIUS_MIN_PX: u32 = 2;
const SAMPLE_RADIUS_MAX: u32 = 10;
const SURFACE_THRESHOLD: u8 = 30;

// An amount up to 1 is how much of the smoothing shows. An amount past 1 shows all of it and makes the smoothing itself
// stronger, by that factor, up to `MAX_AMOUNT`: the radius and the threshold are both multiplied by it, and the
// threshold is held to `BOOSTED_THRESHOLD_MAX`.
const MAX_AMOUNT: f32 = 3.0;
const BOOSTED_THRESHOLD_MAX: u32 = 90;

/// Smooths skin and leaves the features on it sharp. Create one with [`smooth_skin`].
///
/// By default, it uses a color-based mask of skin pixels away from image edges. Use [`with_mask`](Self::with_mask) to
/// supply a replacement mask, such as one from a segmentation model. It blurs the photo with a surface blur and mixes
/// the result through the mask. Both modes can run on the GPU when available.
#[derive(Clone)]
pub struct SmoothSkin {
  amount: f32,
  feather: f32,
  mask: Option<Mask>,
  options: Options,
}

impl SmoothSkin {
  /// How far the edge of the smoothing fades out, as a multiple of the standard fade:
  /// `1.0` is the standard and `2.0` is twice as wide. Below `0.0` counts as `0.0`, the narrowest.
  pub fn with_feather(mut self, p_feather: impl IntoNumber) -> Self {
    self.feather = p_feather.into::<f32>().max(0.0);
    self
  }

  /// Replaces color-based skin detection with a supplied grayscale mask.
  /// The mask is resized to the image dimensions and softened using `with_feather`.
  pub fn with_mask(mut self, p_mask: Mask) -> Self {
    self.mask = Some(p_mask);
    self
  }
}

impl Effect for SmoothSkin {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  /// The radius and the edge measurement come from the size of the whole photo, so the effect runs over all of it and
  /// is then limited to an area and mask, instead of on a crop that would give them different values.
  fn positional(&self) -> bool {
    true
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let blend = self.amount.min(1.0);
    if blend <= 0.0 {
      return;
    }
    let boost = self.amount.max(1.0);
    let (w, h) = p_image.dimensions::<u32>();
    let long_side = w.max(h) as f32;
    let radius = (((long_side * RADIUS_FRACTION).round() as u32).max(RADIUS_MIN_PX) as f32 * boost).round() as u32;
    let step = radius.div_ceil(SAMPLE_RADIUS_MAX);
    let threshold = ((SURFACE_THRESHOLD as f32 * boost).round() as u32).min(BOOSTED_THRESHOLD_MAX) as u8;

    let mask = self
      .mask
      .as_ref()
      .map(|mask| supplied_skin_mask(p_image, mask, self.feather))
      .unwrap_or_else(|| skin_mask(p_image, 1.0));

    // Mix the blurred photo in through the mask, as much as the amount allows.
    let mut smoothed = p_image.clone();
    surface_blur(radius.div_ceil(step), threshold).with_step(step as usize).apply_on_cpu(&mut smoothed);
    let mut pixels = p_image.to_rgba_vec();
    pixels.par_chunks_exact_mut(4).zip(smoothed.rgba().par_chunks_exact(4)).zip(mask.par_iter()).for_each(
      |((pixel, blurred), skin)| {
        let weight = skin * blend;
        for channel in 0..3 {
          let original = pixel[channel] as f32;
          pixel[channel] = (original + (blurred[channel] as f32 - original) * weight).round().clamp(0.0, 255.0) as u8;
        }
      },
    );
    p_image.set_rgba(pixels);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for SmoothSkin {
  /// The same steps as `cpu_processor`, as shader passes. The mask travels in the alpha channel from pass to pass and
  /// the original photo, which has the real alpha, comes back for the last pass. Every number the shaders use comes from
  /// the constants above, as uniforms, so the two cannot drift apart.
  ///
  /// 1 to 6. The skin mask, from [`skin_mask_passes`]. 7. `surface.wgsl`: the surface blur of the color channels.
  /// 8. `skin_blend.wgsl`: mixes it in through the mask.
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let blend = self.amount.min(1.0);
    if blend <= 0.0 {
      return Vec::new();
    }
    let boost = self.amount.max(1.0);
    let long_side = p_width.max(p_height) as f32;
    let radius = (((long_side * RADIUS_FRACTION).round() as u32).max(RADIUS_MIN_PX) as f32 * boost).round() as u32;
    let step = radius.div_ceil(SAMPLE_RADIUS_MAX);
    let threshold = ((SURFACE_THRESHOLD as f32 * boost).round() as u32).min(BOOSTED_THRESHOLD_MAX);

    let mut passes = if self.mask.is_some() {
      Vec::new()
    } else {
      skin_mask_passes(p_width, p_height, 1.0)
    };
    passes.push(GpuPass::new(include_str!("../blur/surface.wgsl"), ints(&[radius.div_ceil(step), threshold, step])));
    let blend_pass = if let Some(mask) = &self.mask {
      GpuPass::new(include_str!("./skin_blend_mask.wgsl"), floats(&[blend]))
        .with_aux(supplied_skin_mask_aux(mask, p_width, p_height, self.feather))
        .with_base()
    } else {
      GpuPass::new(include_str!("./skin_blend.wgsl"), floats(&[blend])).with_base()
    };
    passes.push(blend_pass);
    passes
  }
}

/// Smooths skin and leaves the features on it sharp.
/// - `p_amount`: From `0.0` to `3.0`. Up to `1.0` it is how much of the smoothing shows. Past `1.0` the smoothing itself
///   gets stronger by that factor, which gives a blurrier, flatter skin.
pub fn smooth_skin(p_amount: impl IntoNumber) -> SmoothSkin {
  SmoothSkin {
    amount: p_amount.into::<f64>().clamp(0.0, MAX_AMOUNT as f64) as f32,
    feather: 10.0,
    mask: None,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Area, Channels, Image};
  use mask::Mask;
  use options::ApplyOptions;

  const SKIN: (u8, u8, u8) = (230, 190, 150);

  /// A `p_width` x `p_height` image of `p_color` with a little deterministic grain, like pores, on top.
  fn grainy(p_width: u32, p_height: u32, p_color: (u8, u8, u8)) -> Image {
    let mut img = Image::new(p_width, p_height);
    for y in 0..p_height {
      for x in 0..p_width {
        let grain = ((x * 37 + y * 91) % 13) as i32 - 6;
        let shift = |v: u8| (v as i32 + grain).clamp(0, 255) as u8;
        img.set_pixel(x, y, (shift(p_color.0), shift(p_color.1), shift(p_color.2), 255));
      }
    }
    img
  }

  /// The spread of the red channel over a block of pixels, a stand-in for how grainy it is.
  fn grain(p_img: &Image, p_x: std::ops::Range<usize>, p_y: std::ops::Range<usize>) -> f32 {
    let (w, _) = p_img.dimensions::<usize>();
    let rgba = p_img.rgba();
    let values: Vec<f32> =
      p_y.flat_map(|y| p_x.clone().map(move |x| (x, y))).map(|(x, y)| rgba[(y * w + x) * 4] as f32).collect();
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    (values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32).sqrt()
  }

  #[test]
  fn skin_colors_are_smoothed_and_other_colors_are_left_alone() {
    // Five bands side by side, each grainy: skin, a white like the white of an eye, dark hair, red lips, and blue.
    let colors = [SKIN, (240, 240, 245), (40, 30, 25), (190, 40, 60), (60, 100, 200)];
    let mut img = Image::new(5 * 24, 48);
    for (band, color) in colors.iter().enumerate() {
      let patch = grainy(24, 48, *color);
      for y in 0..48u32 {
        for x in 0..24u32 {
          img.set_pixel(band as u32 * 24 + x, y, patch.get_pixel(x, y).unwrap());
        }
      }
    }
    let original = img.clone();
    smooth_skin(1.0).apply(&mut img);
    // Away from the borders between bands, only the skin band changes.
    let middle = |band: usize| band * 24 + 8..band * 24 + 16;
    assert!(grain(&img, middle(0), 8..40) < grain(&original, middle(0), 8..40) * 0.7, "skin is smoothed");
    // Dark hair is just bright enough, and just warm enough, to count as a little skin, so it may soften a few percent.
    assert!(grain(&img, middle(2), 8..40) > grain(&original, middle(2), 8..40) * 0.9, "dark hair barely changes");
    for band in [1, 3, 4] {
      assert_eq!(grain(&img, middle(band), 8..40), grain(&original, middle(band), 8..40), "band {band} is untouched");
    }
  }

  #[test]
  fn smoothing_softens_grain_but_keeps_an_eyebrow_sharp() {
    let mut img = grainy(64, 64, SKIN);
    for x in 8..56u32 {
      for y in 31..33u32 {
        img.set_pixel(x, y, (50, 35, 30, 255));
      }
    }
    let before = grain(&img, 4..28, 4..28);
    let line_before = img.get_pixel(32, 32).unwrap();
    smooth_skin(1.0).apply(&mut img);
    let after = grain(&img, 4..28, 4..28);
    assert!(after < before * 0.7, "the grain should drop: {before} -> {after}");
    let line_after = img.get_pixel(32, 32).unwrap();
    assert!(
      (line_after.0 as i32 - line_before.0 as i32).abs() < 12,
      "the line should stay dark: {} -> {}",
      line_before.0,
      line_after.0
    );
  }

  #[test]
  fn a_higher_amount_smooths_more_and_still_keeps_the_eyebrow() {
    let mut base = grainy(64, 64, SKIN);
    for x in 8..56u32 {
      for y in 31..33u32 {
        base.set_pixel(x, y, (50, 35, 30, 255));
      }
    }
    let smoothed = |amount: f64| {
      let mut img = base.clone();
      smooth_skin(amount).apply(&mut img);
      img
    };
    let (one, three) = (smoothed(1.0), smoothed(3.0));
    let (grain_one, grain_three) = (grain(&one, 4..28, 4..28), grain(&three, 4..28, 4..28));
    assert!(grain_three < grain_one, "more amount, less grain: {grain_one} -> {grain_three}");
    let line = three.get_pixel(32, 32).unwrap().0 as i32;
    assert!((line - 50).abs() < 14, "the line stays dark even at the largest amount: {line}");
    assert_eq!(smoothed(9.0).to_rgba_vec(), three.to_rgba_vec(), "an amount past 3 is limited to 3");
  }

  #[test]
  fn zero_amount_changes_nothing() {
    let mut img = grainy(32, 32, SKIN);
    let original = img.to_rgba_vec();
    smooth_skin(0.0).apply(&mut img);
    assert_eq!(img.to_rgba_vec(), original);
  }

  #[test]
  fn a_given_mask_limits_the_smoothing_too() {
    let mut img = grainy(40, 40, SKIN);
    let original = img.to_rgba_vec();
    // A mask that is black everywhere: nothing may change.
    let black: Vec<u8> = (0..40 * 40).flat_map(|_| [0u8, 0, 0, 255]).collect();
    let mask = Mask::from_image(Image::new_from_pixels(40, 40, black, Channels::RGBA));
    smooth_skin(1.0).with_options(ApplyOptions::new().with_mask(mask)).apply(&mut img);
    assert_eq!(img.to_rgba_vec(), original);
  }

  #[test]
  fn supplied_mask_replaces_color_detection() {
    let mut image = grainy(32, 32, (60, 100, 200));
    let before = grain(&image, 4..28, 4..28);
    let white = vec![255u8, 255, 255, 255].repeat(32 * 32);
    let mask = Mask::from_image(Image::new_from_pixels(32, 32, white, Channels::RGBA));
    let smoothing = smooth_skin(1.0).with_mask(mask);

    assert!(smoothing.gpu_processor().is_some());
    assert!(smoothing.passes(32, 32).last().unwrap().aux.is_some());
    smoothing.apply_on_cpu(&mut image);

    assert!(grain(&image, 4..28, 4..28) < before * 0.7);
  }

  #[test]
  fn an_area_limits_the_smoothing_but_not_what_it_sees() {
    let whole_image = {
      let mut img = grainy(64, 64, SKIN);
      smooth_skin(1.0).apply(&mut img);
      img
    };
    let mut in_area = grainy(64, 64, SKIN);
    let original = in_area.to_rgba_vec();
    smooth_skin(1.0)
      .with_options(ApplyOptions::new().with_area(Area::rect((0.0, 0.0), (32.0, 64.0))))
      .apply(&mut in_area);
    let (area, whole) = (in_area.to_rgba_vec(), whole_image.to_rgba_vec());
    for y in 0..64usize {
      for x in 0..64usize {
        let i = (y * 64 + x) * 4;
        let expected = if x < 32 { &whole[i..i + 3] } else { &original[i..i + 3] };
        assert_eq!(&area[i..i + 3], expected, "pixel {x},{y}");
      }
    }
  }
}
