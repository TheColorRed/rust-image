use std::time::Instant;

use crate::Image;
use rayon::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlipAxis {
  Horizontal,
  Vertical,
}

/// A flip that has been described but not yet run. Create one with [`flip`], then run it with [`FlipImage::apply`].
pub struct FlipImage {
  pub axis: FlipAxis,
}

impl FlipImage {
  /// Flips the image in place.
  pub fn apply(&self, p_image: &mut Image) {
    let _duration = Instant::now();
    let (width, height) = p_image.dimensions::<u32>();
    let mut new_pixels = vec![0; (width * height * 4) as usize];
    let old_pixels = p_image.rgba();

    new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
      let x = i as u32 % width;
      let y = i as u32 / width;
      let (old_x, old_y) = match self.axis {
        FlipAxis::Horizontal => (width - x - 1, y),
        FlipAxis::Vertical => (x, height - y - 1),
      };
      let old_index = (old_y * width + old_x) as usize;
      chunk.copy_from_slice(&old_pixels[old_index * 4..old_index * 4 + 4]);
    });

    p_image.set_rgba_owned(new_pixels);
    // DebugTransform::Flip("Horizontal".into(), width, height, duration.elapsed()).log();
  }
}

/// Flips an image along the requested axis.
/// # Arguments
/// - `p_axis`: The axis to mirror the image across.
pub fn flip(p_axis: FlipAxis) -> FlipImage {
  FlipImage { axis: p_axis }
}
