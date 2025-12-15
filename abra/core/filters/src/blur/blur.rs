use crate::common::*;

use crate::kernel::apply_kernel;

/// Blurs an image using the blur algorithm.
fn apply_blur(image: &mut Image) {
  #[rustfmt::skip]
  let kernel = vec![
    0.0625, 0.125, 0.0625,
    0.125, 0.25, 0.125,
    0.0625, 0.125, 0.0625
  ];
  apply_kernel(image, &kernel);
}

/// Applies a blur to to an image.
/// - `p_image`: The image to be blurred.
/// - `p_options`: Additional options for applying the blur.
pub fn blur<'a>(p_image: impl Into<ImageRef<'a>>, p_apply_options: impl Into<Options>) {
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  apply_filter!(apply_blur, image, p_apply_options, 1);
}
