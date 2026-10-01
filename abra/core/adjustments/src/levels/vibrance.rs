use abra_core::{
  Image,
  image::gpu::{GpuPass, GpuProcessor},
};
use options::{Effect, Options};

use rayon::prelude::*;

use crate::levels::{
  factor_in_256ths,
  saturation::{gray, saturate_channel},
};

/// The vibrance step, then, when `p_saturation_factor` is given (in 256ths), the saturation step on each pixel in the
/// same loop. The vibrance result is rounded to 8 bits before it is saturated, as the GPU does between its two passes.
///
/// Each color moves away from (or toward) the pixel's average by an amount that depends on how far its strongest
/// channel is from that average. `vibrance.wgsl` does the same math.
fn apply_vibrance(p_image: &mut Image, p_vibrance: f32, p_saturation_factor: Option<i32>) {
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
    let vibrant = |channel: f32| (((channel - avg) * vibrance_factor + avg).clamp(0.0, 1.0) * 255.0).round() as u8;
    dst_px[..3].copy_from_slice(&[vibrant(r), vibrant(g), vibrant(b)]);
    dst_px[3] = a;

    // apply saturation
    if let Some(factor) = p_saturation_factor {
      let gray = gray(dst_px[0], dst_px[1], dst_px[2]);
      for channel in &mut dst_px[..3] {
        *channel = saturate_channel(*channel, gray, factor);
      }
    }
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

impl Effect for Vibrance {
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
    // The saturation uses the Saturation effect's own math, so the GPU's saturation pass gives the same pixels.
    let saturation = (self.saturation_amount() != 0).then(|| factor_in_256ths(self.saturation_amount()));
    apply_vibrance(p_image, self.vibrance_amount(), saturation);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for Vibrance {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    let mut passes = vec![GpuPass::new(
      include_str!("./vibrance.wgsl"),
      self.vibrance_amount().to_le_bytes().to_vec(),
    )];
    if self.saturation_amount() != 0 {
      // `saturation.wgsl` takes the factor in 256ths;
      // a factor of 0 would be fully gray, so no saturation adds no pass.
      let factor = factor_in_256ths(self.saturation_amount());
      passes.push(GpuPass::new(include_str!("./saturation.wgsl"), factor.to_le_bytes().to_vec()));
    }
    passes
  }
}

impl Vibrance {
  /// The vibrance, clamped to `-100..=100`.
  fn vibrance_amount(&self) -> f32 {
    (self.vibrance as f32).clamp(-100.0, 100.0)
  }

  /// The saturation, rounded to a whole number and clamped to `-100..=100`.
  fn saturation_amount(&self) -> i32 {
    self.saturation.round().clamp(-100.0, 100.0) as i32
  }

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
