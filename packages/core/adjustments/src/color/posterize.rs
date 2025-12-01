use abra_core::{Image, ImageRef};
use rayon::prelude::*;

/// Posterizes an image to a specified number of levels.
pub fn posterize<'a>(image: impl Into<ImageRef<'a>>, levels: u8) {
  let mut image_ref: ImageRef = image.into();
  let image = &mut image_ref as &mut Image;
  let levels = (levels as f32).clamp(2.0, 255.0);
  let pixels = image.colors().as_slice_mut().expect("Image colors must be contiguous");

  pixels.par_chunks_mut(4).for_each(|pixel| {
    pixel[0] = ((pixel[0] as f32 / 255.0 * (levels - 1.0) as f32).round() / (levels - 1.0) as f32 * 255.0) as u8;
    pixel[1] = ((pixel[1] as f32 / 255.0 * (levels - 1.0) as f32).round() / (levels - 1.0) as f32 * 255.0) as u8;
    pixel[2] = ((pixel[2] as f32 / 255.0 * (levels - 1.0) as f32).round() / (levels - 1.0) as f32 * 255.0) as u8;
  });

  // pixels mutated in place
}
