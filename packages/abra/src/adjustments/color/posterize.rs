use crate::image::Image;
use rayon::prelude::*;

/// Posterizes an image to a specified number of levels.
pub fn posterize(image: &mut Image, levels: u8) {
  let levels = (levels as f32).clamp(2.0, 255.0);
  let mut pixels = image.rgba();

  pixels.par_chunks_mut(4).for_each(|pixel| {
    pixel[0] = ((pixel[0] as f32 / 255.0 * (levels - 1.0) as f32).round() / (levels - 1.0) as f32 * 255.0) as u8;
    pixel[1] = ((pixel[1] as f32 / 255.0 * (levels - 1.0) as f32).round() / (levels - 1.0) as f32 * 255.0) as u8;
    pixel[2] = ((pixel[2] as f32 / 255.0 * (levels - 1.0) as f32).round() / (levels - 1.0) as f32 * 255.0) as u8;
  });

  image.set_rgba(pixels);
}
