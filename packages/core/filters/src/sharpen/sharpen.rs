use options::Options;

use crate::{apply_filter, kernel::apply_kernel};
use abra_core::Image;

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

pub fn sharpen(image: &mut Image, p_apply_options: impl Into<Options>) {
  apply_filter!(apply_sharpen, image, p_apply_options, 1);
}
