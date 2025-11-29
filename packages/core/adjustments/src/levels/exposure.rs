use abra_core::image::Image;
use options::Options;

use rayon::prelude::*;

use crate::apply_adjustment;

fn apply_exposure(p_image: &mut Image, p_exposure: f32, p_offset: f32, p_gamma_correction: f32) {
  let (width, height) = p_image.dimensions::<i32>();
  let src = p_image.rgba();
  let mut out = vec![0u8; (width * height * 4) as usize];

  // guard gamma and compute exposure factor
  let gamma = if p_gamma_correction <= 0.0 {
    1.0
  } else {
    p_gamma_correction
  };
  let inv_gamma = 1.0 / gamma;
  let exposure_factor = (1.4f32).powf(p_exposure);

  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let i = idx * 4;

    // read sRGB channels
    let r_srgb = src[i] as f32 / 255.0;
    let g_srgb = src[i + 1] as f32 / 255.0;
    let b_srgb = src[i + 2] as f32 / 255.0;
    let a = src[i + 3];

    // convert sRGB -> linear (approximate with pow; use exact sRGB conversion if needed)
    let r_lin = r_srgb.powf(gamma);
    let g_lin = g_srgb.powf(gamma);
    let b_lin = b_srgb.powf(gamma);

    // apply exposure in linear space, add offset (offset should be in linear units)
    let r_lin = (r_lin * exposure_factor + p_offset).max(0.0);
    let g_lin = (g_lin * exposure_factor + p_offset).max(0.0);
    let b_lin = (b_lin * exposure_factor + p_offset).max(0.0);

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

/// Applies an exposure adjustment to the image.
/// - `p_image`: The image to adjust.
/// - `p_exposure`: The exposure value. Positive values increase exposure, negative values decrease it.
/// - `p_offset`: The offset value to add to each color channel.
/// - `p_gamma_correction`: The gamma correction value to apply. `1.0` means no correction.
pub fn exposure(
  p_image: &mut Image, p_exposure: impl Into<f64>, p_offset: impl Into<f64>, p_gamma_correction: impl Into<f64>,
  p_options: impl Into<Options>,
) {
  let exposure = (p_exposure.into() as f32).clamp(-20.0, 20.0);
  let offset = (p_offset.into() as f32).clamp(-0.5, 0.5);
  let gamma_correction = (p_gamma_correction.into() as f32).clamp(0.01, 9.99);

  apply_adjustment!(apply_exposure, p_image, p_options, 1, exposure, offset, gamma_correction);
}

pub fn exposure_plus_one(p_image: &mut Image, p_options: impl Into<Options>) {
  exposure(p_image, 1.0, 0.0, 1.0, p_options);
}

pub fn exposure_minus_one(p_image: &mut Image, p_options: impl Into<Options>) {
  exposure(p_image, -1.0, 0.0, 1.0, p_options);
}

pub fn exposure_plus_two(p_image: &mut Image, p_options: impl Into<Options>) {
  exposure(p_image, 2.0, 0.0, 1.0, p_options);
}

pub fn exposure_minus_two(p_image: &mut Image, p_options: impl Into<Options>) {
  exposure(p_image, -2.0, 0.0, 1.0, p_options);
}
