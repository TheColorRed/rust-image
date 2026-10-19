use crate::Shader;
use abra_core::{Image, ImageRepeat};
use std::sync::Arc;

/// Where a sample at (`p_x`, `p_y`) of an image `p_width` x `p_height` is read from, given how the image repeats. A direction that
/// repeats wraps around; one that does not gives `None` past its edge.
pub(crate) fn repeated_sample(p_x: i32, p_y: i32, p_width: i32, p_height: i32, p_repeat: ImageRepeat) -> Option<(i32, i32)> {
  let x = if p_repeat.repeats_x() { p_x.rem_euclid(p_width) } else { p_x };
  let y = if p_repeat.repeats_y() { p_y.rem_euclid(p_height) } else { p_y };
  if x < 0 || y < 0 || x >= p_width || y >= p_height {
    return None;
  }
  Some((x, y))
}

/// A shader that samples RGBA from a source `Image` at integer coordinates.
///
/// Coordinates are shifted by the provided offset and floored to the
/// nearest pixel coordinate before indexing into the image buffer. If the
/// sample lies outside the image bounds the shader returns *(0,0,0,0)*,
/// unless the image repeats in that direction.
pub(crate) struct ImageShader {
  image: Arc<Image>,
  width: i32,
  height: i32,
  offset_x: f32,
  offset_y: f32,
  repeat: ImageRepeat,
}

impl ImageShader {
  /// Creates a new `ImageShader` that will sample from `p_image` with the provided offset.
  ///
  /// Parameters
  /// - `p_image`: the source image to sample (an owned clone is kept internally)
  /// - `p_offset_x`, `p_offset_y`: offset applied to sample positions (for simple translations)
  /// - `p_repeat`: the directions the image is tiled in
  ///
  /// Example
  /// ```ignore
  /// let shader = ImageShader::new(image.clone(), 5.0, 3.0, ImageRepeat::NoRepeat);
  /// ```
  pub fn new(p_image: Arc<Image>, p_offset_x: f32, p_offset_y: f32, p_repeat: ImageRepeat) -> Self {
    let (width, height) = p_image.dimensions::<i32>();
    ImageShader {
      image: p_image,
      width,
      height,
      offset_x: p_offset_x,
      offset_y: p_offset_y,
      repeat: p_repeat,
    }
  }
}

impl Shader for ImageShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let sample_x = (p_x - self.offset_x).floor() as i32;
    let sample_y = (p_y - self.offset_y).floor() as i32;

    let Some((x, y)) = repeated_sample(sample_x, sample_y, self.width, self.height, self.repeat) else {
      return (0, 0, 0, 0);
    };

    let idx = ((y * self.width + x) as usize) * 4;
    let pixels = self.image.rgba();
    (pixels[idx], pixels[idx + 1], pixels[idx + 2], pixels[idx + 3])
  }
}

