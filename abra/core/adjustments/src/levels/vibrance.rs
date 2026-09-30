use abra_core::{Image, ImageRef};
use options::{Apply, Options};

use rayon::prelude::*;

use crate::apply_adjustment;

fn apply_vibrance(p_image: &mut Image, p_vibrance: f32, p_saturation: f32) {
  let (width, height) = p_image.dimensions::<i32>();
  let src = p_image.rgba();
  let mut out = vec![0u8; (width * height * 4) as usize];

  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let i = idx * 4;
    let r = src[i] as f32 / 255.0;
    let g = src[i + 1] as f32 / 255.0;
    let b = src[i + 2] as f32 / 255.0;
    let a = src[i + 3];

    // compute average
    let avg = (r + g + b) / 3.0;

    // compute vibrance factor
    let max_rgb = r.max(g).max(b);
    let amt = ((max_rgb - avg) * 3.0).clamp(0.0, 1.0);
    let vibrance_factor = 1.0 + (p_vibrance / 100.0) * amt;

    // apply vibrance
    let r_vib = ((r - avg) * vibrance_factor + avg).clamp(0.0, 1.0);
    let g_vib = ((g - avg) * vibrance_factor + avg).clamp(0.0, 1.0);
    let b_vib = ((b - avg) * vibrance_factor + avg).clamp(0.0, 1.0);

    // apply saturation
    let lum_r = 0.2126;
    let lum_g = 0.7152;
    let lum_b = 0.0722;
    let lum = r_vib * lum_r + g_vib * lum_g + b_vib * lum_b;
    let saturation_factor = 1.0 + (p_saturation / 100.0);
    let r_out = (lum + (r_vib - lum) * saturation_factor).clamp(0.0, 1.0);
    let g_out = (lum + (g_vib - lum) * saturation_factor).clamp(0.0, 1.0);
    let b_out = (lum + (b_vib - lum) * saturation_factor).clamp(0.0, 1.0);
    dst_px[0] = (r_out * 255.0).round() as u8;
    dst_px[1] = (g_out * 255.0).round() as u8;
    dst_px[2] = (b_out * 255.0).round() as u8;
    dst_px[3] = a;
  });

  p_image.set_rgba(out);
}

/// A vibrance adjustment. Create one with [`vibrance`].
#[derive(Clone)]
pub struct Vibrance {
  vibrance: f64,
  saturation: f64,
  options: Options,
}

options::cpu_processor!(Vibrance);

impl Apply for Vibrance {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    let vibrance = (self.vibrance as f32).clamp(-100.0, 100.0);
    let saturation = (self.saturation as f32).clamp(-100.0, 100.0);
    apply_adjustment!(apply_vibrance, image, self.options.as_ref(), 1, vibrance, saturation);
  }
}

impl Vibrance {
  /// Also adjusts saturation evenly across all colors, from `-100` to `100`. Defaults to `0`.
  pub fn with_saturation(mut self, p_saturation: impl Into<f64>) -> Self {
    self.saturation = p_saturation.into();
    self
  }
}

/// Adjusts vibrance, which boosts muted colors more than already saturated ones.
/// # Arguments
/// - `p_vibrance`: The vibrance, from `-100` to `100`. Positive values increase vibrance, negative values decrease it.
pub fn vibrance(p_vibrance: impl Into<f64>) -> Vibrance {
  Vibrance {
    vibrance: p_vibrance.into(),
    saturation: 0.0,
    options: None,
  }
}
