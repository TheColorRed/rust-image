use crate::Shader;

/// Brush shader that wraps an inner shader and applies radial alpha falloff.
///
/// This shader is useful for painting brushes where the color is provided
/// by an inner shader (solid, image, gradient) and the alpha (opacity)
/// should be modified based on distance from a center point and the
/// brush hardness parameter.
///
/// Example
/// ```ignore
/// let inner = Box::new(SolidShader::new(Color::from_rgba(255,128,64,192)));
/// let brush = BrushShader::new(inner, 100.0, 100.0, 50.0, 0.5);
/// ```
/// Wraps another shader and applies smooth alpha falloff based on distance from center.
pub(crate) struct BrushShader {
  inner: Box<dyn Shader + Send + Sync>,
  center_x: f32,
  center_y: f32,
  max_distance: f32,
  hardness: f32,
}

impl BrushShader {
  /// Creates a new `BrushShader` that wraps an inner shader and applies radial alpha falloff.
  ///
  /// Parameters
  /// - `p_inner`: boxed inner shader to provide the base RGBA value
  /// - `p_center_x`, `p_center_y`: brush center in device space
  /// - `p_max_distance`: radius at which alpha becomes zero
  /// - `p_hardness`: 0.0 (soft) .. 1.0 (hard) controlling falloff curve
  ///
  /// Example
  /// ```ignore
  /// let brush = BrushShader::new(inner_shader, 10.0, 10.0, 8.0, 0.25);
  /// ```
  pub fn new(
    p_inner: Box<dyn Shader + Send + Sync>, p_center_x: f32, p_center_y: f32, p_max_distance: f32, p_hardness: f32,
  ) -> Self {
    BrushShader {
      inner: p_inner,
      center_x: p_center_x,
      center_y: p_center_y,
      max_distance: p_max_distance,
      hardness: p_hardness.clamp(0.0, 1.0),
    }
  }

  /// Computes alpha falloff based on distance from center and hardness.
  ///
  /// This is the same falloff function used in `BrushCoverageMask` and
  /// returns a factor in `[0.0, 1.0]` that is multiplied into the
  /// alpha component of the underlying `inner` shader.
  fn compute_alpha_falloff(&self, p_x: f32, p_y: f32) -> f32 {
    let dx = p_x - self.center_x;
    let dy = p_y - self.center_y;
    let distance = (dx * dx + dy * dy).sqrt();

    if distance >= self.max_distance {
      return 0.0;
    }

    let normalized_dist = distance / self.max_distance;

    let falloff = if self.hardness < 0.5 {
      let t = self.hardness * 2.0;
      let quadratic = 1.0 - (normalized_dist * normalized_dist);
      let linear = 1.0 - normalized_dist;
      quadratic * (1.0 - t) + linear * t
    } else {
      let t = (self.hardness - 0.5) * 2.0;
      let linear = 1.0 - normalized_dist;
      let hard_edge = if normalized_dist < 0.5 { 1.0 } else { 0.0 };
      linear * (1.0 - t) + hard_edge * t
    };

    falloff.max(0.0)
  }
}

impl Shader for BrushShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let (r, g, b, mut a) = self.inner.shade(p_x, p_y);

    // Apply alpha falloff based on hardness
    let falloff = self.compute_alpha_falloff(p_x, p_y);
    a = ((a as f32) * falloff) as u8;

    (r, g, b, a)
  }
}
