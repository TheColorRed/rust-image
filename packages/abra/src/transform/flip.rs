use std::time::Instant;

use crate::{image::Image, utils::debug::DebugTransform};
use rayon::prelude::*;

/// Flip the image along the horizontal axis.
/// * `image` - The image to flip.
pub fn horizontal(image: &mut Image) {
  let duration = Instant::now();
  let (width, height) = image.dimensions::<u32>();
  let mut new_pixels = vec![0; (width * height * 4) as usize];
  let old_pixels = image.rgba();

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % width;
    let y = i as u32 / width;
    let old_x = width - x - 1;
    let old_y = y;
    let old_index = (old_y * width + old_x) as usize;
    chunk.copy_from_slice(&old_pixels[old_index * 4..old_index * 4 + 4]);
  });

  image.set_rgba(new_pixels);
  DebugTransform::Flip("Horizontal".into(), width, height, duration.elapsed()).log();
}

/// Flip the image along the vertical axis.
/// * `image` - The image to flip.
pub fn vertical(image: &mut Image) {
  let duration = Instant::now();
  let (width, height) = image.dimensions::<u32>();
  let mut new_pixels = vec![0; (width * height * 4) as usize];
  let old_pixels = image.rgba();

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % width;
    let y = i as u32 / width;
    let old_x = x;
    let old_y = height - y - 1;
    let old_index = (old_y * width + old_x) as usize;
    chunk.copy_from_slice(&old_pixels[old_index * 4..old_index * 4 + 4]);
  });

  image.set_rgba(new_pixels);
  DebugTransform::Flip("Vertical".into(), width, height, duration.elapsed()).log();
}
