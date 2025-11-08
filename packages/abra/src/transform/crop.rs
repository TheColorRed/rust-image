use std::time::Instant;

use crate::{image::Image, utils::debug::DebugInfo};

/// Trait for cropping functionality.
pub trait Crop {
  /// Crop the image to the given dimensions.
  fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> &mut Self;
}

/// Crop the image to the given dimensions.
pub fn crop(image: &mut Image, x: u32, y: u32, width: u32, height: u32) {
  let duration = Instant::now();
  let mut new_pixels = vec![0u8; (width * height * 4) as usize];
  let old_pixels = image.rgba();
  let (old_width, old_height): (u32, u32) = image.dimensions();

  for i in 0..(width * height) {
    let new_x = i % width;
    let new_y = i / width;
    let old_x = new_x + x;
    let old_y = new_y + y;
    let old_index = (old_y * old_width + old_x) as usize;
    let new_index = (i * 4) as usize;
    new_pixels[new_index..new_index + 4].copy_from_slice(&old_pixels[old_index * 4..old_index * 4 + 4]);
  }

  image.set_new_pixels(new_pixels, width, height);
  DebugInfo::Crop(old_width, old_height, width, height, duration.elapsed()).log();
}
