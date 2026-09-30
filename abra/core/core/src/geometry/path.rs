use std::fmt::Display;

use crate::Size;

use super::pointf::PointF;
use super::rect::{AspectRatio, Rect};

#[derive(Clone, Copy, Debug, PartialEq)]
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

impl Segment {
  /// The point the segment ends at.
  pub fn end(&self) -> PointF {
    match self {
      Segment::Line { to } | Segment::Quadratic { to, .. } | Segment::Cubic { to, .. } => *to,
    }
  }
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
  /// An unresolved direction angle, used by gradient fills.
  angle_degrees: Option<f32>,
}

impl Path {
  /// Creates a new empty path.
  pub fn new() -> Path {
    Path::default()
  }
  /// Creates a simple line path from point A to point B.
  pub fn line(p_from: impl Into<PointF>, p_to: impl Into<PointF>) -> Path {
    let mut path = Path::new();
    path.move_to(p_from).line_to(p_to);
    path
  }
  /// Sets the starting point of the path (move to).
  pub fn move_to(&mut self, p_start: impl Into<PointF>) -> &mut Self {
    self.start = p_start.into();
    self
  }

  /// Adds a straight line segment to the path.
  pub fn line_to(&mut self, p_to: impl Into<PointF>) -> &mut Self {
    self.segments.push(Segment::Line { to: p_to.into() });
    self
  }

  /// Adds a quadratic Bezier curve segment to the path.
  pub fn quad_to(&mut self, p_ctrl: impl Into<PointF>, p_to: impl Into<PointF>) -> &mut Self {
    self.segments.push(Segment::Quadratic {
      ctrl: p_ctrl.into(),
      to: p_to.into(),
    });
    self
  }

  /// Adds a cubic Bezier curve segment to the path.
  pub fn cubic_to(
    &mut self, p_ctrl1: impl Into<PointF>, p_ctrl2: impl Into<PointF>, p_to: impl Into<PointF>,
  ) -> &mut Self {
    self.segments.push(Segment::Cubic {
      ctrl1: p_ctrl1.into(),
      ctrl2: p_ctrl2.into(),
      to: p_to.into(),
    });
    self
  }

  /// Gets the starting point of the path.
  pub fn start(&self) -> PointF {
    self.start
  }
  /// Gets the ending point of the path.
  pub fn end(&self) -> PointF {
    self.segments.last().map_or(self.start, Segment::end)
  }
  /// Returns the segments of the path.
  pub fn segments(&self) -> &[Segment] {
    &self.segments
  }

  /// Returns the unresolved direction angle, if this path represents one.
  pub fn angle_degrees(&self) -> Option<f32> {
    self.angle_degrees
  }

  /// Returns all points in the path as a flat list of PointF.
  /// This includes the start point and all segment endpoints.
  pub fn points(&self) -> Vec<PointF> {
    std::iter::once(self.start).chain(self.segments.iter().map(Segment::end)).collect()
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

    eval_segment(self.segment_start(segment_index), &self.segments[segment_index], local_t)
  }

  /// Returns the point at parameter t within a specific segment.
  pub fn point_at_segment(&self, p_segment_idx: usize, p_t: f32) -> PointF {
    match self.segments.get(p_segment_idx) {
      Some(segment) => eval_segment(self.segment_start(p_segment_idx), segment, p_t),
      None => self.end(),
    }
  }

  /// The point a segment starts at: the end of the previous segment, or the path start for the first one.
  fn segment_start(&self, p_segment_idx: usize) -> PointF {
    if p_segment_idx == 0 { self.start } else { self.segments[p_segment_idx - 1].end() }
  }

  /// Flattens the path into a polyline (list of points) with the given tolerance.
  /// Tolerance determines how closely the polyline approximates curves.
  pub fn flatten(&self, p_tolerance: f32) -> Vec<PointF> {
    let mut result = vec![self.start];
    let mut current = self.start;

    for segment in &self.segments {
      let pieces = match segment {
        Segment::Line { .. } => 1,
        Segment::Quadratic { ctrl, to } => subdivisions(&[current, *ctrl, *to], p_tolerance),
        Segment::Cubic { ctrl1, ctrl2, to } => subdivisions(&[current, *ctrl1, *ctrl2, *to], p_tolerance),
      };
      for i in 1..=pieces {
        result.push(eval_segment(current, segment, i as f32 / pieces as f32));
      }
      current = segment.end();
    }

    result
  }

  /// Returns an approximate length of the path.
  pub fn length(&self) -> f32 {
    self.flatten(0.5).windows(2).map(|pair| pair[0].distance_to(pair[1])).sum()
  }

  /// Returns the bounding box of the path.
  pub fn bounds(&self) -> Rect {
    Rect::from_points(self.flatten(0.5))
  }

  /// Finds the closest point on the path to the given coordinates and returns the parameter t.
  /// This is useful for gradients and effects that need to map pixels to path positions.
  pub fn closest_time(&self, p_x: f32, p_y: f32) -> f32 {
    self.closest(PointF::new(p_x, p_y), 1.0).map_or(0.0, |(t, _)| t)
  }

  /// Finds the closest point on the path to the given coordinates, returning the point coordinates.
  pub fn closest_point(&self, p_x: f32, p_y: f32) -> PointF {
    self.closest(PointF::new(p_x, p_y), 0.5).map_or(self.start, |(_, point)| point)
  }

  /// The closest position on the flattened path to `p_query`, as `(t, point)` where `t` runs from 0 to 1 across
  /// the flattened segments. A closed path (first point on the last) includes its closing segment. `None` when the
  /// path has fewer than two points.
  fn closest(&self, p_query: PointF, p_tolerance: f32) -> Option<(f32, PointF)> {
    let flattened = self.flatten(p_tolerance);
    let len = flattened.len();
    if len < 2 {
      return None;
    }

    let is_closed = flattened[0].distance_to(flattened[len - 1]) < 0.1;
    let segments = if is_closed { len } else { len - 1 };
    let mut best = (f32::MAX, 0.0, flattened[0]);

    for i in 0..segments {
      let (p1, p2) = (flattened[i], flattened[(i + 1) % len]);
      let segment_vec = p2 - p1;
      let segment_len_sq = segment_vec.length_squared();
      let local_t =
        if segment_len_sq < 1e-4 { 0.0 } else { ((p_query - p1).dot(segment_vec) / segment_len_sq).clamp(0.0, 1.0) };
      let candidate = p1.lerp(p2, local_t);
      let distance = p_query.distance_to(candidate);
      if distance < best.0 {
        best = (distance, (i as f32 + local_t) / segments as f32, candidate);
      }
    }

    Some((best.1, best.2))
  }

  /// Maps this path from the `p_source` coordinate system into a viewport of `p_viewport` size, the way an SVG
  /// `viewBox` maps into its element. See [`Rect::map_point`].
  pub fn transform_to_viewport(
    &self, p_source: &Rect, p_viewport: impl Into<Size>, p_aspect_ratio: AspectRatio,
  ) -> Path {
    let viewport = p_viewport.into();
    let map = |point: PointF| p_source.map_point(point, viewport, p_aspect_ratio);
    Path {
      start: map(self.start),
      segments: self
        .segments
        .iter()
        .map(|segment| match *segment {
          Segment::Line { to } => Segment::Line { to: map(to) },
          Segment::Quadratic { ctrl, to } => Segment::Quadratic {
            ctrl: map(ctrl),
            to: map(to),
          },
          Segment::Cubic { ctrl1, ctrl2, to } => Segment::Cubic {
            ctrl1: map(ctrl1),
            ctrl2: map(ctrl2),
            to: map(to),
          },
        })
        .collect(),
      angle_degrees: self.angle_degrees,
    }
  }

  /// Scales this path from its own bounds into a box of `p_size`.
  /// - `p_size`: The target size.
  /// - `p_aspect_ratio`: How to scale when the shapes differ: [`AspectRatio::meet`] fits inside,
  ///   [`AspectRatio::slice`] covers (may crop), and [`AspectRatio::none`] stretches to fill.
  pub fn fit(&self, p_size: impl Into<Size>, p_aspect_ratio: AspectRatio) -> Path {
    self.transform_to_viewport(&self.bounds(), p_size, p_aspect_ratio)
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
}

impl From<f32> for Path {
  /// Creates a unit direction line from an angle in degrees.
  ///
  /// Zero degrees points right, and positive angles rotate clockwise in the
  /// image coordinate system.
  fn from(p_angle_degrees: f32) -> Self {
    Path {
      start: PointF::zero(),
      segments: Vec::new(),
      angle_degrees: Some(p_angle_degrees),
    }
  }
}

impl From<i32> for Path {
  fn from(p_angle_degrees: i32) -> Self {
    Path::from(p_angle_degrees as f32)
  }
}

impl From<f64> for Path {
  fn from(p_angle_degrees: f64) -> Self {
    Path::from(p_angle_degrees as f32)
  }
}

impl Display for Path {
  /// Displays the path as a string.
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(p_f, "Path(start: {}, segments: {})", self.start, self.segments.len())
  }
}

impl Default for Path {
  fn default() -> Self {
    Path {
      start: PointF::zero(),
      segments: Vec::new(),
      angle_degrees: None,
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
      let ttt = p_t * p_t * p_t;
      p_prev * uuu + *ctrl1 * (3.0 * uu * p_t) + *ctrl2 * (3.0 * u * p_t * p_t) + *to * ttt
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn closest_time_line_is_normalized() {
    let path = Path::line((0.0, 0.0), (0.0, 100.0));
    // top
    assert!((path.closest_time(0.0, 0.0) - 0.0).abs() < 1e-6);
    // middle
    assert!((path.closest_time(0.0, 50.0) - 0.5).abs() < 1e-6);
    // bottom
    assert!((path.closest_time(0.0, 100.0) - 1.0).abs() < 1e-6);
  }
}

/// The number of straight pieces a curve with the given control polygon (start, controls, end) is flattened into,
/// estimated from its length so each piece is about `p_tolerance` long.
fn subdivisions(p_points: &[PointF], p_tolerance: f32) -> usize {
  let chord_len = p_points[0].distance_to(p_points[p_points.len() - 1]);
  let control_dist: f32 = p_points.windows(2).map(|pair| pair[0].distance_to(pair[1])).sum();
  let estimate_len = (chord_len + control_dist) * 0.5;
  (estimate_len / p_tolerance).ceil().max(2.0) as usize
}
