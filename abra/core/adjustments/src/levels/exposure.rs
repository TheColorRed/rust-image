use abra_core::{ImageRef, image::Image};
use options::{Apply, Options};

use rayon::prelude::*;

use crate::apply_adjustment;

fn apply_exposure(p_image: &mut Image, p_exposure: f32, p_offset: f32, p_gamma_correction: f32) {
  let (width, height) = p_image.dimensions::<i32>();
  let src = p_image.rgba();
  let mut out = vec![0u8; (width * height * 4) as usize];

  // guard gamma correction
  let gamma_correction = if p_gamma_correction <= 0.0 { 0.01 } else { p_gamma_correction };
  // fixed gamma for sRGB conversion
  let gamma = 2.2;
  let inv_gamma = 1.0 / gamma;
  let exposure_factor = (1.4f32).powf(p_exposure);

  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let i = idx * 4;

    // read sRGB channels
    let r_srgb = src[i] as f32 / 255.0;
    let g_srgb = src[i + 1] as f32 / 255.0;
    let b_srgb = src[i + 2] as f32 / 255.0;
    let a = src[i + 3];

    // convert sRGB -> linear (approximate with pow)
    let r_lin = r_srgb.powf(gamma);
    let g_lin = g_srgb.powf(gamma);
    let b_lin = b_srgb.powf(gamma);

    // apply exposure in linear space, add offset
    let r_lin = (r_lin * exposure_factor + p_offset).max(0.0);
    let g_lin = (g_lin * exposure_factor + p_offset).max(0.0);
    let b_lin = (b_lin * exposure_factor + p_offset).max(0.0);

    // apply gamma correction in linear space
    let r_lin = r_lin.powf(1.0 / gamma_correction);
    let g_lin = g_lin.powf(1.0 / gamma_correction);
    let b_lin = b_lin.powf(1.0 / gamma_correction);

    // convert back to sRGB
    let r_out = r_lin.powf(inv_gamma).clamp(0.0, 1.0);
    let g_out = g_lin.powf(inv_gamma).clamp(0.0, 1.0);
    let b_out = b_lin.powf(inv_gamma).clamp(0.0, 1.0);

    dst_px[0] = (r_out * 255.0).round() as u8;
    dst_px[1] = (g_out * 255.0).round() as u8;
    dst_px[2] = (b_out * 255.0).round() as u8;
    // Preserve alpha as-is (do NOT gamma-correct alpha):
    dst_px[3] = a;
  });

  p_image.set_rgba(&out);
}

/// An exposure adjustment. Create one with [`exposure`].
pub struct Exposure {
  exposure: f64,
  offset: f64,
  gamma_correction: f64,
  options: Options,
}

impl Apply for Exposure {
  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    let exposure = (self.exposure as f32).clamp(-20.0, 20.0);
    let offset = (self.offset as f32).clamp(-0.5, 0.5);
    let gamma_correction = (self.gamma_correction as f32).clamp(0.01, 9.99);
    apply_adjustment!(apply_exposure, image, self.options.as_ref(), 1, exposure, offset, gamma_correction);
  }
}

impl Exposure {
  /// Sets the offset added to each color channel, from `-0.5` to `0.5`. Lightens or darkens the shadows while
  /// barely touching the highlights. Defaults to `0.0`.
  pub fn with_offset(mut self, p_offset: impl Into<f64>) -> Self {
    self.offset = p_offset.into();
    self
  }

  /// Sets the gamma correction, from `0.01` to `9.99`. `1.0` means no correction. Defaults to `1.0`.
  pub fn with_gamma(mut self, p_gamma_correction: impl Into<f64>) -> Self {
    self.gamma_correction = p_gamma_correction.into();
    self
  }
}

/// Applies an exposure adjustment to the image.
/// # Arguments
/// - `p_exposure`: The exposure in stops, from `-20.0` to `20.0`. Positive values brighten, negative values darken.
pub fn exposure(p_exposure: impl Into<f64>) -> Exposure {
  Exposure {
    exposure: p_exposure.into(),
    offset: 0.0,
    gamma_correction: 1.0,
    options: None,
  }
}
