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

pub struct Sharpen {
  options: Options,
}
impl Apply for Sharpen {
  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }
  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    let options = self.options.clone();
    apply_filter!(apply_sharpen, image, options, 1);
  }
}
pub fn sharpen() -> Sharpen {
  Sharpen { options: None }
}
