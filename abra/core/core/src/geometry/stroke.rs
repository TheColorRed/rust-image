//! Stroke expansion for paths and areas.

use crate::{Area, Path, PointF};
use std::f32::consts::PI;

/// Line cap styles for path endpoints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineCap {
  /// The line ends exactly at the endpoint.
  Butt,
  /// The line ends with a rounded cap.
  Round,
  /// The line ends with a square cap extending beyond the endpoint.
  Square,
}

/// Line join styles for corners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineJoin {
  /// The line joins with a mitered (pointed) corner.
  Miter,
  /// The line joins with a rounded corner.
  Round,
  /// The line joins with a beveled (cut-off) corner.
  Bevel,
}

/// How far a miter may reach, as a multiple of half the stroke width, before it is beveled instead.
const DEFAULT_MITER_LIMIT: f32 = 4.0;

/// A stroke of an open path that has been described but not yet built. Create one with [`Path::stroke`], adjust
/// it with the `with_*` methods, then build the outline with [`PathStroke::to_path`] or convert it with `into()`.
#[derive(Clone, Copy)]
pub struct PathStroke<'a> {
  path: &'a Path,
  width: f32,
  join: LineJoin,
  cap: LineCap,
  miter_limit: f32,
}

impl<'a> PathStroke<'a> {
  /// Sets how corners are drawn. Defaults to [`LineJoin::Miter`].
  pub fn with_join(mut self, p_join: LineJoin) -> Self {
    self.join = p_join;
    self
  }

  /// Sets how the two ends are drawn. Defaults to [`LineCap::Butt`].
  pub fn with_cap(mut self, p_cap: LineCap) -> Self {
    self.cap = p_cap;
    self
  }

  /// Sets how far a miter may reach, as a multiple of half the stroke width, before the corner is beveled instead.
  /// Keeps sharp corners from producing long spikes. Defaults to `4.0`.
  pub fn with_miter_limit(mut self, p_limit: f32) -> Self {
    self.miter_limit = p_limit.max(1.0);
    self
  }

  /// Builds the outline of the stroke as a closed path.
  pub fn to_path(&self) -> Path {
    let half_width = self.width / 2.0;
    // Use a lower flatten tolerance to produce smoother joins and arcs.
    let points = dedupe(self.path.flatten(0.125));
    if points.len() < 2 || half_width <= 0.0 {
      return Path::new();
    }
    let segments = Segments::new(&points, false);

    let left = offset_side(&points, &segments, false, 1.0, half_width, self.join, self.miter_limit);
    let right = offset_side(&points, &segments, false, -1.0, half_width, self.join, self.miter_limit);

    let first = points[0];
    let last = points[points.len() - 1];
    let start_normal = segments.normals[0];
    let end_normal = segments.normals[segments.normals.len() - 1];

    let mut outline = left;
    push_cap(&mut outline, last, end_normal, 1.0, half_width, self.cap);
    outline.extend(right.iter().rev());
    push_cap(&mut outline, first, start_normal, -1.0, half_width, self.cap);

    to_closed_path(&dedupe(outline))
  }
}

impl From<PathStroke<'_>> for Path {
  fn from(p_stroke: PathStroke<'_>) -> Self {
    p_stroke.to_path()
  }
}

impl From<PathStroke<'_>> for Area {
  fn from(p_stroke: PathStroke<'_>) -> Self {
    p_stroke.to_path().into()
  }
}

impl Path {
  /// Strokes this path as an open line. Set the join, cap, and miter limit on the result, then build it with
  /// [`PathStroke::to_path`] or `into()`.
  /// - `p_width`: The stroke width.
  pub fn stroke(&self, p_width: f32) -> PathStroke<'_> {
    PathStroke {
      path: self,
      width: p_width,
      join: LineJoin::Miter,
      cap: LineCap::Butt,
      miter_limit: DEFAULT_MITER_LIMIT,
    }
  }
}

/// A stroke along the boundary of an area that has been described but not yet built. Create one with
/// [`Area::stroke`], adjust it with the `with_*` methods, then build it with [`AreaStroke::to_area`] or `into()`.
#[derive(Clone, Copy)]
pub struct AreaStroke<'a> {
  area: &'a Area,
  width: f32,
  join: LineJoin,
  miter_limit: f32,
}

impl<'a> AreaStroke<'a> {
  /// Sets how corners are drawn. Defaults to [`LineJoin::Miter`].
  pub fn with_join(mut self, p_join: LineJoin) -> Self {
    self.join = p_join;
    self
  }

  /// Sets how far a miter may reach, as a multiple of half the stroke width, before the corner is beveled instead.
  /// Keeps sharp corners from producing long spikes. Defaults to `4.0`.
  pub fn with_miter_limit(mut self, p_limit: f32) -> Self {
    self.miter_limit = p_limit.max(1.0);
    self
  }

  /// Builds the stroke as a ring-shaped area centered on the boundary.
  pub fn to_area(&self) -> Area {
    let half_width = self.width / 2.0;
    let mut points = dedupe(self.area.path.flatten(0.5));
    // The boundary is closed, so a repeated start point would only add a zero-length segment.
    if points.len() > 1 && near(points[0], points[points.len() - 1]) {
      points.pop();
    }
    if points.len() < 3 || half_width <= 0.0 {
      return Area::new();
    }
    let segments = Segments::new(&points, true);

    let outer = offset_side(&points, &segments, true, 1.0, half_width, self.join, self.miter_limit);
    let inner = offset_side(&points, &segments, true, -1.0, half_width, self.join, self.miter_limit);

    // Trace one ring forwards and the other backwards so they wind in opposite directions and the middle stays
    // empty. A seam joins the two rings; it is crossed once each way, so it adds nothing to the shape.
    let mut outline = outer.clone();
    outline.push(outer[0]);
    outline.push(inner[0]);
    outline.extend(inner.iter().skip(1).rev());
    outline.push(inner[0]);
    to_closed_path(&outline).into()
  }
}

impl From<AreaStroke<'_>> for Area {
  fn from(p_stroke: AreaStroke<'_>) -> Self {
    p_stroke.to_area()
  }
}

impl Area {
  /// Strokes the boundary of this area. Set the join and miter limit on the result, then build it with
  /// [`AreaStroke::to_area`] or `into()`.
  /// - `p_width`: The stroke width.
  pub fn stroke(&self, p_width: f32) -> AreaStroke<'_> {
    AreaStroke {
      area: self,
      width: p_width,
      join: LineJoin::Miter,
      miter_limit: DEFAULT_MITER_LIMIT,
    }
  }
}

/// The unit normal and length of each segment of a polyline.
struct Segments {
  normals: Vec<(f32, f32)>,
  lengths: Vec<f32>,
}

impl Segments {
  /// When `p_closed` is set, the last segment runs from the last point back to the first.
  fn new(p_points: &[PointF], p_closed: bool) -> Self {
    let count = if p_closed { p_points.len() } else { p_points.len() - 1 };
    let (normals, lengths) = (0..count)
      .map(|i| {
        let (from, to) = (p_points[i], p_points[(i + 1) % p_points.len()]);
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let length = (dx * dx + dy * dy).sqrt().max(1e-6);
        ((-dy / length, dx / length), length)
      })
      .unzip();
    Segments { normals, lengths }
  }
}

/// Offsets a polyline to one side by half the stroke width, adding joins at the corners.
/// - `p_side`: `1.0` for the side the normals point to, `-1.0` for the other.
fn offset_side(
  p_points: &[PointF], p_segments: &Segments, p_closed: bool, p_side: f32, p_half_width: f32, p_join: LineJoin,
  p_miter_limit: f32,
) -> Vec<PointF> {
  let normals = &p_segments.normals;
  let lengths = &p_segments.lengths;
  let offset = |point: PointF, normal: (f32, f32)| {
    PointF::new(point.x + normal.0 * p_side * p_half_width, point.y + normal.1 * p_side * p_half_width)
  };

  let mut out = Vec::with_capacity(p_points.len() * 2);
  let count = p_points.len();
  for i in 0..count {
    let corner = if p_closed {
      let previous = (i + count - 1) % count;
      Some((previous, i))
    } else if i > 0 && i < count - 1 {
      Some((i - 1, i))
    } else {
      None
    };

    match corner {
      Some((previous, next)) => {
        let join = Join {
          center: p_points[i],
          previous_normal: normals[previous],
          next_normal: normals[next],
          previous_length: lengths[previous],
          next_length: lengths[next],
          side: p_side,
          half_width: p_half_width,
        };
        join.push(&mut out, p_join, p_miter_limit);
      }
      // The two ends of an open line have only one segment each.
      None => out.push(offset(p_points[i], normals[i.min(normals.len() - 1)])),
    }
  }
  out
}

/// A corner between two segments, seen from one side of the stroke.
struct Join {
  center: PointF,
  previous_normal: (f32, f32),
  next_normal: (f32, f32),
  previous_length: f32,
  next_length: f32,
  side: f32,
  half_width: f32,
}

impl Join {
  fn push(&self, p_out: &mut Vec<PointF>, p_join: LineJoin, p_miter_limit: f32) {
    let (n1, n2) = (self.previous_normal, self.next_normal);
    let (s, hw, c) = (self.side, self.half_width, self.center);
    let from = PointF::new(c.x + n1.0 * s * hw, c.y + n1.1 * s * hw);
    let to = PointF::new(c.x + n2.0 * s * hw, c.y + n2.1 * s * hw);
    let dot = n1.0 * n2.0 + n1.1 * n2.1;
    let cross = n1.0 * n2.1 - n1.1 * n2.0;

    // Straight on: nothing to join.
    if dot > 0.9999 {
      p_out.push(from);
      return;
    }

    // Where the two offset edges meet. On the inside of the turn this is the inner corner; on the outside it is
    // the tip of the miter. Its distance from the center is `hw * sqrt(2 / (1 + dot))`.
    let denominator = 1.0 + dot;
    let meeting = (denominator > 1e-6).then(|| {
      let scale = s * hw / denominator;
      PointF::new(c.x + (n1.0 + n2.0) * scale, c.y + (n1.1 + n2.1) * scale)
    });

    // The side the path turns away from is the outside of the corner. A full reversal has no inside.
    let is_outside = s * cross < 0.0 || cross.abs() < 1e-6;
    if !is_outside {
      // Use the inner corner when it lies within both segments; otherwise the segments are too short for the
      // offset edges to meet, and the outline doubles back on itself, which the non-zero fill still covers.
      let reach = hw * ((1.0 - dot) / denominator.max(1e-6)).sqrt();
      match meeting {
        Some(point) if reach <= self.previous_length.min(self.next_length) => p_out.push(point),
        _ => p_out.extend([from, to]),
      }
      return;
    }

    match p_join {
      LineJoin::Miter => match meeting {
        Some(point) if (2.0 / denominator).sqrt() <= p_miter_limit => p_out.push(point),
        _ => p_out.extend([from, to]),
      },
      LineJoin::Bevel => p_out.extend([from, to]),
      LineJoin::Round => {
        let start = (n1.1 * s).atan2(n1.0 * s);
        let end = (n2.1 * s).atan2(n2.0 * s);
        // A full reversal turns around the front of the line.
        let sweep = if cross.abs() < 1e-6 { -s * PI } else { normalize_angle(end - start) };
        p_out.push(from);
        push_arc(p_out, c, hw, start, sweep);
      }
    }
  }
}

/// Adds the cap at one end of an open line, going from the left offset to the right offset.
/// - `p_normal`: The normal of the segment at that end.
/// - `p_direction`: `1.0` at the end of the line, `-1.0` at the start.
fn push_cap(
  p_out: &mut Vec<PointF>, p_point: PointF, p_normal: (f32, f32), p_direction: f32, p_half_width: f32, p_cap: LineCap,
) {
  // At the end the outline arrives on the left and leaves on the right; at the start it is the other way round.
  let (nx, ny) = (p_normal.0 * p_direction, p_normal.1 * p_direction);
  // The direction the line continues in past this end.
  let (ux, uy) = (ny, -nx);
  match p_cap {
    LineCap::Butt => {}
    LineCap::Square => {
      p_out.push(PointF::new(p_point.x + (nx + ux) * p_half_width, p_point.y + (ny + uy) * p_half_width));
      p_out.push(PointF::new(p_point.x + (-nx + ux) * p_half_width, p_point.y + (-ny + uy) * p_half_width));
    }
    LineCap::Round => push_arc(p_out, p_point, p_half_width, ny.atan2(nx), -PI),
  }
}

/// Adds points along a circular arc, leaving out the start point.
fn push_arc(p_out: &mut Vec<PointF>, p_center: PointF, p_radius: f32, p_start: f32, p_sweep: f32) {
  // Keep each chord within a tenth of a pixel of the true arc.
  let tolerance = 0.1;
  let step = if p_radius > tolerance { 2.0 * (1.0 - tolerance / p_radius).acos() } else { PI / 2.0 };
  let steps = ((p_sweep.abs() / step).ceil() as usize).clamp(1, 256);
  for k in 1..=steps {
    let angle = p_start + p_sweep * k as f32 / steps as f32;
    p_out.push(PointF::new(p_center.x + angle.cos() * p_radius, p_center.y + angle.sin() * p_radius));
  }
}

/// Wraps an angle into `-PI..=PI`.
fn normalize_angle(p_angle: f32) -> f32 {
  let mut angle = p_angle;
  while angle <= -PI {
    angle += 2.0 * PI;
  }
  while angle > PI {
    angle -= 2.0 * PI;
  }
  angle
}

fn near(p_a: PointF, p_b: PointF) -> bool {
  (p_a.x - p_b.x).abs() < 1e-4 && (p_a.y - p_b.y).abs() < 1e-4
}

/// Removes consecutive repeated points, which have no direction to offset along.
fn dedupe(p_points: Vec<PointF>) -> Vec<PointF> {
  let mut out: Vec<PointF> = Vec::with_capacity(p_points.len());
  for point in p_points {
    if out.last().map_or(true, |last| !near(*last, point)) {
      out.push(point);
    }
  }
  out
}

fn to_closed_path(p_points: &[PointF]) -> Path {
  let mut path = Path::new();
  if let Some((first, rest)) = p_points.split_first() {
    path.move_to(*first);
    for point in rest {
      path.line_to(*point);
    }
    path.line_to(*first);
  }
  path
}

#[cfg(test)]
mod tests {
  use super::*;

  fn polyline(p_points: &[(f32, f32)]) -> Path {
    let mut path = Path::new();
    path.move_to(p_points[0]);
    for point in &p_points[1..] {
      path.line_to(*point);
    }
    path
  }

  fn assert_bounds(p_actual: (f32, f32, f32, f32), p_expected: (f32, f32, f32, f32)) {
    // Arcs are built to within 0.1px of the true curve.
    let close = |a: f32, b: f32| (a - b).abs() <= 0.1;
    assert!(
      close(p_actual.0, p_expected.0)
        && close(p_actual.1, p_expected.1)
        && close(p_actual.2, p_expected.2)
        && close(p_actual.3, p_expected.3),
      "{p_actual:?} != {p_expected:?}"
    );
  }

  #[test]
  fn caps_set_how_far_the_line_reaches() {
    let line = polyline(&[(0.0, 0.0), (10.0, 0.0)]);
    assert_bounds(line.stroke(4.0).to_path().bounds(), (0.0, -2.0, 10.0, 2.0));
    assert_bounds(line.stroke(4.0).with_cap(LineCap::Square).to_path().bounds(), (-2.0, -2.0, 12.0, 2.0));
    assert_bounds(line.stroke(4.0).with_cap(LineCap::Round).to_path().bounds(), (-2.0, -2.0, 12.0, 2.0));
  }

  #[test]
  fn square_cap_is_flat() {
    let line = polyline(&[(0.0, 0.0), (10.0, 0.0)]);
    let area: Area = line.stroke(4.0).with_cap(LineCap::Square).into();
    // The corners of a square cap are filled; a pointed cap would leave them empty.
    assert!(area.contains((11.8, 1.8)));
    assert!(area.contains((-1.8, -1.8)));
  }

  #[test]
  fn joins_shape_the_outside_corner() {
    let corner = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
    let miter: Area = corner.stroke(4.0).with_join(LineJoin::Miter).into();
    let bevel: Area = corner.stroke(4.0).with_join(LineJoin::Bevel).into();
    let round: Area = corner.stroke(4.0).with_join(LineJoin::Round).into();

    // The miter reaches the corner of the offset edges at (12, -2).
    assert!(miter.contains((11.8, -1.8)));
    assert!(!bevel.contains((11.8, -1.8)));
    assert!(!round.contains((11.8, -1.8)));
    // The round join reaches further than the bevel along the diagonal.
    assert!(round.contains((11.3, -1.3)));
    assert!(!bevel.contains((11.3, -1.3)));
    // The inside of the corner is covered by all of them.
    for area in [&miter, &bevel, &round] {
      assert!(area.contains((8.5, 1.5)));
    }
  }

  #[test]
  fn sharp_miters_are_beveled_past_the_limit() {
    let spike = polyline(&[(0.0, 0.0), (10.0, 0.0), (0.0, 1.0)]);
    let (_, _, max_x, _) = spike.stroke(2.0).to_path().bounds();
    assert!(max_x < 12.0, "miter reached {max_x}");
    let (_, _, max_x, _) = spike.stroke(2.0).with_miter_limit(100.0).to_path().bounds();
    assert!(max_x > 20.0, "miter reached {max_x}");
  }

  #[test]
  fn area_stroke_is_a_ring() {
    let square = Area::rect((0.0, 0.0), (10.0, 10.0));
    let ring = square.stroke(2.0).to_area();
    assert_bounds(ring.bounds::<f32>(), (-1.0, -1.0, 11.0, 11.0));
    assert!(ring.contains((0.0, 5.0)));
    assert!(ring.contains((10.0, 5.0)));
    assert!(!ring.contains((5.0, 5.0)));
    // Every side is covered, including the one that closes the boundary.
    assert!(ring.contains((5.0, 0.0)));
    assert!(ring.contains((5.0, 10.0)));
  }

  #[test]
  fn area_stroke_joins() {
    let square = Area::rect((0.0, 0.0), (10.0, 10.0));
    let miter = square.stroke(2.0).to_area();
    let round = square.stroke(2.0).with_join(LineJoin::Round).to_area();
    assert!(miter.contains((-0.9, -0.9)));
    assert!(!round.contains((-0.9, -0.9)));
  }
}
