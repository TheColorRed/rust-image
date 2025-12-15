use crate::common::*;

use crate::kernel::apply_kernel;

/// Sharpen an image
fn apply_sharpen(image: &mut Image) {
  #[rustfmt::skip]
  let kernel = vec![
    0.0, -0.25, 0.0,
    -0.25, 2.0, -0.25,
    0.0, -0.25, 0.0
  ];
  apply_kernel(image, kernel.as_slice());
}

pub fn sharpen<'a>(p_image: impl Into<ImageRef<'a>>, p_apply_options: impl Into<Options>) {
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  apply_filter!(apply_sharpen, image, p_apply_options, 1);
}
