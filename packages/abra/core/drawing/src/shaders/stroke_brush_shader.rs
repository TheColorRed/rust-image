use crate::Shader;
use abra_core::{Path, PointF, StrokeOpacity, StrokeWidth};

/// Stroke brush shader: applies hardness-based falloff relative to a path centerline.
///
/// Wraps another shader and modulates its alpha along a path. The shader
/// computes the closest point on the provided `Path` and uses the
/// perpendicular distance to that centerline to reduce alpha according to
/// the given `max_distance` and `hardness` values.
///
/// Example
/// ```ignore
/// let stroke = StrokeBrushShader::new(inner_shader, path.clone(), 6.0, 0.6);
/// ```
pub(crate) struct StrokeBrushShader {
  /// Inner shader providing base RGBA values.
  inner: Box<dyn Shader + Send + Sync>,
  fallback_point: PointF,
  /// How wide the stroke is along the path, `0` at its start and `1` at its end. Half of it is the influence distance from the centerline.
  width: StrokeWidth,
  /// How opaque the stroke is along the path, `0` at its start and `1` at its end.
  opacity: StrokeOpacity,
  /// How far along the path each flattened point is, from `0` at the start to `1` at the end, by length.
  along: Vec<f32>,
  /// Falloff hardness in [0.0, 1.0].
  hardness: f32,
  // Pre-flattened path points for fast closest-point queries during shading.
  flattened: Vec<PointF>,
}

impl StrokeBrushShader {
  /// Creates a new `StrokeBrushShader` that wraps a shader and applies
  /// alpha falloff relative to the provided `Path`.
  ///
  /// Parameters
  /// - `p_inner`: boxed inner shader providing base RGBA values
  /// - `p_path`: path forming the stroke centerline
  /// - `p_width`: how wide the stroke is along the path; half of it is the influence distance from the centerline
  /// - `p_opacity`: how opaque the stroke is along the path
  /// - `p_hardness`: falloff hardness in [0.0, 1.0]
  ///
  /// Example
  /// ```ignore
  /// let s = StrokeBrushShader::new(inner, path.clone(), 6.0, 0.7);
  /// ```
  pub fn new(
    p_inner: Box<dyn Shader + Send + Sync>, p_path: &Path, p_width: StrokeWidth, p_opacity: StrokeOpacity, p_hardness: impl Into<f64>,
  ) -> Self {
    // Pre-flatten the path to a set of points; choose a tolerance that balances accuracy and performance.
    let flattened = p_path.flatten(1.0);
    let mut along = vec![0.0; flattened.len()];
    for at in 1..flattened.len() {
      let (from, to) = (flattened[at - 1], flattened[at]);
      along[at] = along[at - 1] + ((to.x - from.x).powi(2) + (to.y - from.y).powi(2)).sqrt();
    }
    let total = along.last().copied().unwrap_or(0.0);
    if total > 0.0 {
      along.iter_mut().for_each(|length| *length /= total);
    }
    StrokeBrushShader {
      inner: p_inner,
      fallback_point: p_path.start(),
      width: p_width,
      opacity: p_opacity,
      along,
      hardness: p_hardness.into().clamp(0.0, 1.0) as f32,
      flattened,
    }
  }

  /// Computes the falloff (a factor in 0..1) given a perpendicular distance
  /// from the stroke's centerline. This function is used by `shade` to
  /// modulate the alpha channel.
  /// - `p_distance`: perpendicular distance from the stroke centerline.
  /// - `p_max_distance`: how far from the centerline the stroke reaches there: half its width.
  fn compute_alpha_falloff_from_distance(&self, p_distance: f32, p_max_distance: f32) -> f32 {
    if p_distance >= p_max_distance {
      return 0.0;
    }
    let normalized_dist = p_distance / p_max_distance;

    if self.hardness < 0.5 {
      let t = self.hardness * 2.0;
      let quadratic = 1.0 - (normalized_dist * normalized_dist);
      let linear = 1.0 - normalized_dist;
      (quadratic * (1.0 - t) + linear * t).max(0.0)
    } else {
      let t = (self.hardness - 0.5) * 2.0;
      let linear = 1.0 - normalized_dist;
      let hard_edge = if normalized_dist < 0.5 { 1.0 } else { 0.0 };
      (linear * (1.0 - t) + hard_edge * t).max(0.0)
    }
  }
}

impl Shader for StrokeBrushShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let (r, g, b, mut a) = self.inner.shade(p_x, p_y);
    let (distance_sq, along) = self.closest_on_path(p_x, p_y);
    let falloff = self.compute_alpha_falloff_from_distance(distance_sq.sqrt(), self.width.width_at(along) / 2.0);
    a = ((a as f32) * falloff * self.opacity.opacity_at(along)) as u8;
    (r, g, b, a)
  }
}

impl StrokeBrushShader {
  // Finds the closest point on the pre-flattened path to (p_x, p_y).
  /// Finds the closest point on the pre-flattened path to `(p_x, p_y)`.
  ///
  /// This uses a simple linear search of the flattened polyline and
  /// projects onto each segment to find the closest candidate.
  ///
  /// Parameters
  /// - `p_x`, `p_y`: the query point in device coordinates.
  ///
  /// Returns a `PointF` representing the point on the path closest to the query.
  fn closest_on_path(&self, p_x: f32, p_y: f32) -> (f32, f32) {
    if self.flattened.len() < 2 {
      let dx = p_x - self.fallback_point.x;
      let dy = p_y - self.fallback_point.y;
      return (dx * dx + dy * dy, 0.0);
    }
    let mut min_distance_sq = f32::MAX;
    let mut along = 0.0;
    for i in 0..self.flattened.len() - 1 {
      let p1 = self.flattened[i];
      let p2 = self.flattened[i + 1];
      let seg_x = p2.x - p1.x;
      let seg_y = p2.y - p1.y;
      let seg_len_sq = seg_x * seg_x + seg_y * seg_y;
      if seg_len_sq == 0.0 {
        continue;
      }
      let t = (((p_x - p1.x) * seg_x + (p_y - p1.y) * seg_y) / seg_len_sq).clamp(0.0, 1.0);
      let dx = p_x - (p1.x + seg_x * t);
      let dy = p_y - (p1.y + seg_y * t);
      let distance_sq = dx * dx + dy * dy;
      if distance_sq < min_distance_sq {
        min_distance_sq = distance_sq;
        along = self.along[i] + (self.along[i + 1] - self.along[i]) * t;
      }
    }
    (min_distance_sq, along)
  }
}
