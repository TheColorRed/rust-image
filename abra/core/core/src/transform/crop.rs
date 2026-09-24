use std::time::Instant;

use crate::{Image, IntoNumber};
use primitives::Image as PrimitiveImage;

/// Trait for cropping functionality.
pub trait Crop {
  /// Crop the image to the given dimensions.
  fn crop(&mut self, p_x: u32, p_y: u32, p_width: u32, p_height: u32);
}

pub struct CropImage {
  x: u32,
  y: u32,
  width: u32,
  height: u32,
}

impl CropImage {
  pub fn apply(&self, p_image: &mut Image) {
    let _duration = Instant::now();
    let x = self.x;
    let y = self.y;
    let width = self.width;
    let height = self.height;

    let mut new_pixels = vec![0u8; (width * height * 4) as usize];
    let old_pixels = p_image.rgba();
    let (old_width, _old_height): (u32, u32) = p_image.dimensions();

    for i in 0..(width * height) {
      let new_x = i % width;
      let new_y = i / width;
      let old_x = new_x + x;
      let old_y = new_y + y;
      let old_index = (old_y * old_width + old_x) as usize;
      let new_index = (i * 4) as usize;
      new_pixels[new_index..new_index + 4].copy_from_slice(&old_pixels[old_index * 4..old_index * 4 + 4]);
    }

    p_image.set_new_pixels(&new_pixels, width, height);
  }
}

/// Crop the image to the given dimensions.
pub fn crop(
  p_x: impl IntoNumber, p_y: impl IntoNumber, p_width: impl IntoNumber, p_height: impl IntoNumber,
) -> CropImage {
  CropImage {
    x: p_x.into::<u32>(),
    y: p_y.into::<u32>(),
    width: p_width.into::<u32>(),
    height: p_height.into::<u32>(),
  }
}

pub fn cropped(p_image: &Image, p_x: u32, p_y: u32, p_width: u32, p_height: u32) -> Image {
  let mut new_image = p_image.clone();
  crop(p_x, p_y, p_width, p_height).apply(&mut new_image);
  new_image
}

impl Crop for PrimitiveImage {
  fn crop(&mut self, p_x: u32, p_y: u32, p_width: u32, p_height: u32) {
    crate::transform::crop(p_x, p_y, p_width, p_height).apply(self);
  }
}
