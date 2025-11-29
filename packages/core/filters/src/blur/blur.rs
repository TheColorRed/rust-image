use options::Options;

use crate::{apply_filter, kernel::apply_kernel};
use abra_core::Image;

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
pub fn blur(p_image: &mut Image, p_apply_options: impl Into<Options>) {
  apply_filter!(apply_blur, p_image, p_apply_options, 1);
}
