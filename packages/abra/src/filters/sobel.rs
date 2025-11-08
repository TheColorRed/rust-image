use crate::{image::Image, utils::kernel::apply_kernel};

/// Applies the Sobel filter to the image in the horizontal direction.
pub fn sobel_horizontal(image: &mut Image) {
  #[rustfmt::skip]
  let kernel_x = vec![
    1.0, 2.0, 1.0,
    0.0, 0.0, 0.0,
    -1.0, -2.0, -1.0
  ];
  apply_kernel(image, &kernel_x);
}

/// Applies the Sobel filter to the image in the vertical direction.
pub fn sobel_vertical(image: &mut Image) {
  #[rustfmt::skip]
  let kernel_y = vec![
    1.0, 0.0, -1.0,
    2.0, 0.0, -2.0,
    1.0, 0.0, -1.0
  ];
  apply_kernel(image, &kernel_y);
}
