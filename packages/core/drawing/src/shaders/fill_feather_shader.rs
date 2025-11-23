use crate::Shader;
use core::PointF;

/// Fill feather shader: wraps an inner shader and modulates its alpha based on
/// the distance to the path boundary. The shader computes the closest point on
/// the provided `Path` and treats that as the boundary; inside the path alpha
/// is reduced near the edge based on `max_distance`.
pub(crate) struct FillFeatherShader {
  inner: Box<dyn Shader + Send + Sync>,
  flattened: Vec<PointF>,
  max_distance: f32,
}

impl FillFeatherShader {
  /// Creates a new `FillFeatherShader` from pre-flattened path points in local coordinates
  pub fn new_from_flattened(
    p_inner: Box<dyn Shader + Send + Sync>, p_flattened: Vec<PointF>, p_max_distance: impl Into<f32>,
  ) -> Self {
    let flattened = p_flattened;
    FillFeatherShader {
      inner: p_inner,
      flattened,
      max_distance: p_max_distance.into(),
    }
  }

  fn compute_inner_falloff(&self, p_x: f32, p_y: f32) -> f32 {
    if self.max_distance <= 0.0 {
      return 1.0;
    }
    let center = PointF::new(p_x, p_y);
    // Find closest point on flattened polyline
    if self.flattened.len() < 2 {
      return 1.0;
    }
    let mut min_distance = f32::MAX;
    for i in 0..self.flattened.len() - 1 {
      let p1 = self.flattened[i];
      let p2 = self.flattened[i + 1];
      let seg_vec = PointF::new(p2.x - p1.x, p2.y - p1.y);
      let seg_len_sq = seg_vec.length_squared();
      if seg_len_sq == 0.0 {
        continue;
      }
      let query_vec = PointF::new(center.x - p1.x, center.y - p1.y);
      let t = (query_vec.dot(seg_vec) / seg_len_sq).clamp(0.0, 1.0);
      let candidate = p1.lerp(p2, t);
      let d = center.distance_to(candidate);
      if d < min_distance {
        min_distance = d;
      }
    }
    let alpha_factor = (min_distance / self.max_distance).clamp(0.0, 1.0);
    alpha_factor
  }
}

impl Shader for FillFeatherShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let (r, g, b, mut a) = self.inner.shade(p_x, p_y);
    let falloff = self.compute_inner_falloff(p_x, p_y);
    a = ((a as f32) * falloff) as u8;
    (r, g, b, a)
  }
}
