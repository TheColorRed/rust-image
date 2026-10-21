//! Turning a traced outline into a clean shape: dropping the points it does not need, and joining the rest with smooth curves.
//!
//! An outline found on a grid of pixels (see `Mask::contours`) has a point for every step of the grid, so a straight edge is a staircase.
//! [`simplify_points`] keeps only the points that matter, and [`smoothed_area`] draws curves through them that are smooth at every point but
//! the real corners.

use super::{Area, PointF};

/// How far a point is from the line through two points, or from the first one when they are the same point.
fn distance_to_line(p_point: PointF, p_from: PointF, p_to: PointF) -> f32 {
  let (dx, dy) = (p_to.x - p_from.x, p_to.y - p_from.y);
  let length = (dx * dx + dy * dy).sqrt();
  if length == 0.0 {
    return ((p_point.x - p_from.x).powi(2) + (p_point.y - p_from.y).powi(2)).sqrt();
  }
  ((p_point.x - p_from.x) * dy - (p_point.y - p_from.y) * dx).abs() / length
}

/// The points to keep of a run of points that goes from its first to its last (Ramer-Douglas-Peucker): the end points, and then, as far as
/// a point is more than `p_tolerance` from the line between the ones kept around it, that point too.
fn simplify_run(p_points: &[PointF], p_tolerance: f32) -> Vec<PointF> {
  if p_points.len() < 3 {
    return p_points.to_vec();
  }
  let mut keep = vec![false; p_points.len()];
  keep[0] = true;
  keep[p_points.len() - 1] = true;
  // The runs still to look at, as the places of their end points. A stack and not a call to itself, since a long outline is deep.
  let mut stack = vec![(0, p_points.len() - 1)];
  while let Some((first, last)) = stack.pop() {
    let mut farthest = (0.0, first);
    for at in first + 1..last {
      let distance = distance_to_line(p_points[at], p_points[first], p_points[last]);
      if distance > farthest.0 {
        farthest = (distance, at);
      }
    }
    if farthest.0 > p_tolerance {
      keep[farthest.1] = true;
      stack.push((first, farthest.1));
      stack.push((farthest.1, last));
    }
  }
  p_points.iter().zip(keep).filter_map(|(point, keep)| keep.then_some(*point)).collect()
}

/// Drops the points of an outline that the shape does not need, so that no point of the outline is more than `p_tolerance` from the line
/// that replaces it. A staircase along a diagonal becomes a line, and a straight edge becomes its two end points. It keeps the first
/// and last point of an open outline, and the shape of a closed one.
/// - `p_points`: The points of the outline, in order.
/// - `p_tolerance`: How far, in the units of the points, the result may be from the outline. `0` drops only the points on a straight line.
/// - `p_closed`: Whether the last point joins the first (and is not repeated), as the outlines of `Mask::contours` do.
pub fn simplify_points(p_points: &[PointF], p_tolerance: f32, p_closed: bool) -> Vec<PointF> {
  if p_points.len() < 3 {
    return p_points.to_vec();
  }
  let tolerance = p_tolerance.max(0.0);
  if !p_closed {
    return simplify_run(p_points, tolerance);
  }
  // A closed outline has no ends, so it is cut in two where it is widest: from its first point to the one farthest from it, and back.
  let first = p_points[0];
  let split = (1..p_points.len())
    .max_by(|a, b| {
      let (da, db) = (distance_to_line(p_points[*a], first, first), distance_to_line(p_points[*b], first, first));
      da.total_cmp(&db)
    })
    .unwrap_or(1);
  let mut out = simplify_run(&p_points[..=split], tolerance);
  let mut back: Vec<PointF> = p_points[split..].to_vec();
  back.push(first);
  let back = simplify_run(&back, tolerance);
  // Each half ends on the point the other one starts at, so those are left out of the second half and the closing point is not repeated.
  out.extend(&back[1..back.len() - 1]);
  out
}

/// The angle, in degrees from `0` for straight on to `180` for turning right back, that an outline turns at a point.
fn turn_degrees(p_before: PointF, p_at: PointF, p_after: PointF) -> f32 {
  let (ax, ay) = (p_at.x - p_before.x, p_at.y - p_before.y);
  let (bx, by) = (p_after.x - p_at.x, p_after.y - p_at.y);
  let (la, lb) = ((ax * ax + ay * ay).sqrt(), (bx * bx + by * by).sqrt());
  if la == 0.0 || lb == 0.0 {
    return 0.0;
  }
  ((ax * bx + ay * by) / (la * lb)).clamp(-1.0, 1.0).acos().to_degrees()
}

/// A closed shape through the points, as curves that are smooth at every point, except where the outline turns by more than
/// `p_corner_degrees`: that point is a corner, and the curves come into it straight. A long straight edge stays straight, since the
/// curve through a point leaves it along the line from the point before to the point after, and a short step of curve is shaped by its
/// own length so that uneven points do not make bulges.
///
/// Simplify a traced outline first (see [`simplify_points`]), so the curves go through the points that matter and not through every step
/// of the pixel grid.
/// - `p_points`: The points of the shape, in order, the last joining the first and not repeated.
/// - `p_corner_degrees`: How sharp a turn has to be to stay a corner, from `0` (every point is a corner, so the shape is straight lines)
///   to `180` (none is).
pub fn smoothed_area(p_points: &[PointF], p_corner_degrees: f32) -> Area {
  let mut area = Area::new();
  let count = p_points.len();
  if count < 3 {
    if let Some(first) = p_points.first() {
      area.move_to(*first);
      for point in &p_points[1..] {
        area.line_to(*point);
      }
    }
    return area;
  }
  let point = |at: usize| p_points[at % count];
  // The way the curve goes through each point: along the line from the point before to the point after, or nowhere at a corner.
  let tangents: Vec<(f32, f32)> = (0..count)
    .map(|at| {
      let (before, here, after) = (point(at + count - 1), point(at), point(at + 1));
      if turn_degrees(before, here, after) > p_corner_degrees {
        return (0.0, 0.0);
      }
      let (dx, dy) = (after.x - before.x, after.y - before.y);
      let length = (dx * dx + dy * dy).sqrt();
      if length == 0.0 { (0.0, 0.0) } else { (dx / length, dy / length) }
    })
    .collect();
  area.move_to(point(0));
  for at in 0..count {
    let (from, to) = (point(at), point(at + 1));
    let reach = ((to.x - from.x).powi(2) + (to.y - from.y).powi(2)).sqrt() / 3.0;
    let (leave, arrive) = (tangents[at], tangents[(at + 1) % count]);
    area.cubic_to(
      PointF { x: from.x + leave.0 * reach, y: from.y + leave.1 * reach },
      PointF { x: to.x - arrive.0 * reach, y: to.y - arrive.1 * reach },
      to,
    );
  }
  area
}

#[cfg(test)]
mod tests {
  use super::*;

  fn points(p_pairs: &[(f32, f32)]) -> Vec<PointF> {
    p_pairs.iter().map(|(x, y)| PointF { x: *x, y: *y }).collect()
  }

  #[test]
  fn points_on_a_straight_line_are_dropped() {
    let line = points(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]);
    assert_eq!(simplify_points(&line, 0.0, false), points(&[(0.0, 0.0), (3.0, 0.0)]));
  }

  #[test]
  fn a_staircase_within_the_tolerance_becomes_a_line() {
    let stairs = points(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (2.0, 1.0), (2.0, 2.0), (3.0, 2.0), (3.0, 3.0)]);
    assert_eq!(simplify_points(&stairs, 0.8, false), points(&[(0.0, 0.0), (3.0, 3.0)]));
    // A tolerance that is too small to cut the steps keeps them.
    assert!(simplify_points(&stairs, 0.1, false).len() > 3);
  }

  #[test]
  fn a_point_farther_than_the_tolerance_is_kept() {
    let bump = points(&[(0.0, 0.0), (5.0, 3.0), (10.0, 0.0)]);
    assert_eq!(simplify_points(&bump, 2.0, false).len(), 3);
    assert_eq!(simplify_points(&bump, 4.0, false).len(), 2);
  }

  #[test]
  fn a_closed_outline_keeps_its_corners_and_is_not_closed_twice() {
    // A square with a point in the middle of each side.
    let square = points(&[(0.0, 0.0), (5.0, 0.0), (10.0, 0.0), (10.0, 5.0), (10.0, 10.0), (5.0, 10.0), (0.0, 10.0), (0.0, 5.0)]);
    let simple = simplify_points(&square, 0.1, true);
    assert_eq!(simple.len(), 4, "{simple:?}");
    for corner in points(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]) {
      assert!(simple.contains(&corner), "{simple:?}");
    }
  }

  #[test]
  fn short_outlines_are_left_as_they_are() {
    let two = points(&[(0.0, 0.0), (1.0, 1.0)]);
    assert_eq!(simplify_points(&two, 5.0, true), two);
    assert!(simplify_points(&[], 5.0, true).is_empty());
  }

  /// A polygon of `p_count` points on a circle of radius 100 around (100, 100).
  fn circle(p_count: usize) -> Vec<PointF> {
    (0..p_count)
      .map(|at| {
        let angle = at as f32 / p_count as f32 * std::f32::consts::TAU;
        PointF { x: 100.0 + 100.0 * angle.cos(), y: 100.0 + 100.0 * angle.sin() }
      })
      .collect()
  }

  #[test]
  fn a_smoothed_polygon_of_a_circle_stays_on_the_circle() {
    let area = smoothed_area(&circle(12), 60.0);
    for point in area.flatten(0.1) {
      let radius = ((point.x - 100.0).powi(2) + (point.y - 100.0).powi(2)).sqrt();
      assert!((radius - 100.0).abs() < 1.0, "{point:?} {radius}");
    }
  }

  #[test]
  fn a_smoothed_square_keeps_its_corners_and_straight_sides() {
    let square = points(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
    let area = smoothed_area(&square, 60.0);
    // Each corner turns by 90 degrees, more than the limit, so the shape is the square itself.
    for point in area.flatten(0.05) {
      assert!(point.x >= -0.001 && point.x <= 10.001 && point.y >= -0.001 && point.y <= 10.001, "{point:?}");
      assert!(point.x.min(10.0 - point.x).min(point.y).min(10.0 - point.y) < 0.001, "{point:?} is not on a side");
    }
    assert!(area.flatten(0.05).iter().any(|point| (point.x - 10.0).abs() < 0.001 && (point.y - 10.0).abs() < 0.001));
  }

  #[test]
  fn a_square_is_rounded_when_no_turn_counts_as_a_corner() {
    let square = points(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
    let area = smoothed_area(&square, 180.0);
    // The curve still goes through the corner but leaves it on the diagonal, so near it there are points off both sides of the square.
    assert!(area.flatten(0.05).iter().any(|point| {
      let (off_x, off_y) = ((point.x - 10.0).abs(), (point.y - 10.0).abs());
      off_x > 0.05 && off_y > 0.05 && off_x < 2.0 && off_y < 2.0
    }));
  }

  #[test]
  fn fewer_than_three_points_are_not_curved() {
    let area = smoothed_area(&points(&[(0.0, 0.0), (4.0, 4.0)]), 60.0);
    assert_eq!(area.flatten(0.1), points(&[(0.0, 0.0), (4.0, 4.0)]));
  }
}
