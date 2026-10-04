use super::skin::{floats, skin_mask, skin_mask_passes};
use crate::common::*;

use abra_core::image::gpu::{GpuPass, GpuProcessor};
use abra_core::{Color, IntoNumber};
use options::Effect;

/// Tans skin. Create one with [`tan_skin`].
///
/// It works out the same mask as [`smooth_skin`](super::smooth_skin) (skin-colored pixels that are not on or near an
/// edge) and multiplies the photo by the tan color through it. Multiplying darkens and warms the skin while keeping its
/// shading and texture, which a flat mix toward the color would wash out. On the GPU the same steps run as a chain of
/// shader passes and the photo never leaves the GPU.
#[derive(Clone)]
pub struct TanSkin {
  color: Color,
  feather: f32,
  options: Options,
}

impl TanSkin {
  /// How far the edge of the tan fades out, as a multiple of the standard fade used by [`smooth_skin`](super::smooth_skin):
  /// `1.0` is the standard and `2.0` (the default) is twice as wide. Below `0.0` counts as `0.0`, the narrowest.
  pub fn with_feather(mut self, p_feather: impl IntoNumber) -> Self {
    self.feather = p_feather.into::<f32>().max(0.0);
    self
  }
}

impl Effect for TanSkin {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  /// The mask comes from the size of the whole photo, so the effect runs over all of it and is then limited to an area
  /// and mask, instead of on a crop that would give it different values.
  fn positional(&self) -> bool {
    true
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let (red, green, blue, alpha) = self.color.rgba();
    let tint = [red as f32, green as f32, blue as f32];
    let strength = alpha as f32 / 255.0;
    let mask = skin_mask(p_image, self.feather);

    let mut pixels = p_image.to_rgba_vec();
    pixels.par_chunks_exact_mut(4).zip(mask.par_iter()).for_each(|(pixel, skin)| {
      let weight = skin * strength;
      for channel in 0..3 {
        let original = pixel[channel] as f32;
        let tanned = (original * tint[channel] / 255.0).round();
        pixel[channel] = (original + (tanned - original) * weight).round().clamp(0.0, 255.0) as u8;
      }
    });
    p_image.set_rgba(pixels);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for TanSkin {
  /// The same steps as `cpu_processor`, as shader passes: the six passes of the skin mask from [`skin_mask_passes`],
  /// then `tan_blend.wgsl`, which multiplies by the tan color through the mask.
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let (red, green, blue, alpha) = self.color.rgba();
    let mut passes = skin_mask_passes(p_width, p_height, self.feather);
    passes.push(
      GpuPass::new(include_str!("./tan_blend.wgsl"), floats(&[red as f32, green as f32, blue as f32, alpha as f32]))
        .with_base(),
    );
    passes
  }
}

/// Tans skin and leaves the features on it sharp.
/// - `p_color`: The tan. The skin is multiplied by it, so a color like a warm brown darkens and warms the skin, and white
///   changes nothing. Its alpha is how much of the tan shows: opaque is all of it, transparent is none.
pub fn tan_skin(p_color: Color) -> TanSkin {
  TanSkin {
    color: p_color,
    feather: 10.0,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Channels, Image};
  use mask::Mask;
  use options::ApplyOptions;

  const SKIN: (u8, u8, u8) = (230, 190, 150);

  fn flat(p_width: u32, p_height: u32, p_color: (u8, u8, u8)) -> Image {
    let mut img = Image::new(p_width, p_height);
    for y in 0..p_height {
      for x in 0..p_width {
        img.set_pixel(x, y, (p_color.0, p_color.1, p_color.2, 255));
      }
    }
    img
  }

  #[test]
  fn skin_is_tanned_and_other_colors_are_left_alone() {
    // Skin on the left, blue on the right.
    let mut img = flat(48, 48, SKIN);
    for y in 0..48u32 {
      for x in 24..48u32 {
        img.set_pixel(x, y, (60, 100, 200, 255));
      }
    }
    tan_skin(Color::from_rgb(200, 140, 90)).apply(&mut img);
    let skin = img.get_pixel(8, 24).unwrap();
    assert!(skin.0 < SKIN.0 && skin.1 < SKIN.1 && skin.2 < SKIN.2, "skin gets darker: {skin:?}");
    assert!(skin.2 as f32 / skin.0 as f32 <= SKIN.2 as f32 / SKIN.0 as f32, "and warmer: {skin:?}");
    assert_eq!(img.get_pixel(40, 24).unwrap(), (60, 100, 200, 255), "blue is untouched");
  }

  #[test]
  fn a_white_tan_changes_nothing() {
    let mut img = flat(32, 32, SKIN);
    let original = img.to_rgba_vec();
    tan_skin(Color::from_rgb(255, 255, 255)).apply(&mut img);
    assert_eq!(img.to_rgba_vec(), original);
  }

  #[test]
  fn a_given_mask_limits_the_tan_too() {
    let mut img = flat(40, 40, SKIN);
    let original = img.to_rgba_vec();
    // A mask that is black everywhere: nothing may change.
    let black: Vec<u8> = (0..40 * 40).flat_map(|_| [0u8, 0, 0, 255]).collect();
    let mask = Mask::from_image(Image::new_from_pixels(40, 40, black, Channels::RGBA));
    tan_skin(Color::from_rgb(200, 140, 90)).with_options(ApplyOptions::new().with_mask(mask)).apply(&mut img);
    assert_eq!(img.to_rgba_vec(), original);
  }
}
