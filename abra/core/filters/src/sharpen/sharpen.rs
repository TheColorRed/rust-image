use crate::common::*;

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
}
pub fn sharpen() -> Sharpen {
  Sharpen { options: None }
}
