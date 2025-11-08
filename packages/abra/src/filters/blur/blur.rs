use crate::{image::Image, utils::kernel::apply_kernel};

/// Blurs an image using the blur algorithm.
pub fn blur(image: &mut Image) {
  #[rustfmt::skip]
  let kernel = vec![
    0.0625, 0.125, 0.0625,
    0.125, 0.25, 0.125,
    0.0625, 0.125, 0.0625
  ];
  apply_kernel(image, &kernel);
}
