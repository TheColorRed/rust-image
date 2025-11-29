use crate::Shader;
use abra_core::{Gradient, Path};

/// Linear gradient shader that maps a parameter t along a `Path` to a color.
///
/// The path is used to compute a 0..1 parameter `t` for each sample via
/// `Path::closest_time`, then the gradient returns the interpolated color
/// for `t`.
pub(crate) struct LinearGradientShader {
  path: Path,
  gradient: Gradient,
}

impl LinearGradientShader {
  /// Creates a `LinearGradientShader` from a `Path` and a `Gradient`.
  ///
  /// Parameters
  /// - `p_path`: path used to compute sampling parameter `t` for the gradient
  /// - `p_gradient`: gradient providing the color ramp used to compute RGBA by `t`
  ///
  /// Example
  /// ```ignore
  /// let shader = LinearGradientShader::new(path, gradient.clone());
  /// ```
  pub fn new(p_path: Path, p_gradient: Gradient) -> Self {
    LinearGradientShader {
      path: p_path,
      gradient: p_gradient,
    }
  }
}

impl Shader for LinearGradientShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let t = self.path.closest_time(p_x, p_y);
    self.gradient.get_color(t)
  }
}
