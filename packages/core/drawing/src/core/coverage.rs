//! Coverage mask implementations used by the rasterizer to test sample inclusion.
//!
//! A `CoverageMask` is a compact geometry abstraction used by the
//! rasterizer to limit its work and to determine which sub-pixel samples
//! are part of a fill or stroke. This module implements several useful
//! masks:
//! * `PolygonCoverage` — point-in-polygon coverage for arbitrary polygons.
//! * `BrushCoverageMask` — a polygonal region with radial alpha falloff
//!   (useful for painting brushes).
//! * `FullCoverage` — a trivial mask covering the entire image.
//!
//! Usage pattern
//! - The rasterizer queries `bounds()` to limit the set of pixel rows
//!   it needs to process. Implement `bounds()` correctly to minimize
//!   rasterization overhead.
//! - For each sub-pixel sample, `contains()` answers whether that sample
//!   is within the shape. Some masks (like `BrushCoverageMask`) also
//!   provide falloff functions for continuous alpha if needed.
//!
//! Example
//! ```ignore
//! let poly = PolygonCoverage::new(vec![PointF::new(0.0, 0.0), PointF::new(10.0, 0.0), PointF::new(10.0, 10.0)]);
//! assert!(poly.contains(5.0, 5.0));
//! let bounds = poly.bounds().unwrap();
//! ```

use core::PointF;

/// A trait representing a geometric coverage test for sample points.
///
/// `CoverageMask` is a small abstraction allowing various shapes and
/// brush falloffs to be used by the rasterizer. Implementations should
/// be `Sync` as they may be queried from multiple threads during
/// parallel rasterization.
///
/// Returns an optional bounding box as `(min_x, min_y, max_x, max_y)` in
/// device coordinates to allow the rasterizer to limit the set of pixels
/// it examines.
///
/// Example
/// ```ignore
/// let poly = PolygonCoverage::new(vec![PointF::new(0.0,0.0), PointF::new(10.0,0.0), PointF::new(10.0,10.0)]);
/// assert!(poly.contains(5.0, 5.0));
/// ```
pub trait CoverageMask: Sync {
  /// Tests if the point (x, y) is inside the coverage area.
  fn contains(&self, p_x: f32, p_y: f32) -> bool;
  /// Returns an optional bounding box for this coverage mask in device coordinates
  /// as (min_x, min_y, max_x, max_y). If None, the coverage applies to the full image.
  fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
    None
  }
}

/// A polygon-based coverage mask.
///
/// This implementation stores a flat vector of vertices and performs a
/// point-in-polygon test for `contains`. It also computes a bounding box
/// that can be used by the rasterizer to optimize pixel iteration.
pub struct PolygonCoverage {
  /// Pre-flattened polygon vertices.
  polygon: Vec<(f32, f32)>,
}

impl PolygonCoverage {
  /// Creates a new `PolygonCoverage` from a vector of `PointF` vertices.
  ///
  /// The polygon does not need to be closed; the algorithm handles the
  /// last-to-first edge implicitly. Coordinates are assumed to be in the
  /// rasterization device coordinate space.
  ///
  /// Parameters
  /// - `p_points`: Vector of polygon vertices in order.
  ///
  /// Example
  /// ```ignore
  /// let poly = PolygonCoverage::new(vec![PointF::new(0.0,0.0), PointF::new(20.0,0.0), PointF::new(20.0,20.0)]);
  /// ```
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

  /// Ray-casting point-in-polygon test using the non-zero winding rule.
  ///
  /// Returns `true` when the given point is inside the polygon. This
  /// method is an implementation detail but is used by `contains`.
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
