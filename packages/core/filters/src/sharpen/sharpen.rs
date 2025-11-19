use crate::kernel::apply_kernel;
use core::Image;

/// Sharpen an image
pub fn sharpen(image: &mut Image) {
  #[rustfmt::skip]
  let kernel = vec![
    0.0, -0.25, 0.0,
    -0.25, 2.0, -0.25,
    0.0, -0.25, 0.0
  ];
  apply_kernel(image, kernel.as_slice());
}
