use crate::{image::Image, utils::kernel::apply_kernel};

/// Smooths the image using a 3x3 box blur kernel
pub fn smooth(image: &mut Image) {
  let kernel = [0.0; 9].iter().map(|_| 1.0 / 9.0).collect::<Vec<f32>>();
  apply_kernel(image, kernel.as_slice());
}
