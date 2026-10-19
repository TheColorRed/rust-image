use abra_core::{
  IntoNumber,
  image::{
    Image,
    gpu::{GpuPass, GpuProcessor},
  },
};
use options::{Effect, Options};

use rayon::prelude::*;

fn apply_exposure(
  p_image: &mut Image, p_exposure: impl IntoNumber, p_offset: impl IntoNumber, p_gamma_correction: impl IntoNumber,
) {
  let (width, height) = p_image.dimensions::<i32>();
  let src = p_image.rgba();
  let mut out = vec![0u8; (width * height * 4) as usize];

  // guard gamma correction
  let p_exposure = p_exposure.into::<f32>();
  let p_offset = p_offset.into::<f32>();
  let p_gamma_correction = p_gamma_correction.into::<f32>();

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

  p_image.set_rgba(out);
}

#[derive(Clone)]
/// An exposure adjustment. Create one with [`exposure`].
pub struct Exposure {
  exposure: f64,
  offset: f64,
  gamma_correction: f64,
  options: Options,
}

impl Effect for Exposure {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    1
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let (exposure, offset, gamma_correction) = self.params();
    apply_exposure(p_image, exposure, offset, gamma_correction);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for Exposure {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    let (exposure, offset, gamma_correction) = self.params();
    let uniforms = [(1.4f32).powf(exposure), offset, 1.0 / gamma_correction, 0.0];
    let bytes: Vec<u8> = uniforms.iter().flat_map(|value| value.to_le_bytes()).collect();
    vec![GpuPass::new(include_str!("./exposure.wgsl"), bytes)]
  }
}

impl Exposure {
  /// The exposure, offset and gamma correction clamped to their valid ranges.
  fn params(&self) -> (f32, f32, f32) {
    (
      (self.exposure as f32).clamp(-20.0, 20.0),
      (self.offset as f32).clamp(-0.5, 0.5),
      (self.gamma_correction as f32).clamp(0.01, 9.99),
    )
  }

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

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;
  use gpu::{GpuContext, LiveRenderer};

  #[test]
  fn gpu_matches_cpu_within_one_byte_for_exposure_offset_and_gamma() -> anyhow::Result<()> {
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let pixels: Vec<u8> =
      (0..65536u32).flat_map(|i| [i as u8, (i >> 8) as u8, (i * 37 % 256) as u8, (i * 13 % 256) as u8]).collect();
    for (exposure, offset, gamma) in [(0.0, 0.0, 1.0), (0.5, 0.0, 1.0), (-1.0, -0.2, 0.7), (2.0, 0.3, 1.8)] {
      let effect = exposure_with(exposure, offset, gamma);
      let actual = renderer.process(&[effect.passes(256, 256)], 256, 256, &pixels)?;
      let mut expected = Image::new_from_pixels(256, 256, pixels.clone(), Channels::RGBA);
      effect.apply_on_cpu(&mut expected);
      for (index, (&actual, &expected)) in actual.iter().zip(expected.rgba().iter()).enumerate() {
        let tolerance = if index % 4 == 3 { 0 } else { 1 };
        assert!(
          (actual as i16 - expected as i16).abs() <= tolerance,
          "byte {index}, E={exposure}, O={offset}, G={gamma}: {actual} != {expected}"
        );
      }
    }
    Ok(())
  }

  fn exposure_with(stops: f64, offset_amount: f64, gamma_amount: f64) -> Exposure {
    exposure(stops).with_offset(offset_amount).with_gamma(gamma_amount)
  }
}
