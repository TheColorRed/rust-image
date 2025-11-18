//! Coverage mask for geometry testing.

use crate::geometry::PointF;

/// Trait for testing whether a point is inside a geometric region.
pub(crate) trait CoverageMask: Sync {
  /// Tests if the point (x, y) is inside the coverage area.
  fn contains(&self, p_x: f32, p_y: f32) -> bool;
  /// Returns an optional bounding box for this coverage mask in device coordinates
  /// as (min_x, min_y, max_x, max_y). If None, the coverage applies to the full image.
  fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
    None
  }
}

/// Coverage mask from a polygon (list of vertices).
pub(crate) struct PolygonCoverage {
  /// Pre-flattened polygon vertices.
  polygon: Vec<(f32, f32)>,
}

impl PolygonCoverage {
  /// Creates a new polygon coverage mask from a list of points.
  pub fn new(p_points: Vec<PointF>) -> Self {
    PolygonCoverage {
      polygon: p_points.iter().map(|p| (p.x, p.y)).collect(),
    }
  }

  fn compute_bounds(&self) -> (f32, f32, f32, f32) {
    if self.polygon.is_empty() {
      return (0.0, 0.0, 0.0, 0.0);
    }
    let mut min_x = self.polygon[0].0;
    let mut min_y = self.polygon[0].1;
    let mut max_x = self.polygon[0].0;
    let mut max_y = self.polygon[0].1;
    for &(x, y) in &self.polygon {
      min_x = min_x.min(x);
      min_y = min_y.min(y);
      max_x = max_x.max(x);
      max_y = max_y.max(y);
    }
    (min_x, min_y, max_x, max_y)
  }

  /// Ray-casting point-in-polygon test.
  fn point_in_polygon(&self, p_point: (f32, f32)) -> bool {
    if self.polygon.is_empty() {
      return false;
    }

    // Non-zero winding rule
    let mut winding = 0i32;
    let mut j = self.polygon.len() - 1;
    for i in 0..self.polygon.len() {
      let (xi, yi) = self.polygon[i];
      let (xj, yj) = self.polygon[j];
      if yi <= p_point.1 {
        if yj > p_point.1 {
          // upward crossing
          let is_left = (xj - xi) * (p_point.1 - yi) - (p_point.0 - xi) * (yj - yi);
          if is_left > 0.0 {
            winding += 1;
          }
        }
      } else {
        if yj <= p_point.1 {
          // downward crossing
          let is_left = (xj - xi) * (p_point.1 - yi) - (p_point.0 - xi) * (yj - yi);
          if is_left < 0.0 {
            winding -= 1;
          }
        }
      }
      j = i;
    }
    winding != 0
  }
}

impl CoverageMask for PolygonCoverage {
  fn contains(&self, p_x: f32, p_y: f32) -> bool {
    self.point_in_polygon((p_x, p_y))
  }
  fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
    Some(self.compute_bounds())
  }
}

/// Full-screen coverage (always returns true).
pub(crate) struct FullCoverage;

impl CoverageMask for FullCoverage {
  fn contains(&self, _p_x: f32, _p_y: f32) -> bool {
    true
  }
  fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
    None
  }
}

/// Brush coverage mask with hardness-based alpha falloff.
/// Applies Gaussian-like falloff for soft brushes (hardness=0.0) and sharp edges for hard brushes (hardness=1.0).
pub(crate) struct BrushCoverageMask {
  /// The underlying polygon coverage for the brush shape.
  polygon: Vec<(f32, f32)>,
  /// Brush center position.
  center_x: f32,
  center_y: f32,
  /// Maximum distance from center (brush radius).
  max_distance: f32,
  /// Hardness parameter (0.0=soft, 1.0=hard).
  hardness: f32,
}

impl BrushCoverageMask {
  /// Creates a new brush coverage mask from polygon vertices, center, and hardness.
  pub fn new(p_points: Vec<PointF>, p_center_x: f32, p_center_y: f32, p_max_distance: f32, p_hardness: f32) -> Self {
    BrushCoverageMask {
      polygon: p_points.iter().map(|p| (p.x, p.y)).collect(),
      center_x: p_center_x,
      center_y: p_center_y,
      max_distance: p_max_distance,
      hardness: p_hardness.clamp(0.0, 1.0),
    }
  }

  /// Ray-casting point-in-polygon test.
  fn point_in_polygon(&self, p_point: (f32, f32)) -> bool {
    if self.polygon.is_empty() {
      return false;
    }

    let mut inside = false;
    let mut j = self.polygon.len() - 1;

    for i in 0..self.polygon.len() {
      let (xi, yi) = self.polygon[i];
      let (xj, yj) = self.polygon[j];
      let intersects = ((yi < p_point.1 && yj >= p_point.1) || (yj < p_point.1 && yi >= p_point.1))
        && (xi + (p_point.1 - yi) / (yj - yi) * (xj - xi) < p_point.0);
      if intersects {
        inside = !inside;
      }
      j = i;
    }

    inside
  }

  /// Computes alpha falloff based on distance from center and hardness.
  /// Returns 1.0 for full opacity in hard regions, 0.0 for transparent regions.
  fn compute_alpha_falloff(&self, p_x: f32, p_y: f32) -> f32 {
    let dx = p_x - self.center_x;
    let dy = p_y - self.center_y;
    let distance = (dx * dx + dy * dy).sqrt();

    if distance >= self.max_distance {
      return 0.0;
    }

    // Normalized distance: 0.0 at center, 1.0 at max_distance
    let normalized_dist = distance / self.max_distance;

    // Apply hardness: hardness=0.0 uses smooth Gaussian-like falloff, hardness=1.0 uses sharp falloff
    // Interpolate between quadratic falloff (soft) and linear falloff (hard)
    let falloff = if self.hardness < 0.5 {
      // Soft region: use quadratic (smooth) falloff
      let t = self.hardness * 2.0; // 0.0 to 1.0 for hardness 0.0 to 0.5
      let quadratic = 1.0 - (normalized_dist * normalized_dist);
      let linear = 1.0 - normalized_dist;
      quadratic * (1.0 - t) + linear * t
    } else {
      // Hard region: use linear falloff approaching hard edge
      let t = (self.hardness - 0.5) * 2.0; // 0.0 to 1.0 for hardness 0.5 to 1.0
      let linear = 1.0 - normalized_dist;
      let hard_edge = if normalized_dist < 0.5 { 1.0 } else { 0.0 };
      linear * (1.0 - t) + hard_edge * t
    };

    falloff.max(0.0)
  }
}

impl CoverageMask for BrushCoverageMask {
  fn contains(&self, p_x: f32, p_y: f32) -> bool {
    // For brush coverage, we use the alpha falloff directly in the rasterizer
    // This trait method returns true if the point is inside the brush polygon region
    self.point_in_polygon((p_x, p_y))
  }
  fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
    if self.polygon.is_empty() {
      return None;
    }
    let mut min_x = self.polygon[0].0;
    let mut min_y = self.polygon[0].1;
    let mut max_x = self.polygon[0].0;
    let mut max_y = self.polygon[0].1;
    for &(x, y) in &self.polygon {
      min_x = min_x.min(x);
      min_y = min_y.min(y);
      max_x = max_x.max(x);
      max_y = max_y.max(y);
    }
    // Inflate by max_distance so shading beyond polygon is covered
    Some((min_x - self.max_distance, min_y - self.max_distance, max_x + self.max_distance, max_y + self.max_distance))
  }
}
