use abra_core::{
  Image,
  image::gpu::{GpuEffect, GpuOp, GpuPass, GpuProcessor},
};

use options::{Effect, Options};

// `255 * factor + 128` must remain in a signed 32-bit shader calculation.
const MAX_GPU_FACTOR: i64 = 8_421_503;

/// The saturation factor, as a whole number of 256ths.
///
/// `0` gives gray and `100` gives a factor of `2.0` (double the distance from gray). The supplied amount is not
/// limited here: applications own their control ranges. The wider integer keeps every `i32` amount representable in
/// the CPU calculation.
pub fn factor_in_256ths(p_amount: i32) -> i64 {
  let numerator = (i64::from(p_amount) + 100) * 256;
  if numerator >= 0 { (numerator + 50) / 100 } else { (numerator - 50) / 100 }
}

/// A pixel's gray, from the 0.299, 0.587 and 0.114 luma weights in 256ths. `saturation.wgsl` does the same.
pub(crate) fn gray(p_r: u8, p_g: u8, p_b: u8) -> i32 {
  (77 * p_r as i32 + 150 * p_g as i32 + 29 * p_b as i32 + 128) >> 8
}

/// One channel moved away from (or toward) `p_gray` by `p_factor` in 256ths, rounded and clamped.
pub(crate) fn saturate_channel(p_channel: u8, p_gray: i32, p_factor: impl Into<i64>) -> u8 {
  let factor = p_factor.into();
  (i64::from(p_gray) + (((i64::from(p_channel) - i64::from(p_gray)) * factor + 128) >> 8)).clamp(0, 255) as u8
}

/// Adjust the saturation of an image by moving each RGB channel toward or away from its gray value.
///
/// This is whole-number math, and `saturation.wgsl` does exactly the same steps when its signed 32-bit factor can
/// represent the adjustment, so the GPU and CPU give the same pixels:
/// gray is `(77 r + 150 g + 29 b + 128) >> 8` (the 0.299, 0.587 and 0.114 luma weights in 256ths), and each channel
/// moves from gray by the factor in 256ths, rounded.
/// - `p_image` - The image to adjust.
/// - `p_value` - The adjustment amount. `-100` gives gray, `0` makes no change, and `100` doubles the distance from
///   gray. Values outside that range are preserved. Results outside the byte-channel gamut are clipped to `0..=255`.
fn apply_saturation(p_image: &mut Image, p_value: i32) {
  let factor = factor_in_256ths(p_value);

  p_image.mut_pixels(|mut pixel| {
    let gray = gray(pixel[0], pixel[1], pixel[2]);
    for channel in 0..3 {
      pixel[channel] = saturate_channel(pixel[channel], gray, factor);
    }
  });
}

#[derive(Clone)]
pub struct Saturation {
  amount: i32,
  options: Options,
}

impl Effect for Saturation {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    0
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_saturation(p_image, self.amount);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    // The shader uses signed 32-bit multiplication. Larger requested amounts remain valid and use the wider CPU path.
    let factor = factor_in_256ths(self.amount);
    (-MAX_GPU_FACTOR..=MAX_GPU_FACTOR).contains(&factor).then_some(self)
  }
}

impl GpuProcessor for Saturation {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let factor = i32::try_from(factor_in_256ths(self.amount)).expect("a GPU saturation pass requires an i32 factor");
    let op = GpuOp::new(include_str!("./saturation.wgsl"), factor.to_le_bytes());
    op.passes(p_width, p_height)
  }
}

pub fn saturation(p_amount: i32) -> Saturation {
  Saturation {
    amount: p_amount,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn factor_preserves_amounts_outside_an_app_slider_range() {
    assert_eq!(factor_in_256ths(-100), 0);
    assert_eq!(factor_in_256ths(100), 512);
    assert_eq!(factor_in_256ths(101), 515);
    assert_eq!(factor_in_256ths(-101), -3);
  }

  #[test]
  fn channel_output_is_still_clipped_to_a_byte() {
    assert_eq!(saturate_channel(255, 128, 512), 255);
    assert_eq!(saturate_channel(0, 128, 512), 0);
  }
}
