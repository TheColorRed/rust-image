use std::fmt::Display;

use crate::geometry::Size;

use super::point::Point;
use super::pointf::PointF;
use super::viewbox::{AspectRatio, ViewBox};

#[derive(Clone, Debug, PartialEq)]
/// A segment in a path.
pub enum Segment {
  /// A straight line segment to a point.
  Line {
    /// The endpoint of the line segment.
    to: PointF,
  },
  /// A quadratic Bezier curve with one control point.
  Quadratic {
    /// The control point of the quadratic curve.
    ctrl: PointF,
    /// The endpoint of the quadratic curve.
    to: PointF,
  },
  /// A cubic Bezier curve with two control points.
  Cubic {
    /// The first control point of the cubic curve.
    ctrl1: PointF,
    /// The second control point of the cubic curve.
    ctrl2: PointF,
    /// The endpoint of the cubic curve.
    to: PointF,
  },
}

#[derive(Clone, Debug)]
/// A path represents a geometric shape made of lines and curves.
/// Paths are geometric utilities that can be used for drawing, following, effects, and more.
/// A path is not a closed shape. Use an Area for closed shapes.
pub struct Path {
  /// The starting point of the path.
  start: PointF,
  /// The segments that make up the path.
  segments: Vec<Segment>,
}

impl Path {
  /// Creates a new empty path.
  pub fn new() -> Path {
    Path::default()
  }
  /// Creates a simple line path from point A to point B.
  pub fn line(p_from: impl Into<PointF>, p_to: impl Into<PointF>) -> Path {
    let mut path = Path::new();
    path.with_move_to(p_from).with_line_to(p_to);
    path
  }
  /// Sets the starting point of the path (move to).
  pub fn with_move_to(&mut self, p_start: impl Into<PointF>) -> &mut Self {
    self.start = p_start.into();
    self
  }

  /// Adds a straight line segment to the path.
  pub fn with_line_to(&mut self, p_to: impl Into<PointF>) -> &mut Self {
    self.segments.push(Segment::Line { to: p_to.into() });
    self
  }

  /// Adds a quadratic Bezier curve segment to the path.
  pub fn with_quad_to(&mut self, p_ctrl: impl Into<PointF>, p_to: impl Into<PointF>) -> &mut Self {
    self.segments.push(Segment::Quadratic {
      ctrl: p_ctrl.into(),
      to: p_to.into(),
    });
    self
  }

  /// Adds a cubic Bezier curve segment to the path.
  pub fn with_cubic_to(
    &mut self, p_ctrl1: impl Into<PointF>, p_ctrl2: impl Into<PointF>, p_to: impl Into<PointF>,
  ) -> &mut Self {
    self.segments.push(Segment::Cubic {
      ctrl1: p_ctrl1.into(),
      ctrl2: p_ctrl2.into(),
      to: p_to.into(),
    });
    self
  }

  /// Sets the starting point of the path (move to).
  /// Returns the starting point of the path.
  pub fn start(&self) -> PointF {
    self.start
  }

  /// Returns the segments of the path.
  pub fn segments(&self) -> &[Segment] {
    &self.segments
  }

  /// Returns all points in the path as a flat list of PointF.
  /// This includes the start point and all segment endpoints.
  pub fn points(&self) -> Vec<PointF> {
    let mut pts = vec![self.start];
    for segment in &self.segments {
      match segment {
        Segment::Line { to } => pts.push(*to),
        Segment::Quadratic { to, .. } => pts.push(*to),
        Segment::Cubic { to, .. } => pts.push(*to),
      }
    }
    pts
  }

  /// Returns the point at parameter t (0 to 1) along the entire path.
  /// Uses uniform parametric distribution (not arc-length).
  pub fn point_at(&self, p_t: f32) -> PointF {
    if self.segments.is_empty() {
      return self.start;
    }

    let clamped_t = p_t.clamp(0.0, 1.0);
    let num_segments = self.segments.len() as f32;
    let segment_index = (clamped_t * num_segments).floor() as usize;
    let segment_index = segment_index.min(self.segments.len() - 1);
    let local_t = (clamped_t * num_segments) - segment_index as f32;

    let prev_point = if segment_index == 0 {
      self.start
    } else {
      self.point_at_segment(segment_index - 1, 1.0)
    };

    eval_segment(prev_point, &self.segments[segment_index], local_t)
  }

  /// Returns the point at parameter t within a specific segment.
  pub fn point_at_segment(&self, p_segment_idx: usize, p_t: f32) -> PointF {
    if p_segment_idx >= self.segments.len() {
      return self.points().last().copied().unwrap_or(self.start);
    }

    let prev_point = if p_segment_idx == 0 {
      self.start
    } else {
      self.point_at_segment(p_segment_idx - 1, 1.0)
    };

    eval_segment(prev_point, &self.segments[p_segment_idx], p_t)
  }

  /// Flattens the path into a polyline (list of points) with the given tolerance.
  /// Tolerance determines how closely the polyline approximates curves.
  pub fn flatten(&self, p_tolerance: f32) -> Vec<PointF> {
    let mut result = vec![self.start];
    let mut current = self.start;

    for segment in &self.segments {
      match segment {
        Segment::Line { to } => {
          result.push(*to);
          current = *to;
        }
        Segment::Quadratic { ctrl, to } => {
          let subdivisions = calculate_subdivisions(current, *ctrl, *to, p_tolerance);
          for i in 1..=subdivisions {
            let t = i as f32 / subdivisions as f32;
            let pt = eval_segment(current, segment, t);
            result.push(pt);
          }
          current = *to;
        }
        Segment::Cubic { ctrl1, ctrl2, to } => {
          let subdivisions = calculate_subdivisions_cubic(current, *ctrl1, *ctrl2, *to, p_tolerance);
          for i in 1..=subdivisions {
            let t = i as f32 / subdivisions as f32;
            let pt = eval_segment(current, segment, t);
            result.push(pt);
          }
          current = *to;
        }
      }
    }

    result
  }

  /// Returns an approximate length of the path.
  pub fn length(&self) -> f32 {
    let flattened = self.flatten(0.5);
    let mut total = 0.0;
    for i in 1..flattened.len() {
      total += flattened[i - 1].distance_to(flattened[i]);
    }
    total
  }

  /// Returns the bounding box of the path as (min_x, min_y, max_x, max_y).
  pub fn bounds(&self) -> (f32, f32, f32, f32) {
    let pts = self.flatten(0.5);
    if pts.is_empty() {
      return (0.0, 0.0, 0.0, 0.0);
    }

    let mut min_x = pts[0].x;
    let mut min_y = pts[0].y;
    let mut max_x = pts[0].x;
    let mut max_y = pts[0].y;

    for pt in &pts {
      min_x = min_x.min(pt.x);
      min_y = min_y.min(pt.y);
      max_x = max_x.max(pt.x);
      max_y = max_y.max(pt.y);
    }

    (min_x, min_y, max_x, max_y)
  }

  /// Converts the path to a list of integer points (for raster operations).
  /// This flattens curves and rounds coordinates to integers.
  pub fn to_points(&self, p_tolerance: f32) -> Vec<Point> {
    self.flatten(p_tolerance).iter().map(|p| Point::from(*p)).collect()
  }

  /// Finds the closest point on the path to the given coordinates and returns the parameter t.
  /// This is useful for gradients and effects that need to map pixels to path positions.
  pub fn closest_time(&self, p_x: f32, p_y: f32) -> f32 {
    let query = PointF::new(p_x, p_y);
    let flattened = self.flatten(1.0);

    if flattened.len() < 2 {
      return 0.0;
    }

    let mut min_distance = f32::MAX;
    let mut closest_t = 0.0;
    let total_segments = (flattened.len() - 1) as f32;

    for i in 0..flattened.len() - 1 {
      let p1 = flattened[i];
      let p2 = flattened[i + 1];

      let segment_vec = p2 - p1;
      let query_vec = query - p1;
      let segment_len_sq = segment_vec.length_squared();

      if segment_len_sq == 0.0 {
        continue;
      }

      let local_t = (query_vec.dot(segment_vec) / segment_len_sq).clamp(0.0, 1.0);
      let closest_point = p1.lerp(p2, local_t);
      let distance = query.distance_to(closest_point);

      if distance < min_distance {
        min_distance = distance;
        // Map to global t (0 to 1 across entire path)
        closest_t = (i as f32 + local_t) / total_segments;
      }
    }

    closest_t
  }

  /// Finds the closest point on the path to the given coordinates, returning the point coordinates.
  pub fn closest_point(&self, p_x: f32, p_y: f32) -> PointF {
    let query = PointF::new(p_x, p_y);
    let flattened = self.flatten(1.0);

    if flattened.len() < 2 {
      return self.start;
    }

    let mut min_distance = f32::MAX;
    let mut closest_point = flattened[0];

    for i in 0..flattened.len() - 1 {
      let p1 = flattened[i];
      let p2 = flattened[i + 1];

      let segment_vec = p2 - p1;
      let query_vec = query - p1;
      let segment_len_sq = segment_vec.length_squared();

      if segment_len_sq == 0.0 {
        continue;
      }

      let local_t = (query_vec.dot(segment_vec) / segment_len_sq).clamp(0.0, 1.0);
      let candidate = p1.lerp(p2, local_t);
      let distance = query.distance_to(candidate);

      if distance < min_distance {
        min_distance = distance;
        closest_point = candidate;
      }
    }

    closest_point
  }

  /// Transforms this path to fit within a viewport using a ViewBox.
  ///
  /// This enables SVG-style resolution-independent rendering where the path
  /// is defined in abstract coordinates and scaled to fit a viewport.
  ///
  /// # Arguments
  ///
  /// * `viewbox` - The source coordinate system (defines the abstract space)
  /// * `viewport_width` - Target width in pixels
  /// * `viewport_height` - Target height in pixels
  /// * `aspect_ratio` - How to preserve aspect ratio when scaling
  ///
  /// # Example
  ///
  /// ```
  /// use abra::geometry::{Path, ViewBox, AspectRatio};
  ///
  /// // Define a path in 0-100 coordinate space
  /// let path = Path::rect((0.0, 0.0), 100.0, 100.0);
  /// let viewbox = ViewBox::new(0.0, 0.0, 100.0, 100.0);
  ///
  /// // Render at 500x500 pixels
  /// let scaled = path.transform_to_viewport(&viewbox, 500.0, 500.0, AspectRatio::default());
  /// ```
  pub fn transform_to_viewport(
    &self, p_viewbox: &ViewBox, p_viewport_width: f32, p_viewport_height: f32, p_aspect_ratio: AspectRatio,
  ) -> Path {
    let mut transformed = Path {
      start: p_viewbox.map_point(self.start, p_viewport_width, p_viewport_height, p_aspect_ratio),
      segments: Vec::with_capacity(self.segments.len()),
    };

    for segment in &self.segments {
      let transformed_segment = match segment {
        Segment::Line { to } => Segment::Line {
          to: p_viewbox.map_point(*to, p_viewport_width, p_viewport_height, p_aspect_ratio),
        },
        Segment::Quadratic { ctrl, to } => Segment::Quadratic {
          ctrl: p_viewbox.map_point(*ctrl, p_viewport_width, p_viewport_height, p_aspect_ratio),
          to: p_viewbox.map_point(*to, p_viewport_width, p_viewport_height, p_aspect_ratio),
        },
        Segment::Cubic { ctrl1, ctrl2, to } => Segment::Cubic {
          ctrl1: p_viewbox.map_point(*ctrl1, p_viewport_width, p_viewport_height, p_aspect_ratio),
          ctrl2: p_viewbox.map_point(*ctrl2, p_viewport_width, p_viewport_height, p_aspect_ratio),
          to: p_viewbox.map_point(*to, p_viewport_width, p_viewport_height, p_aspect_ratio),
        },
      };
      transformed.segments.push(transformed_segment);
    }

    transformed
  }

  /// Convenience method to create a ViewBox from this path's bounds.
  /// Useful for normalizing a path to its bounding box.
  pub fn to_viewbox(&self) -> ViewBox {
    let (min_x, min_y, max_x, max_y) = self.bounds();
    ViewBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
  }

  /// Fits this path into the given viewport size preserving aspect ratio (Meet).
  /// This uses the path's bounds as its implicit viewBox.
  pub fn fit(&self, p_size: impl Into<Size>) -> Path {
    let size = p_size.into();
    self.transform_to_viewport(&self.to_viewbox(), size.width, size.height, AspectRatio::meet())
  }

  /// Fits this path into a square viewport of the given size (Meet).
  pub fn fit_square(&self, p_size: impl Into<f32>) -> Path {
    let size = p_size.into();
    self.fit(Size::new(size, size))
  }

  /// Fits this path into the given viewport using a specific aspect ratio policy.
  /// This uses the path's bounds as its implicit viewBox.
  pub fn fit_with_aspect(&self, p_viewport_width: f32, p_viewport_height: f32, p_aspect_ratio: AspectRatio) -> Path {
    self.transform_to_viewport(&self.to_viewbox(), p_viewport_width, p_viewport_height, p_aspect_ratio)
  }

  /// Stretches this path non-uniformly to fill the viewport (no aspect ratio preservation).
  pub fn stretch(&self, p_size: impl Into<Size>) -> Path {
    let size = p_size.into();
    self.transform_to_viewport(&self.to_viewbox(), size.width, size.height, AspectRatio::none())
  }

  /// Samples uniformly spaced points along the path based on spacing ratio.
  /// - `p_spacing_ratio`: Fraction of path length between samples (0.0 to 1.0).
  ///   For example, 0.1 means sample every 10% of the path length.
  pub fn sample_points(&self, p_spacing_ratio: f32) -> Vec<PointF> {
    if self.segments.is_empty() {
      return vec![self.start];
    }

    let spacing = p_spacing_ratio.clamp(0.0, 1.0);
    let mut samples = vec![self.start];

    if spacing <= 0.0 {
      return samples;
    }

    let mut current_t = spacing;
    while current_t < 1.0 {
      samples.push(self.point_at(current_t));
      current_t += spacing;
    }

    samples.push(self.point_at(1.0));
    samples
  }

  /// Scales this path uniformly to cover the viewport (may crop) using Slice.
  pub fn cover(&self, p_size: impl Into<Size>) -> Path {
    let size = p_size.into();
    self.transform_to_viewport(&self.to_viewbox(), size.width, size.height, AspectRatio::slice())
  }
}

impl Display for Path {
  /// Displays the path as a string.
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Path(start: {}, segments: {})", self.start, self.segments.len())
  }
}

impl Default for Path {
  fn default() -> Self {
    Path {
      start: PointF::zero(),
      segments: Vec::new(),
    }
  }
}

/// Evaluates a segment at parameter t (0 to 1).
fn eval_segment(p_prev: PointF, p_segment: &Segment, p_t: f32) -> PointF {
  match p_segment {
    Segment::Line { to } => p_prev.lerp(*to, p_t),
    Segment::Quadratic { ctrl, to } => {
      // Quadratic Bezier: B(t) = (1-t)^2 * P0 + 2(1-t)t * P1 + t^2 * P2
      let u = 1.0 - p_t;
      let uu = u * u;
      let tt = p_t * p_t;
      p_prev * uu + *ctrl * (2.0 * u * p_t) + *to * tt
    }
    Segment::Cubic { ctrl1, ctrl2, to } => {
      // Cubic Bezier: B(t) = (1-t)^3 * P0 + 3(1-t)^2*t * P1 + 3(1-t)*t^2 * P2 + t^3 * P3
      let u = 1.0 - p_t;
      let uu = u * u;
      let uuu = uu * u;
      let tt = p_t * p_t;
      let ttt = tt * p_t;
      p_prev * uuu + *ctrl1 * (3.0 * uu * p_t) + *ctrl2 * (3.0 * u * tt) + *to * ttt
    }
  }
}

/// Calculates the number of subdivisions needed for a quadratic curve.
fn calculate_subdivisions(p_p0: PointF, p_p1: PointF, p_p2: PointF, p_tolerance: f32) -> usize {
  // Estimate curve length and divide by tolerance
  let chord_len = p_p0.distance_to(p_p2);
  let control_dist = p_p0.distance_to(p_p1) + p_p1.distance_to(p_p2);
  let estimate_len = (chord_len + control_dist) * 0.5;
  (estimate_len / p_tolerance).ceil().max(2.0) as usize
}

/// Calculates the number of subdivisions needed for a cubic curve.
fn calculate_subdivisions_cubic(p_p0: PointF, p_p1: PointF, p_p2: PointF, p_p3: PointF, p_tolerance: f32) -> usize {
  let chord_len = p_p0.distance_to(p_p3);
  let control_dist = p_p0.distance_to(p_p1) + p_p1.distance_to(p_p2) + p_p2.distance_to(p_p3);
  let estimate_len = (chord_len + control_dist) * 0.5;
  (estimate_len / p_tolerance).ceil().max(2.0) as usize
}
