//! Perspective transforms between two planes.
//!
//! A [`Homography`] maps any four points onto any other four, carrying straight lines to straight lines. It is what
//! a camera does to a flat surface seen at an angle, so it is used to correct perspective, to place an image onto a
//! quad, and to work out which parts of a warped image the source still covers.

use super::quad::Quad;

/// A perspective transform, written as the eight numbers `a` to `h` of
/// `x' = (a*x + b*y + c) / (g*x + h*y + 1)` and `y' = (d*x + e*y + f) / (g*x + h*y + 1)`.
///
/// Positions where `g*x + h*y + 1` reaches zero are on the line where the transform goes to infinity, the horizon of
/// the plane. Positions past it have no meaningful image, and [`Homography::map_finite`] returns `None` for them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Homography([f64; 8]);

impl Homography {
  /// The transform that takes each corner of `p_from` onto the matching corner of `p_to`. Returns `None` if either
  /// quad is degenerate, for example if three of its corners lie on a line.
  pub fn from_quads(p_from: &Quad, p_to: &Quad) -> Option<Homography> {
    Homography::from_points(&p_from.to_f64_corners(), &p_to.to_f64_corners())
  }

  /// The transform that takes each of the four `p_from` points onto the matching `p_to` point. Returns `None` if the
  /// points are degenerate, for example if three of them lie on a line.
  pub fn from_points(p_from: &[(f64, f64); 4], p_to: &[(f64, f64); 4]) -> Option<Homography> {
    // Each point gives two equations in the eight unknowns.
    let mut rows = [[0.0f64; 9]; 8];
    for (i, (&(u, v), &(x, y))) in p_from.iter().zip(p_to).enumerate() {
      rows[2 * i] = [u, v, 1.0, 0.0, 0.0, 0.0, -u * x, -v * x, x];
      rows[2 * i + 1] = [0.0, 0.0, 0.0, u, v, 1.0, -u * y, -v * y, y];
    }

    // Gaussian elimination with partial pivoting.
    for column in 0..8 {
      let pivot = (column..8).max_by(|&a, &b| rows[a][column].abs().total_cmp(&rows[b][column].abs()))?;
      if rows[pivot][column].abs() < 1e-9 {
        return None;
      }
      rows.swap(column, pivot);
      for row in 0..8 {
        if row != column {
          let factor = rows[row][column] / rows[column][column];
          for k in column..9 {
            rows[row][k] -= factor * rows[column][k];
          }
        }
      }
    }

    let mut coefficients = [0.0; 8];
    for i in 0..8 {
      coefficients[i] = rows[i][8] / rows[i][i];
    }
    Some(Homography(coefficients))
  }

  /// The eight numbers `a` to `h` of the transform, as described on [`Homography`].
  pub fn coefficients(&self) -> [f64; 8] {
    self.0
  }

  /// Where the transform takes the point `(p_x, p_y)`. Points on or past the horizon come out at infinity or
  /// mirrored; use [`Homography::map_finite`] to leave them out.
  pub fn map(&self, p_x: f64, p_y: f64) -> (f64, f64) {
    let [a, b, c, d, e, f, g, h] = self.0;
    let divisor = g * p_x + h * p_y + 1.0;
    ((a * p_x + b * p_y + c) / divisor, (d * p_x + e * p_y + f) / divisor)
  }

  /// Like [`Homography::map`], but `None` when the point is on or past the horizon, so it has no meaningful position.
  pub fn map_finite(&self, p_x: f64, p_y: f64) -> Option<(f64, f64)> {
    let [_, _, _, _, _, _, g, h] = self.0;
    (g * p_x + h * p_y + 1.0 > 1e-9).then(|| self.map(p_x, p_y))
  }

  /// The largest window this transform fully covers with a source rectangle, as `(left, top, scale)`: the window is
  /// `p_size` times `scale`, with its top-left corner at `(left, top)`. It keeps the aspect ratio of `p_size` and
  /// stays within `p_bounds`, given as `(left, top, right, bottom)`. The transform takes the window's plane to the
  /// source, a `p_source` sized rectangle from the origin.
  ///
  /// This is how far to zoom into a warped image so every pixel comes from the source. Returns `(0, 0, 1)` when no
  /// window fits.
  ///
  /// A position maps inside the source when `0 <= x' <= width` and `0 <= y' <= height`. Multiplied through by the
  /// transform's divisor, which must be positive, each of those is a straight line on the plane, so the covered part
  /// of the plane is convex and the window fits inside it exactly when its four corners do. Each corner is linear in
  /// `(left, top, scale)`, so finding the biggest window is a small linear program. With three unknowns its best
  /// answer is where three of the limits meet, so every such meeting point is tried.
  pub fn covered_window(
    &self, p_size: (f64, f64), p_source: (f64, f64), p_bounds: (f64, f64, f64, f64),
  ) -> (f64, f64, f64) {
    let [a, b, c, d, e, f, g, h] = self.0;
    let (width, height) = p_size;
    let (source_width, source_height) = p_source;
    let (bounds_left, bounds_top, bounds_right, bounds_bottom) = p_bounds;

    // Each limit is `row . (left, top, scale) <= limit`.
    let mut limits: Vec<([f64; 3], f64)> = vec![
      ([-1.0, 0.0, 0.0], -bounds_left),
      ([0.0, -1.0, 0.0], -bounds_top),
      ([1.0, 0.0, width], bounds_right),
      ([0.0, 1.0, height], bounds_bottom),
      ([0.0, 0.0, -1.0], 0.0),
    ];
    for (corner_x, corner_y) in [(0.0, 0.0), (width, 0.0), (width, height), (0.0, height)] {
      // A limit on a corner's position, `along_x * x + along_y * y <= limit`, where the corner is at
      // `(left + corner_x * scale, top + corner_y * scale)`.
      let mut limit_corner = |along_x: f64, along_y: f64, limit: f64| {
        limits.push(([along_x, along_y, along_x * corner_x + along_y * corner_y], limit));
      };
      // In front of the horizon: `g*x + h*y + 1` stays above a tiny amount.
      limit_corner(-g, -h, 1.0 - 1e-6);
      // `x' >= 0` and `x' <= source_width`, then the same for `y'`.
      limit_corner(-a, -b, c);
      limit_corner(a - source_width * g, b - source_width * h, source_width - c);
      limit_corner(-d, -e, f);
      limit_corner(d - source_height * g, e - source_height * h, source_height - f);
    }

    let fits = |point: [f64; 3]| {
      limits.iter().all(|(row, limit)| {
        let value = row[0] * point[0] + row[1] * point[1] + row[2] * point[2];
        let size = row[0].abs() * point[0].abs() + row[1].abs() * point[1].abs() + row[2].abs() * point[2].abs();
        value <= limit + 1e-9 * (1.0 + limit.abs() + size)
      })
    };

    let mut corners: Vec<[f64; 3]> = Vec::new();
    for i in 0..limits.len() {
      for j in i + 1..limits.len() {
        for k in j + 1..limits.len() {
          let rows = [limits[i].0, limits[j].0, limits[k].0];
          if let Some(point) = solve_3x3(rows, [limits[i].1, limits[j].1, limits[k].1])
            && point[2] > 0.0
            && fits(point)
          {
            corners.push(point);
          }
        }
      }
    }

    let Some(biggest) = corners.iter().map(|point| point[2]).max_by(f64::total_cmp) else {
      return (0.0, 0.0, 1.0);
    };
    // When the biggest window can sit in more than one place, such as sliding sideways along a strip, the limits
    // meet at each end of the range. Their average is inside it too and keeps the window centered.
    let best: Vec<&[f64; 3]> = corners.iter().filter(|point| point[2] >= biggest * (1.0 - 1e-9)).collect();
    let count = best.len() as f64;
    let left = best.iter().map(|point| point[0]).sum::<f64>() / count;
    let top = best.iter().map(|point| point[1]).sum::<f64>() / count;
    (left, top, best.iter().map(|point| point[2]).sum::<f64>() / count)
  }
}

/// Solves three equations in three unknowns, `p_rows[i] . x = p_values[i]`, or `None` if they have no single answer.
fn solve_3x3(p_rows: [[f64; 3]; 3], p_values: [f64; 3]) -> Option<[f64; 3]> {
  let determinant = |m: [[f64; 3]; 3]| {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
      + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
  };
  let whole = determinant(p_rows);
  // Scale the test for a zero determinant by the size of the numbers, which run from tiny to the image size.
  let size = p_rows.iter().map(|row| row.iter().map(|value| value.abs()).sum::<f64>()).product::<f64>();
  if whole.abs() <= 1e-12 * size {
    return None;
  }
  let mut answer = [0.0; 3];
  for (column, value) in answer.iter_mut().enumerate() {
    let mut replaced = p_rows;
    for row in 0..3 {
      replaced[row][column] = p_values[row];
    }
    *value = determinant(replaced) / whole;
  }
  Some(answer)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_transform_maps_the_corners_exactly() {
    let quad = Quad::new((10, 20), (90, 10), (100, 80), (0, 90));
    let rectangle = Quad::rect((0, 0), (80, 60));
    let to_source = Homography::from_quads(&rectangle, &quad).unwrap();
    let to_flat = Homography::from_quads(&quad, &rectangle).unwrap();
    for (&(u, v), &(x, y)) in rectangle.to_f64_corners().iter().zip(&quad.to_f64_corners()) {
      let (mapped_x, mapped_y) = to_source.map(u, v);
      assert!((mapped_x - x).abs() < 1e-6 && (mapped_y - y).abs() < 1e-6, "{u},{v} -> {mapped_x},{mapped_y}");
      // And the other way round.
      let (back_u, back_v) = to_flat.map(x, y);
      assert!((back_u - u).abs() < 1e-6 && (back_v - v).abs() < 1e-6, "{x},{y} -> {back_u},{back_v}");
    }
  }

  #[test]
  fn degenerate_points_have_no_transform() {
    let line = Quad::new((0, 0), (10, 10), (20, 20), (30, 30));
    assert!(Homography::from_quads(&line, &Quad::rect((0, 0), (10, 10))).is_none());
  }

  #[test]
  fn the_covered_window_only_holds_positions_inside_the_source() {
    for quad in [
      Quad::new((50, 40), (150, 60), (140, 150), (60, 130)),
      Quad::new((50, 30), (160, 20), (160, 130), (30, 130)),
      // Narrows sharply towards the top, like a tall building shot from its foot.
      Quad::new((45, 10), (155, 10), (195, 160), (5, 160)),
      Quad::new((45, 5), (55, 5), (100, 35), (0, 35)),
    ] {
      let (width, height) = (200.0, 200.0);
      let to_source = Homography::from_quads(&quad.bounds().into(), &quad).unwrap();
      let (left, top, scale) = to_source.covered_window((width, height), (width, height), (0.0, 0.0, width, height));
      assert!(scale > 0.0 && scale <= 1.0, "scale {scale} for {quad:?}");
      // The window stays inside the bounds, so the view only zooms in.
      assert!(left >= -1e-6 && top >= -1e-6, "window starts at {left},{top} for {quad:?}");
      assert!(left + width * scale <= width + 1e-6 && top + height * scale <= height + 1e-6, "{quad:?}");
      for (x, y) in [(0.0, 0.0), (width, 0.0), (width, height), (0.0, height)] {
        let (source_x, source_y) = to_source.map_finite(left + x * scale, top + y * scale).expect("past infinity");
        assert!(
          (-1e-6..=width + 1e-6).contains(&source_x) && (-1e-6..=height + 1e-6).contains(&source_y),
          "corner {x},{y} comes from {source_x},{source_y} for {quad:?}"
        );
      }
    }
  }
}
