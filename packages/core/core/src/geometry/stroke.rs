//! Stroke expansion for paths and areas.

// use crate::utils::debug::DebugGeometry;
use crate::{Area, Path, PointF};
use std::time::Instant;

/// Line cap styles for path endpoints.
#[derive(Clone, Debug)]
pub enum LineCap {
  /// The line ends exactly at the endpoint.
  Butt,
  /// The line ends with a rounded cap.
  Round,
  /// The line ends with a square cap extending beyond the endpoint.
  Square,
}

/// Line join styles for corners.
#[derive(Clone, Debug)]
pub enum LineJoin {
  /// The line joins with a mitered (pointed) corner.
  Miter,
  /// The line joins with a rounded corner.
  Round,
  /// The line joins with a beveled (cut-off) corner.
  Bevel,
}

impl Path {
  /// Creates a stroked outline path from this path.
  /// Returns an open path representing the stroke outline.
  /// - `width`: The stroke width.
  /// - `join`: How corners are drawn.
  /// - `cap`: How endpoints are drawn.
  pub fn stroke(&self, p_width: f32, _p_join: LineJoin, p_cap: LineCap) -> Path {
    use std::f32::consts::PI;

    let _start = Instant::now();
    let half_width = p_width / 2.0;
    // Use a lower flatten tolerance to produce smoother joins and arcs.
    // More points reduce polygon stair-casing when approximating curves.
    let flattened = self.flatten(0.125);

    if flattened.len() < 2 {
      return Path::new();
    }

    // Endpoints and tangents
    let start_p1 = flattened[0];
    let start_p2 = flattened[1];
    let end_p1 = flattened[flattened.len() - 2];
    let end_p2 = flattened[flattened.len() - 1];
    let start_dx = start_p2.x - start_p1.x;
    let start_dy = start_p2.y - start_p1.y;
    let end_dx = end_p2.x - end_p1.x;
    let end_dy = end_p2.y - end_p1.y;

    // Build per-segment normals and per-vertex offsets so we can construct proper joins.
    let seg_count = flattened.len() - 1;
    let mut normals: Vec<(f32, f32)> = Vec::with_capacity(seg_count);
    for i in 0..seg_count {
      let p1 = flattened[i];
      let p2 = flattened[i + 1];
      let dx = p2.x - p1.x;
      let dy = p2.y - p1.y;
      let len = (dx * dx + dy * dy).sqrt().max(1e-6);
      let nx = -dy / len;
      let ny = dx / len;
      normals.push((nx, ny));
    }

    // Per-vertex offset points allowing for join arc generation.
    let mut left_points: Vec<PointF> = Vec::with_capacity(flattened.len());
    let mut right_points: Vec<PointF> = Vec::with_capacity(flattened.len());
    for i in 0..flattened.len() {
      let p = flattened[i];
      // Choose normal for vertex: if interior (has both adjacent normals) we'll compute offsets per adjacent normals when needed.
      // For the purpose of an initial offset for the sequence, prefer the next segment's normal if available, otherwise prev
      let (nx, ny) = if i < seg_count { normals[i] } else { normals[i - 1] };
      left_points.push(PointF::new(p.x + nx * half_width, p.y + ny * half_width));
      right_points.push(PointF::new(p.x - nx * half_width, p.y - ny * half_width));
    }

    // Helper: circular arc between offset points around same center.
    let build_round_cap_arc =
      |center: PointF, start_pt: PointF, end_pt: PointF, steps: usize, prefer_dir: Option<(f32, f32)>| {
        let mut pts = Vec::new();
        let start_ang = (start_pt.y - center.y).atan2(start_pt.x - center.x);
        let end_ang = (end_pt.y - center.y).atan2(end_pt.x - center.x);
        let mut diff = end_ang - start_ang;
        while diff <= -PI {
          diff += 2.0 * PI;
        }
        while diff > PI {
          diff -= 2.0 * PI;
        }
        if diff.abs() < 1e-6 {
          diff = PI;
        }
        // If a preferred outward direction is provided, pick arc direction (sign of diff)
        // such that the midpoint of arc faces the preferred direction.
        if let Some((px, py)) = prefer_dir {
          let mid_ang = start_ang + diff / 2.0;
          let pref_ang = py.atan2(px);
          let mut ang_diff = mid_ang - pref_ang;
          while ang_diff <= -PI {
            ang_diff += 2.0 * PI;
          }
          while ang_diff > PI {
            ang_diff -= 2.0 * PI;
          }
          if ang_diff.abs() > PI / 2.0 {
            // flip direction to face preferred direction
            if diff > 0.0 {
              diff -= 2.0 * PI;
            } else {
              diff += 2.0 * PI;
            }
          }
        }
        let abs_diff = diff.abs();
        let step_angle = std::f32::consts::PI / 60.0; // ~3 degrees
        let computed_steps = (abs_diff / step_angle).ceil() as usize;
        let mut final_steps = std::cmp::max(6, std::cmp::min(computed_steps, 128));
        if steps > 0 {
          final_steps = steps
        }
        let inc = diff / final_steps as f32;
        for i in 0..=final_steps {
          let a = start_ang + inc * i as f32;
          pts.push(PointF::new(center.x + a.cos() * half_width, center.y + a.sin() * half_width));
        }
        pts
      };

    let mut stroke_path = Path::new();
    if left_points.is_empty() || right_points.is_empty() {
      return stroke_path;
    }

    // Left edge forward (handle round joins between segments)
    stroke_path.move_to(left_points[0]);
    for i in 1..left_points.len() {
      let pt = left_points[i];
      // If there is an interior vertex (i < len - 1) and requested join is Round, insert an arc between
      // the left offset computed from the previous segment and the offset from the next segment.
      if i < flattened.len() - 1 {
        if let LineJoin::Round = _p_join {
          // compute exact offsets using adjacent normals
          let prev_n = normals[i - 1];
          let next_n = normals[i];
          let center = flattened[i];
          let prev_offset = PointF::new(center.x + prev_n.0 * half_width, center.y + prev_n.1 * half_width);
          let next_offset = PointF::new(center.x + next_n.0 * half_width, center.y + next_n.1 * half_width);
          // Determine concavity by cross product of adjacent segment vectors
          let seg_vec_prev = PointF::new(flattened[i].x - flattened[i - 1].x, flattened[i].y - flattened[i - 1].y);
          let seg_vec_next = PointF::new(flattened[i + 1].x - flattened[i].x, flattened[i + 1].y - flattened[i].y);
          let cross = seg_vec_prev.x * seg_vec_next.y - seg_vec_prev.y * seg_vec_next.x;
          // If corner is concave for this side, don't add outward left arc — it's an inner join
          if cross <= 0.0 {
            stroke_path.line_to(pt);
            continue;
          }
          // prefer outward vector as bisector of normals
          let prefer_x = prev_n.0 + next_n.0;
          let prefer_y = prev_n.1 + next_n.1;
          let arc_pts = build_round_cap_arc(center.into(), prev_offset, next_offset, 12, Some((prefer_x, prefer_y)));
          // add arc points (skip duplicated start)
          for arc_pt in arc_pts.iter().skip(1) {
            stroke_path.line_to(*arc_pt);
          }
          continue;
        }
      }
      stroke_path.line_to(pt);
    }

    // End cap
    match p_cap {
      LineCap::Round => {
        let end_left = *left_points.last().unwrap();
        let end_right = *right_points.last().unwrap();
        let seg_len = (end_dx * end_dx + end_dy * end_dy).sqrt().max(1e-6);
        let ux = end_dx / seg_len;
        let uy = end_dy / seg_len;
        let cap_pts = build_round_cap_arc(end_p2.into(), end_left, end_right, 12, Some((ux, uy)));
        for pt in cap_pts.iter().skip(1) {
          stroke_path.line_to(*pt);
        }
      }
      LineCap::Square => {
        let seg_len = (end_dx * end_dx + end_dy * end_dy).sqrt().max(1e-6);
        let ux = end_dx / seg_len;
        let uy = end_dy / seg_len;
        stroke_path.line_to(PointF::new(end_p2.x + ux * half_width, end_p2.y + uy * half_width));
      }
      LineCap::Butt => {}
    }

    // Right edge back (handle round joins similarly, iterating reversed)
    for idx in (0..right_points.len()).rev() {
      // Skip last end point if round cap was emitted already
      if let LineCap::Round = p_cap {
        if idx == right_points.len() - 1 {
          continue;
        }
      }
      // For interior join add right-side round arc
      if idx > 0 && idx < flattened.len() - 1 {
        if let LineJoin::Round = _p_join {
          let center = flattened[idx];
          let prev_n = normals[idx - 1];
          let next_n = normals[idx];
          let prev_offset = PointF::new(center.x - prev_n.0 * half_width, center.y - prev_n.1 * half_width);
          let next_offset = PointF::new(center.x - next_n.0 * half_width, center.y - next_n.1 * half_width);
          // Prefer outward direction (negative bisector)
          let prefer_x = -(prev_n.0 + next_n.0);
          let prefer_y = -(prev_n.1 + next_n.1);
          let arc_pts = build_round_cap_arc(center.into(), prev_offset, next_offset, 12, Some((prefer_x, prefer_y)));
          // add arc points in reverse order (skip duplicated start)
          for arc_pt in arc_pts.iter().rev().skip(1) {
            stroke_path.line_to(*arc_pt);
          }
          continue;
        }
      }
      let pt = right_points[idx];
      stroke_path.line_to(pt);
    }

    // Start cap
    match p_cap {
      LineCap::Round => {
        let start_right = right_points[0];
        let start_left = left_points[0];
        let s_len = (start_dx * start_dx + start_dy * start_dy).sqrt().max(1e-6);
        let ux = -start_dx / s_len;
        let uy = -start_dy / s_len; // prefer outward = back along the line
        let cap_pts = build_round_cap_arc(start_p1.into(), start_right, start_left, 12, Some((ux, uy)));
        for pt in cap_pts.iter().skip(1) {
          stroke_path.line_to(*pt);
        }
      }
      LineCap::Square => {
        let seg_len = (start_dx * start_dx + start_dy * start_dy).sqrt().max(1e-6);
        let ux = start_dx / seg_len;
        let uy = start_dy / seg_len;
        stroke_path.line_to(PointF::new(start_p1.x - ux * half_width, start_p1.y - uy * half_width));
      }
      LineCap::Butt => {}
    }

    stroke_path.line_to(left_points[0]);
    // DebugGeometry::Stroke(self.clone(), p_width, start.elapsed()).log();
    stroke_path
  }
}

impl Area {
  /// Creates a stroked outline area from this closed area's boundary.
  /// The resulting area is a closed shape representing the stroke.
  /// - `width`: The stroke width.
  /// - `join`: How corners are drawn.
  pub fn stroke(&self, p_width: f32, _p_join: LineJoin) -> Area {
    let _ = &_p_join;
    let half_width = p_width / 2.0;
    let flattened = self.path.flatten(0.5);

    if flattened.len() < 3 {
      return Area::new();
    }

    let mut outer_points = Vec::new();
    let mut inner_points = Vec::new();
    let n = flattened.len();
    for i in 0..n {
      let p_prev = flattened[(i + n - 1) % n];
      let p_curr = flattened[i];
      let p_next = flattened[(i + 1) % n];

      let dx1 = p_curr.x - p_prev.x;
      let dy1 = p_curr.y - p_prev.y;
      let len1 = (dx1 * dx1 + dy1 * dy1).sqrt().max(0.001);
      let n1x = -dy1 / len1;
      let n1y = dx1 / len1;

      let dx2 = p_next.x - p_curr.x;
      let dy2 = p_next.y - p_curr.y;
      let len2 = (dx2 * dx2 + dy2 * dy2).sqrt().max(0.001);
      let n2x = -dy2 / len2;
      let n2y = dx2 / len2;

      let mut nx = (n1x + n2x) / 2.0;
      let mut ny = (n1y + n2y) / 2.0;
      let nlen = (nx * nx + ny * ny).sqrt().max(0.001);
      nx = nx / nlen;
      ny = ny / nlen;

      outer_points.push(PointF::new(p_curr.x + nx * half_width, p_curr.y + ny * half_width));
      inner_points.push(PointF::new(p_curr.x - nx * half_width, p_curr.y - ny * half_width));
    }

    let mut stroke_path = Path::new();
    if !outer_points.is_empty() {
      stroke_path.move_to(outer_points[0]);
      for pt in outer_points.iter().skip(1) {
        stroke_path.line_to(*pt);
      }
      for pt in inner_points.iter().rev() {
        stroke_path.line_to(*pt);
      }
      stroke_path.line_to(outer_points[0]);
    }
    stroke_path.into()
  }
}

// (Second impl Area removed; only first Area::stroke is used.)
