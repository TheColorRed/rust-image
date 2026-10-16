use crate::common::*;
use abra_core::image::gpu::{GpuPass, GpuProcessor};

use crate::kernel::apply_kernel;

/// Blurs an image using the blur algorithm.
fn apply_blur(p_image: &mut Image) {
  #[rustfmt::skip]
  let kernel = vec![
    0.0625, 0.125, 0.0625,
    0.125, 0.25, 0.125,
    0.0625, 0.125, 0.0625
  ];
  apply_kernel(p_image, &kernel);
}

/// Applies a blur to to an image.
/// - `p_image`: The image to be blurred.
/// - `p_options`: Additional options for applying the blur.
#[derive(Default, Clone)]
pub struct Blur {
  options: Options,
}

impl GpuProcessor for Blur {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![GpuPass::new(include_str!("./blur.wgsl"), Vec::new()).with_pixel_exact()]
  }
}

impl Effect for Blur {
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
    apply_blur(p_image);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

pub fn blur() -> Blur {
  Blur::default()
}
