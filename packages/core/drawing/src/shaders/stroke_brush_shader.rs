use crate::Shader;
use core::{Path, PointF};

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
  /// Path forming the stroke centerline.
  path: Path,
  /// Maximum influence distance from the centerline.
  max_distance: f32,
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
  /// - `p_max_distance`: maximum influence distance from the centerline
  /// - `p_hardness`: falloff hardness in [0.0, 1.0]
  ///
  /// Example
  /// ```ignore
  /// let s = StrokeBrushShader::new(inner, path.clone(), 6.0, 0.7);
  /// ```
  pub fn new(
    p_inner: Box<dyn Shader + Send + Sync>, p_path: Path, p_max_distance: impl Into<f64>, p_hardness: impl Into<f64>,
  ) -> Self {
    // Pre-flatten the path to a set of points; choose a tolerance that balances accuracy and performance.
    let flattened = p_path.flatten(1.0);
    StrokeBrushShader {
      inner: p_inner,
      path: p_path,
      max_distance: p_max_distance.into() as f32,
      hardness: p_hardness.into().clamp(0.0, 1.0) as f32,
      flattened,
    }
  }

  /// Computes the falloff (a factor in 0..1) given a perpendicular distance
  /// from the stroke's centerline. This function is used by `shade` to
  /// modulate the alpha channel.
  /// - `p_distance`: perpendicular distance from the stroke centerline.
  fn compute_alpha_falloff_from_distance(&self, p_distance: f32) -> f32 {
    if p_distance >= self.max_distance {
      return 0.0;
    }
    let normalized_dist = p_distance / self.max_distance;

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
    // Compute closest point on the path and distance to it
    let center_point = self.closest_point_on_flattened(p_x, p_y);

    let dx = p_x - center_point.x;
    let dy = p_y - center_point.y;
    let dist = (dx * dx + dy * dy).sqrt();
    let falloff = self.compute_alpha_falloff_from_distance(dist);
    // (Debug prints removed)
    a = ((a as f32) * falloff) as u8;
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
  fn closest_point_on_flattened(&self, p_x: f32, p_y: f32) -> PointF {
    if self.flattened.len() < 2 {
      return self.path.start();
    }
    let query = PointF::new(p_x, p_y);
    let mut min_distance = f32::MAX;
    let mut closest = self.flattened[0];
    for i in 0..self.flattened.len() - 1 {
      let p1 = self.flattened[i];
      let p2 = self.flattened[i + 1];
      let seg_vec = PointF::new(p2.x - p1.x, p2.y - p1.y);
      let seg_len_sq = seg_vec.length_squared();
      if seg_len_sq == 0.0 {
        continue;
      }
      let query_vec = PointF::new(query.x - p1.x, query.y - p1.y);
      let t = (query_vec.dot(seg_vec) / seg_len_sq).clamp(0.0, 1.0);
      let candidate = p1.lerp(p2, t);
      let dist = query.distance_to(candidate);
      if dist < min_distance {
        min_distance = dist;
        closest = candidate;
      }
    }
    closest
  }
}
