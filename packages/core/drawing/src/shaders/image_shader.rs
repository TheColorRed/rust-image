use crate::Shader;
use core::Image;
use std::sync::Arc;

/// A shader that samples RGBA from a source `Image` at integer coordinates.
///
/// Coordinates are shifted by the provided offset and floored to the
/// nearest pixel coordinate before indexing into the image buffer. If the
/// sample lies outside the image bounds the shader returns *(0,0,0,0)*.
pub(crate) struct ImageShader {
  image: Arc<Image>,
  width: i32,
  height: i32,
  offset_x: f32,
  offset_y: f32,
}

impl ImageShader {
  /// Creates a new `ImageShader` that will sample from `p_image` with the provided offset.
  ///
  /// Parameters
  /// - `p_image`: the source image to sample (an owned clone is kept internally)
  /// - `p_offset_x`, `p_offset_y`: offset applied to sample positions (for simple translations)
  ///
  /// Example
  /// ```ignore
  /// let shader = ImageShader::new(image.clone(), 5.0, 3.0);
  /// ```
  pub fn new(p_image: Arc<Image>, p_offset_x: f32, p_offset_y: f32) -> Self {
    let (width, height) = p_image.dimensions::<i32>();
    ImageShader {
      image: p_image,
      width,
      height,
      offset_x: p_offset_x,
      offset_y: p_offset_y,
    }
  }
}

impl Shader for ImageShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let sample_x = (p_x - self.offset_x).floor() as i32;
    let sample_y = (p_y - self.offset_y).floor() as i32;

    if sample_x < 0 || sample_y < 0 || sample_x >= self.width || sample_y >= self.height {
      return (0, 0, 0, 0);
    }

    let idx = ((sample_y * self.width + sample_x) as usize) * 4;
    let pixels = self.image.rgba_slice();
    (pixels[idx], pixels[idx + 1], pixels[idx + 2], pixels[idx + 3])
  }
}
