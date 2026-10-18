use crate::common::*;
use abra_core::image::gpu::{GpuPass, GpuProcessor};

use crate::kernel::apply_kernel;

/// Sharpen an image. Each cardinal neighbour is subtracted `p_amount` times and the centre makes up the difference, so the
/// weights sum to one and the brightness stays the same.
fn apply_sharpen(p_image: &mut Image, p_amount: f32) {
  #[rustfmt::skip]
  let kernel = vec![
    0.0, -p_amount, 0.0,
    -p_amount, 1.0 + 4.0 * p_amount, -p_amount,
    0.0, -p_amount, 0.0
  ];
  apply_kernel(p_image, kernel.as_slice());
}

#[derive(Clone)]
pub struct Sharpen {
  amount: f32,
  options: Options,
}

impl GpuProcessor for Sharpen {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![GpuPass::new(include_str!("./sharpen.wgsl"), [self.amount, 0.0, 0.0, 0.0].into_iter().flat_map(f32::to_le_bytes).collect::<Vec<u8>>()).with_pixel_exact()]
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
    apply_sharpen(p_image, self.amount);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }

  /// A kernel of one pixel on each side: at a smaller picture it is the same kernel, as wide as a pixel of that picture.
  fn for_level(&self, _p_level: u32) -> Option<std::sync::Arc<dyn abra_core::image::gpu::CpuProcessor>> {
    Some(std::sync::Arc::new(Sharpen { ..self.clone() }))
  }
}
impl Sharpen {
  /// Sets how strongly edges are sharpened. `0.0` leaves the image as it is. Defaults to `0.25`.
  pub fn with_amount(mut self, p_amount: f32) -> Self {
    self.amount = p_amount;
    self
  }
}

/// Sharpens the image. Use `with_amount` to change how strongly.
pub fn sharpen() -> Sharpen {
  Sharpen { amount: 0.25, options: None }
}
