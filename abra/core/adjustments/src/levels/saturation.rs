use abra_core::{
  Image,
  image::gpu::{GpuEffect, GpuOp, GpuPass, GpuProcessor},
};

use options::{Effect, Options};

/// The saturation factor, from `0.0` (gray) to `2.0` (double), as a whole number of 256ths so the CPU and the shader
/// can work in whole numbers and give exactly the same pixels.
/// - `p_amount` - The amount in the range [-100, 100] where 0 means no change.
pub fn factor_in_256ths(p_amount: i32) -> i32 {
  ((p_amount.clamp(-100, 100) + 100) * 256 + 50) / 100
}

/// A pixel's gray, from the 0.299, 0.587 and 0.114 luma weights in 256ths. `saturation.wgsl` does the same.
pub(crate) fn gray(p_r: u8, p_g: u8, p_b: u8) -> i32 {
  (77 * p_r as i32 + 150 * p_g as i32 + 29 * p_b as i32 + 128) >> 8
}

/// One channel moved away from (or toward) `p_gray` by `p_factor` in 256ths, rounded and clamped.
pub(crate) fn saturate_channel(p_channel: u8, p_gray: i32, p_factor: i32) -> u8 {
  (p_gray + (((p_channel as i32 - p_gray) * p_factor + 128) >> 8)).clamp(0, 255) as u8
}

/// Adjust the saturation of an image where 0.0 is grayscale and 100.0 is maximum saturation.
///
/// This is whole-number math, and `saturation.wgsl` does exactly the same steps, so the GPU gives the same pixels:
/// gray is `(77 r + 150 g + 29 b + 128) >> 8` (the 0.299, 0.587 and 0.114 luma weights in 256ths), and each channel
/// moves from gray by the factor in 256ths, rounded.
/// - `p_image` - The image to adjust.
/// - `p_value` - The value to adjust the saturation by. The range is [-100, 100] where 0 means no change.
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
    Some(self)
  }
}

impl GpuProcessor for Saturation {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let op = GpuOp::new(include_str!("./saturation.wgsl"), (factor_in_256ths(self.amount)).to_le_bytes());
    op.passes(p_width, p_height)
  }
}

pub fn saturation(p_amount: i32) -> Saturation {
  Saturation {
    amount: p_amount,
    options: None,
  }
}
