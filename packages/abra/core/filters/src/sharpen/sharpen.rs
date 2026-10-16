use crate::common::*;
use abra_core::image::gpu::{GpuPass, GpuProcessor};

use crate::kernel::apply_kernel;

/// Sharpen an image
fn apply_sharpen(p_image: &mut Image) {
  #[rustfmt::skip]
  let kernel = vec![
    0.0, -0.25, 0.0,
    -0.25, 2.0, -0.25,
    0.0, -0.25, 0.0
  ];
  apply_kernel(p_image, kernel.as_slice());
}

#[derive(Clone)]
pub struct Sharpen {
  options: Options,
}

impl GpuProcessor for Sharpen {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![GpuPass::new(include_str!("./sharpen.wgsl"), Vec::new()).with_pixel_exact()]
  }
}

impl Effect for Sharpen {
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
    apply_sharpen(p_image);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }

  /// A kernel of one pixel on each side: at a smaller picture it is the same kernel, as wide as a pixel of that picture.
  fn for_level(&self, _p_level: u32) -> Option<std::sync::Arc<dyn abra_core::image::gpu::CpuProcessor>> {
    Some(std::sync::Arc::new(Sharpen { ..self.clone() }))
  }
}
pub fn sharpen() -> Sharpen {
  Sharpen { options: None }
}
