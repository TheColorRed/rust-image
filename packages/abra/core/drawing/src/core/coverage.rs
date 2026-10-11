//! Coverage mask implementations used by the rasterizer to test sample inclusion.
//!
//! A `CoverageMask` is a compact geometry abstraction used by the
//! rasterizer to limit its work and to determine which sub-pixel samples
//! are part of a fill or stroke. This module implements several useful
//! masks:
//! * `PolygonCoverage` — point-in-polygon coverage for arbitrary polygons.
//! * `StrokeCoverage` — everything within a radius of a path, like a round brush dragged along it.
//! * `RectCoverage` — an upright rectangle.
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

use abra_core::{LineSegment, PointF, Size};

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
  bounds: Option<(f32, f32, f32, f32)>,
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
    let mut polygon = Vec::with_capacity(p_points.len());
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    for point in p_points {
      bounds = Some(match bounds {
        Some((min_x, min_y, max_x, max_y)) => {
          (min_x.min(point.x), min_y.min(point.y), max_x.max(point.x), max_y.max(point.y))
        }
        None => (point.x, point.y, point.x, point.y),
      });
      polygon.push((point.x, point.y));
    }
    PolygonCoverage { polygon, bounds }
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
    self.bounds
  }
}

/// Every point within `radius` of a path of straight lines, like a round brush dragged along it. A path of one
/// point is a circle, and one of two is a line with round ends.
pub struct StrokeCoverage {
  segments: Vec<LineSegment>,
  radius: f32,
  bounds: Option<(f32, f32, f32, f32)>,
}

impl StrokeCoverage {
  /// Creates the stroke of a round brush with the given radius dragged through the points, in order.
  /// - `p_points`: The points the brush passes through. No points covers nothing.
  /// - `p_radius`: The radius of the brush.
  pub fn new(p_points: &[PointF], p_radius: f32) -> Self {
    let radius = p_radius.abs();
    let segments: Vec<LineSegment> = match p_points {
      [] => Vec::new(),
      [point] => vec![LineSegment::new(*point, *point)],
      points => points.windows(2).map(|pair| LineSegment::new(pair[0], pair[1])).collect(),
    };
    let bounds = bounds_of(p_points)
      .map(|(min_x, min_y, max_x, max_y)| (min_x - radius, min_y - radius, max_x + radius, max_y + radius));
    StrokeCoverage {
      segments,
      radius,
      bounds,
    }
  }
}

impl CoverageMask for StrokeCoverage {
  fn contains(&self, p_x: f32, p_y: f32) -> bool {
    let limit = self.radius * self.radius;
    self.segments.iter().any(|segment| segment.distance_squared_to((p_x, p_y)) <= limit)
  }
  fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
    self.bounds
  }
}

/// An upright rectangle.
pub struct RectCoverage {
  bounds: (f32, f32, f32, f32),
}

impl RectCoverage {
  /// Creates a rectangle with its top-left corner at `p_origin`.
  pub fn new(p_origin: impl Into<PointF>, p_size: impl Into<Size>) -> Self {
    let (origin, size) = (p_origin.into(), p_size.into());
    RectCoverage {
      bounds: (origin.x, origin.y, origin.x + size.width, origin.y + size.height),
    }
  }

  /// Creates a rectangle centered on `p_center`.
  pub fn centered(p_center: impl Into<PointF>, p_size: impl Into<Size>) -> Self {
    let (center, size) = (p_center.into(), p_size.into());
    RectCoverage::new((center.x - size.width / 2.0, center.y - size.height / 2.0), size)
  }
}

impl CoverageMask for RectCoverage {
  fn contains(&self, p_x: f32, p_y: f32) -> bool {
    let (min_x, min_y, max_x, max_y) = self.bounds;
    p_x >= min_x && p_x <= max_x && p_y >= min_y && p_y <= max_y
  }
  fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
    Some(self.bounds)
  }
}

/// The bounding box of some points as `(min_x, min_y, max_x, max_y)`, or `None` when there are none.
fn bounds_of(p_points: &[PointF]) -> Option<(f32, f32, f32, f32)> {
  let first = p_points.first()?;
  Some(p_points.iter().fold((first.x, first.y, first.x, first.y), |(min_x, min_y, max_x, max_y), point| {
    (min_x.min(point.x), min_y.min(point.y), max_x.max(point.x), max_y.max(point.y))
  }))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_stroke_covers_everything_near_its_path() {
    let stroke = StrokeCoverage::new(&[PointF::new(0, 0), PointF::new(10, 0), PointF::new(10, 10)], 2.0);
    assert!(stroke.contains(5.0, 1.5));
    assert!(stroke.contains(11.5, 5.0));
    assert!(!stroke.contains(5.0, 5.0));
    // Round ends.
    assert!(stroke.contains(-1.4, -1.4));
    assert!(!stroke.contains(-2.0, -2.0));
    assert_eq!(stroke.bounds(), Some((-2.0, -2.0, 12.0, 12.0)));
  }

  #[test]
  fn a_stroke_of_one_point_is_a_circle_and_of_none_is_nothing() {
    let dot = StrokeCoverage::new(&[PointF::new(5, 5)], 3.0);
    assert!(dot.contains(7.0, 7.0));
    assert!(!dot.contains(8.0, 8.0));
    let nothing = StrokeCoverage::new(&[], 3.0);
    assert!(!nothing.contains(0.0, 0.0));
    assert_eq!(nothing.bounds(), None);
  }

  #[test]
  fn a_centered_rectangle_is_placed_around_its_center() {
    let rect = RectCoverage::centered((50, 50), (10, 4));
    assert_eq!(rect.bounds(), Some((45.0, 48.0, 55.0, 52.0)));
    assert!(rect.contains(54.0, 51.0));
    assert!(!rect.contains(56.0, 50.0));
  }
}
