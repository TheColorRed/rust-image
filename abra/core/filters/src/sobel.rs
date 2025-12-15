use crate::kernel::apply_kernel;
use abra_core::{Image, image::image_ext::ImageRef};

/// Applies the Sobel filter to the image in the horizontal direction.
pub fn sobel_horizontal<'a>(image: impl Into<ImageRef<'a>>) {
  let mut image_ref: ImageRef = image.into();
  let image = &mut image_ref as &mut Image;
  #[rustfmt::skip]
  let kernel_x = vec![
    1.0, 2.0, 1.0,
    0.0, 0.0, 0.0,
    -1.0, -2.0, -1.0
  ];
  apply_kernel(image, &kernel_x);
}

/// Applies the Sobel filter to the image in the vertical direction.
pub fn sobel_vertical<'a>(image: impl Into<ImageRef<'a>>) {
  let mut image_ref: ImageRef = image.into();
  let image = &mut image_ref as &mut Image;
  #[rustfmt::skip]
  let kernel_y = vec![
    1.0, 0.0, -1.0,
    2.0, 0.0, -2.0,
    1.0, 0.0, -1.0
  ];
  apply_kernel(image, &kernel_y);
}
