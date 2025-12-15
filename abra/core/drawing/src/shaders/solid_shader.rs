use crate::Shader;
use abra_core::Color;

/// A shader that returns a single constant color for any sample location.
///
/// This is the simplest shader that always returns the same RGBA value
/// regardless of the input coordinates.
pub(crate) struct SolidShader {
  color: (u8, u8, u8, u8),
}

impl SolidShader {
  /// Creates a new `SolidShader` with the specified color.
  ///
  /// Parameters
  /// - `p_color`: a `Color` representing the constant output color.
  ///
  /// Example
  /// ```ignore
  /// let shader = SolidShader::new(Color::from_rgba(255, 0, 0, 128));
  /// ```
  pub fn new(p_color: Color) -> Self {
    SolidShader { color: p_color.rgba() }
  }
}

impl Shader for SolidShader {
  fn shade(&self, _p_x: f32, _p_y: f32) -> (u8, u8, u8, u8) {
    self.color
  }
}
