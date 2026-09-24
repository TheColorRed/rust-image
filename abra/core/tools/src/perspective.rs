use std::cmp::Ordering;

use abra_core::{Area, Image, PointF, Segment, TransformAlgorithm, sample};
use rayon::prelude::*;

use crate::Tool;

pub struct Perspective {
  area: Area,
  algorithm: TransformAlgorithm,
  crop: bool,
}

impl Perspective {
  /// Sets the interpolation algorithm used to resample the image. Defaults to `TransformAlgorithm::Lanczos`,
  /// the highest quality and the slowest.
  ///
  /// The edge-directed resize algorithms have no meaning here and use Lanczos.
  pub fn with_algorithm(mut self, p_algorithm: TransformAlgorithm) -> Self {
    self.algorithm = p_algorithm;
    self
  }

  /// Sets whether to crop the result to the area. Defaults to `true`.
  ///
  /// When `true` only the flattened area is kept, at the size of the area. When `false` the result stays the same
  /// size as the image and the area is stretched out in place to its bounding box, with the view zoomed in if needed
  /// so that every pixel comes from the source: nothing is stretched from the edges and no transparent gaps open up.
  pub fn with_crop(mut self, p_crop: bool) -> Self {
    self.crop = p_crop;
    self
  }
}

impl Tool for Perspective {
  fn apply<'a>(&self, p_image: impl Into<abra_core::ImageRef<'a>>) {
    let segments = self.area.path.segments();
    if segments.len() < 4 {
      return;
    }
    let Some(corners) = quad_corners(&self.area) else {
      return;
    };
    let quad = corners.map(|corner| (corner.x as f64, corner.y as f64));

    let mut image_ref = p_image.into();
    let image: &mut Image = &mut image_ref;
    let (image_width, image_height): (u32, u32) = image.dimensions();

    // Cropping flattens the area onto a rectangle from (0, 0) at least as wide and tall as the longer of each pair
    // of opposite sides, shaped so that nothing changes shape at the corner where its two longest sides meet, the
    // part nearest the camera. Sized by the sides alone, the far end would come out squashed.
    //
    // Otherwise the area is stretched out in place over its bounding box. Each corner only moves outwards, and no
    // further than the corners next to it already reach: the top-left moves up no higher than the top-right and left
    // no further than the bottom-left, and so on round. So the top lines up with the higher top corner, the bottom
    // with the lower bottom corner, and each side with the corner that sticks out further.
    let (width, height, target) = if self.crop {
      let (area_width, area_height) = output_size(&corners);
      let (grown_width, grown_height) = grown_to(flattened_aspect(&quad), area_width as f64, area_height as f64);
      let (grown_width, grown_height) = (grown_width.round().max(1.0), grown_height.round().max(1.0));
      (grown_width as usize, grown_height as usize, rectangle_corners(grown_width, grown_height))
    } else {
      (image_width as usize, image_height as usize, bounding_box_corners(&quad))
    };
    // Takes a position in the result back to a position in the source image.
    let Some(to_source) = solve(&target, &quad) else {
      return;
    };

    // The result shows a window onto the warped plane, as `(left, top, scale)`: result pixel `x` shows the plane at
    // `left + x * scale`. Cropping shows the flattened rectangle as it is. Otherwise stretching the area out also
    // pulls some of the plane from past the edges of the source, most of all beyond the area's longer sides, so the
    // window is the largest part of the image's frame that the source fully covers.
    let (left, top, scale) = if self.crop {
      (0.0, 0.0, 1.0)
    } else {
      let bounds = (0.0, 0.0, width as f64, height as f64);
      covered_window(&to_source, (width as f64, height as f64), (image_width as f64, image_height as f64), bounds)
    };

    // Rounding can still put the outermost pixels a hair past the edges of the source. Those take the nearest edge
    // pixel. The sampler reads anything past the edges as transparent, so it samples a copy with the edge pixels
    // repeated out as far as its widest kernel reaches, which keeps the result's edges from fading out.
    let clamp = !self.crop;
    let (max_x, max_y) = (image_width as f64 - 0.5, image_height as f64 - 0.5);
    let padded;
    let (source, offset): (&Image, f64) = if clamp {
      padded = with_repeated_edges(image, EDGE_PADDING);
      (&padded, EDGE_PADDING as f64)
    } else {
      (image, 0.0)
    };

    let interpolation = self.algorithm.interpolation();
    let mut pixels = vec![0u8; width * height * 4];
    pixels.par_chunks_mut(4).enumerate().for_each(|(index, pixel)| {
      // Each output pixel is found by mapping its center back into the source image. The map works in continuous
      // coordinates, where pixel `i` spans `i..i + 1`, while sampling takes the position of a pixel center.
      let x = left + ((index % width) as f64 + 0.5) * scale;
      let y = top + ((index / width) as f64 + 0.5) * scale;
      // Past the line where the warp goes to infinity there is no source pixel, so the pixel stays transparent.
      if let Some((mut source_x, mut source_y)) = to_source.map_finite(x, y) {
        if clamp {
          source_x = source_x.clamp(0.5, max_x);
          source_y = source_y.clamp(0.5, max_y);
        }
        let (sample_x, sample_y) = ((source_x + offset - 0.5) as f32, (source_y + offset - 0.5) as f32);
        pixel.copy_from_slice(&sample(source, sample_x, sample_y, interpolation));
      }
    });

    image.set_new_pixels(&pixels, width as u32, height as u32);
  }
}

/// Correct the perspective of an image, using an area that should be a rectangle but was photographed at an angle,
/// such as a building, a sign or a page.
///
/// The image is warped so the four corners of the area become the corners of a rectangle. The rectangle is at least
/// as wide and tall as the longer of each pair of opposite sides, and shaped so that nothing changes shape at the
/// corner where the two longest sides meet, the part of the area nearest the camera. Sized by the sides alone, the
/// far end would come out squashed, like the top of a building shot from its foot. By default the result is
/// cropped to that rectangle.
///
/// With [`Perspective::with_crop`] set to `false` the whole image is kept instead, at its original size, and the
/// area is stretched out in place to fill its bounding box. Each corner only moves outwards, and no further than
/// the corners next to it already reach: the top lines up with the higher of the two top corners, the bottom with
/// the lower of the two bottom corners, and each side with whichever of its corners sticks out further. That also
/// pulls in parts of the view from past the edges of the source, most of all beyond the area's longer sides, so the
/// result then zooms in just enough to leave them out: nothing is stretched from the edges and no transparent gaps
/// open up.
///
/// The area must be a polygon with exactly four corners that form a convex shape, like
/// `Area::from_points(&[[x, y], ...])`. The corners can be given in any order. Anything else, such as a
/// circle or a shape with curves, leaves the image unchanged.
pub fn perspective(p_area: impl Into<Area>) -> Perspective {
  Perspective {
    area: p_area.into(),
    algorithm: TransformAlgorithm::Lanczos,
    crop: true,
  }
}

/// The four corners of a straight-sided, four-cornered convex area, ordered top-left, top-right, bottom-right,
/// bottom-left. Returns `None` for any other shape.
fn quad_corners(p_area: &Area) -> Option<[PointF; 4]> {
  // Curves have no corners to pull out.
  if p_area.segments().iter().any(|segment| !matches!(segment, Segment::Line { .. })) {
    return None;
  }

  let mut points = p_area.points();
  // A path that closes itself repeats its first point at the end.
  if points.len() > 1 && points.first() == points.last() {
    points.pop();
  }
  if points.len() != 4 {
    return None;
  }

  // Put the corners in order around the shape. Because y points down, going clockwise on screen is going by
  // increasing angle around the middle. Unlike picking corners by their position, this holds however far the
  // shape is skewed, such as a building narrowing sharply towards the top.
  let center_x = points.iter().map(|point| point.x).sum::<f32>() / 4.0;
  let center_y = points.iter().map(|point| point.y).sum::<f32>() / 4.0;
  let angle = |point: &PointF| (point.y - center_y).atan2(point.x - center_x);
  points.sort_by(|a, b| angle(a).total_cmp(&angle(b)));

  // Going around the corners, every turn must be the same way. Otherwise the shape has a dent, or two corners are
  // the same point, or three of them lie on a line, and it cannot be flattened into a rectangle.
  let turns: Vec<f32> = (0..4)
    .map(|i| {
      let (a, b, c) = (points[i], points[(i + 1) % 4], points[(i + 2) % 4]);
      (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x)
    })
    .collect();
  if !(turns.iter().all(|&turn| turn > 0.0) || turns.iter().all(|&turn| turn < 0.0)) {
    return None;
  }

  // Going clockwise, the top edge runs from left to right, so it starts at the top-left corner. Of the edges that
  // run that way, the top one is the closest to level. Picking the highest edge instead would turn a tilted
  // rectangle a quarter turn once it is tilted far enough.
  let slope = |i: usize| {
    let (from, to) = (points[i], points[(i + 1) % 4]);
    (to.y - from.y).abs().atan2(to.x - from.x)
  };
  let top = (0..4).filter(|&i| points[(i + 1) % 4].x > points[i].x).min_by(|&a, &b| slope(a).total_cmp(&slope(b)))?;
  Some([
    points[top],
    points[(top + 1) % 4],
    points[(top + 2) % 4],
    points[(top + 3) % 4],
  ])
}

/// The size in pixels of the rectangle the corners are stretched out to: the longer of each pair of opposite sides.
fn output_size(p_corners: &[PointF; 4]) -> (usize, usize) {
  let [top_left, top_right, bottom_right, bottom_left] = *p_corners;
  let width = top_left.distance_to(top_right).max(bottom_left.distance_to(bottom_right));
  let height = top_left.distance_to(bottom_left).max(top_right.distance_to(bottom_right));
  (width.round().max(1.0) as usize, height.round().max(1.0) as usize)
}

/// The corners of a `p_width` x `p_height` rectangle at the origin, in the same order as the corners of a quad:
/// top-left, top-right, bottom-right, bottom-left.
fn rectangle_corners(p_width: f64, p_height: f64) -> [(f64, f64); 4] {
  [(0.0, 0.0), (p_width, 0.0), (p_width, p_height), (0.0, p_height)]
}

/// How far past each edge the widest sampling kernel, Lanczos, reads.
const EDGE_PADDING: u32 = 3;

/// A copy of the image grown by `p_padding` pixels on every side, filled by repeating the nearest edge pixel.
fn with_repeated_edges(p_image: &Image, p_padding: u32) -> Image {
  let (width, height): (u32, u32) = p_image.dimensions();
  let (padded_width, padded_height) = (width + 2 * p_padding, height + 2 * p_padding);
  let pixels = p_image.rgba();
  let mut bytes = Vec::with_capacity((padded_width * padded_height * 4) as usize);
  for y in 0..padded_height {
    let source_y = y.saturating_sub(p_padding).min(height - 1);
    for x in 0..padded_width {
      let source_x = x.saturating_sub(p_padding).min(width - 1);
      let index = ((source_y * width + source_x) * 4) as usize;
      bytes.extend_from_slice(&pixels[index..index + 4]);
    }
  }
  Image::from_rgba_bytes(padded_width, padded_height, &bytes)
}

/// The largest window onto the warped plane that the source image fully covers, as `(left, top, scale)`: the window
/// is `p_size` times `scale`, with its top-left corner at `(left, top)`. It keeps the aspect ratio of `p_size` and
/// stays within `p_bounds`, given as `(left, top, right, bottom)`. `p_to_source` maps the plane back to the source,
/// which is `p_source` in size.
///
/// A position maps inside the source when `0 <= x' <= width` and `0 <= y' <= height`. Multiplied through by the
/// transform's divisor, which must be positive, each of those is a straight line on the plane, so the covered part
/// of the plane is convex and the window fits inside it exactly when its four corners do. Each corner is linear in
/// `(left, top, scale)`, so finding the biggest window is a small linear program. With three unknowns its best
/// answer is where three of the limits meet, so every such meeting point is tried.
fn covered_window(
  p_to_source: &Homography, p_size: (f64, f64), p_source: (f64, f64), p_bounds: (f64, f64, f64, f64),
) -> (f64, f64, f64) {
  let [a, b, c, d, e, f, g, h] = p_to_source.0;
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
    // In front of the line where the warp goes to infinity: `g*x + h*y + 1` stays above a tiny amount.
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

/// The width-to-height ratio of the rectangle a quad is flattened to, or `None` for a quad too degenerate to tell.
///
/// Without knowing the camera, the true shape of the rectangle behind a quad cannot be worked out: with the top
/// and bottom level, as when a building is shot from its foot, it depends on the lens. So the shape is picked to
/// keep things undistorted at the corner where the quad's two longest sides meet. That corner is the part nearest
/// the camera, where the photo is closest to the real proportions, and it is where the result stays put.
fn flattened_aspect(p_quad: &[(f64, f64); 4]) -> Option<f64> {
  let [top_left, top_right, bottom_right, bottom_left] = *p_quad;
  let (vertical, horizontal) = longer_sides(p_quad);
  let (x, y) = match (vertical == Ordering::Less, horizontal == Ordering::Less) {
    (false, false) => bottom_right,
    (false, true) => bottom_left,
    (true, false) => top_right,
    (true, true) => top_left,
  };

  // Flattened to a unit square, a small square in the photo at the corner comes out as a patch whose width and
  // height follow how fast the position across and down the square changes around the corner. Stretching the
  // square to `aspect` wide makes that patch as wide as it is tall, as near as a skewed corner allows.
  let to_square = solve(p_quad, &rectangle_corners(1.0, 1.0))?;
  let [a, b, _, d, e, _, g, h] = to_square.0;
  let (across, down) = to_square.map(x, y);
  let divisor = g * x + h * y + 1.0;
  let change_across = ((a - g * across) / divisor).hypot((b - h * across) / divisor);
  let change_down = ((d - g * down) / divisor).hypot((e - h * down) / divisor);
  let aspect = change_down / change_across;
  (aspect.is_finite() && aspect > 0.0).then_some(aspect)
}

/// Which of each pair of opposite sides of a quad is longer, as `(bottom against top, right against left)`: for
/// example `Greater` first when the bottom is longer. Sides within a hair of each other count as `Equal`.
fn longer_sides(p_quad: &[(f64, f64); 4]) -> (Ordering, Ordering) {
  let [top_left, top_right, bottom_right, bottom_left] = *p_quad;
  let length = |from: (f64, f64), to: (f64, f64)| (to.0 - from.0).hypot(to.1 - from.1);
  let compare = |one: f64, other: f64| {
    if (one - other).abs() <= 1e-6 * one.max(other) { Ordering::Equal } else { one.total_cmp(&other) }
  };
  (
    compare(length(bottom_left, bottom_right), length(top_left, top_right)),
    compare(length(top_right, bottom_right), length(top_left, bottom_left)),
  )
}

/// The size of a `p_width` x `p_height` box grown in one direction until its width over height is `p_aspect`, or
/// the box as it is without an aspect.
fn grown_to(p_aspect: Option<f64>, p_width: f64, p_height: f64) -> (f64, f64) {
  match p_aspect {
    Some(aspect) => {
      let height = p_height.max(p_width / aspect);
      (height * aspect, height)
    }
    None => (p_width, p_height),
  }
}

/// The corners of the smallest upright box around a quad, in the same order as the quad's corners: top-left,
/// top-right, bottom-right, bottom-left.
fn bounding_box_corners(p_quad: &[(f64, f64); 4]) -> [(f64, f64); 4] {
  let left = p_quad.iter().map(|corner| corner.0).fold(f64::INFINITY, f64::min);
  let right = p_quad.iter().map(|corner| corner.0).fold(f64::NEG_INFINITY, f64::max);
  let top = p_quad.iter().map(|corner| corner.1).fold(f64::INFINITY, f64::min);
  let bottom = p_quad.iter().map(|corner| corner.1).fold(f64::NEG_INFINITY, f64::max);
  [(left, top), (right, top), (right, bottom), (left, bottom)]
}

/// A perspective transform, written as the eight numbers `a` to `h` of
/// `x' = (a*x + b*y + c) / (g*x + h*y + 1)` and `y' = (d*x + e*y + f) / (g*x + h*y + 1)`.
struct Homography([f64; 8]);

impl Homography {
  fn map(&self, p_x: f64, p_y: f64) -> (f64, f64) {
    let [a, b, c, d, e, f, g, h] = self.0;
    let divisor = g * p_x + h * p_y + 1.0;
    ((a * p_x + b * p_y + c) / divisor, (d * p_x + e * p_y + f) / divisor)
  }

  /// Like `map`, but `None` when the point is on or past the line where the warp goes to infinity, so it has no
  /// meaningful position.
  fn map_finite(&self, p_x: f64, p_y: f64) -> Option<(f64, f64)> {
    let [_, _, _, _, _, _, g, h] = self.0;
    (g * p_x + h * p_y + 1.0 > 1e-9).then(|| self.map(p_x, p_y))
  }
}

/// The transform that takes each of the four `p_from` points onto the matching `p_to` point. Returns `None` if the
/// points are degenerate, for example if three of them lie on a line.
fn solve(p_from: &[(f64, f64); 4], p_to: &[(f64, f64); 4]) -> Option<Homography> {
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

  let mut solution = [0.0; 8];
  for i in 0..8 {
    solution[i] = rows[i][8] / rows[i][i];
  }
  Some(Homography(solution))
}

#[cfg(test)]
mod tests {
  use super::*;

  const GRAY: [u8; 4] = [128, 128, 128, 255];
  const RED: [u8; 4] = [255, 0, 0, 255];

  /// A gray image with the given quad filled red.
  fn image_with_quad(p_width: u32, p_height: u32, p_quad: &[[f32; 2]]) -> Image {
    let area = Area::from_points(p_quad);
    let mut bytes = Vec::with_capacity((p_width * p_height * 4) as usize);
    for y in 0..p_height {
      for x in 0..p_width {
        bytes.extend_from_slice(if area.contains((x as f32 + 0.5, y as f32 + 0.5)) { &RED } else { &GRAY });
      }
    }
    Image::from_rgba_bytes(p_width, p_height, &bytes)
  }

  fn share_of_red(p_image: &Image) -> f32 {
    let pixels = p_image.rgba().chunks(4);
    let total = pixels.len() as f32;
    pixels.filter(|pixel| pixel[0] > 200 && pixel[1] < 60 && pixel[2] < 60).count() as f32 / total
  }

  #[test]
  fn the_whole_image_as_the_area_changes_nothing() {
    let bytes: Vec<u8> =
      (0..40 * 30).flat_map(|i| [(i % 251) as u8, (i % 13 * 19) as u8, (i % 7 * 36) as u8, 255]).collect();
    let mut image = Image::from_rgba_bytes(40, 30, &bytes);
    perspective(Area::rect((0, 0), (40, 30))).apply(&mut image);

    assert_eq!(image.dimensions::<u32>(), (40, 30));
    let worst = image.rgba().iter().zip(&bytes).map(|(&a, &b)| (a as i32 - b as i32).abs()).max().unwrap();
    assert!(worst <= 2, "pixels drifted by {worst}");
  }

  #[test]
  fn a_skewed_quad_becomes_a_rectangle() {
    let quad = [[50.0, 40.0], [150.0, 60.0], [140.0, 150.0], [60.0, 130.0]];
    let mut image = image_with_quad(200, 200, &quad);
    perspective(Area::from_points(&quad)).with_crop(true).apply(&mut image);

    // The longer of the top and bottom edges is 102px, and of the left and right edges 91px. The rectangle is at
    // least that big, grown in one direction to the shape that leaves the nearest corner undistorted.
    let (width, height): (u32, u32) = image.dimensions();
    assert!(width >= 102 && height >= 91, "{width}x{height}");
    let expected = flattened_aspect(&quad.map(|[x, y]| (x as f64, y as f64))).unwrap();
    assert!((width as f64 / height as f64 - expected).abs() < 0.02, "{width}x{height} is not {expected:.3} wide");
    // Only the pixels right along the border can pick up the gray outside.
    assert!(share_of_red(&image) > 0.9, "only {:.0}% of the result is red", share_of_red(&image) * 100.0);
  }

  #[test]
  fn corners_are_found_however_skewed_or_tilted_the_shape_is() {
    // Narrows so sharply that the bottom-left corner has a smaller x + y than the top-left one.
    let keystone = [[45.0, 5.0], [55.0, 5.0], [100.0, 35.0], [0.0, 35.0]];
    // A rectangle turned by 30 degrees, listed starting from the corner that ends up top-left.
    let tilted = [[30.0, 10.0], [116.6, 60.0], [91.6, 103.3], [5.0, 53.3]];

    for (shape, expected) in [(keystone, keystone), (tilted, tilted)] {
      // Try every starting corner and both directions round the shape.
      for start in 0..4 {
        for reversed in [false, true] {
          let mut points: Vec<[f32; 2]> = (0..4).map(|i| shape[(start + i) % 4]).collect();
          if reversed {
            points.reverse();
          }
          let corners = quad_corners(&Area::from_points(&points)).expect("expected four corners");
          let found: Vec<[f32; 2]> = corners.iter().map(|corner| [corner.x, corner.y]).collect();
          assert_eq!(found, expected, "start {start}, reversed {reversed}");
        }
      }
    }
  }

  #[test]
  fn corner_order_does_not_matter() {
    let quad = [[50.0, 40.0], [150.0, 60.0], [140.0, 150.0], [60.0, 130.0]];
    let shuffled = [[140.0, 150.0], [50.0, 40.0], [60.0, 130.0], [150.0, 60.0]];

    let mut a = image_with_quad(200, 200, &quad);
    let mut b = a.clone();
    perspective(Area::from_points(&quad)).apply(&mut a);
    perspective(Area::from_points(&shuffled)).apply(&mut b);
    assert_eq!(a.dimensions::<u32>(), b.dimensions::<u32>());
    assert_eq!(a.rgba(), b.rgba());
  }

  #[test]
  fn shapes_without_four_straight_convex_corners_change_nothing() {
    let shapes = [
      ("a circle", Area::circle((50, 50), 30)),
      ("a triangle", Area::from_points(&[[10.0, 10.0], [90.0, 20.0], [50.0, 80.0]])),
      ("a dented shape", Area::from_points(&[[10.0, 10.0], [90.0, 10.0], [50.0, 90.0], [50.0, 30.0]])),
      ("a line", Area::from_points(&[[10.0, 10.0], [20.0, 20.0], [30.0, 30.0], [40.0, 40.0]])),
    ];
    for (name, area) in shapes {
      let mut image = image_with_quad(100, 100, &[[20.0, 20.0], [80.0, 20.0], [80.0, 80.0], [20.0, 80.0]]);
      let before = image.rgba().to_vec();
      perspective(area).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), (100, 100), "{name}");
      assert_eq!(image.rgba(), before.as_slice(), "{name}");
    }
  }

  #[test]
  fn the_transform_maps_the_corners_exactly() {
    let quad = [(10.0, 20.0), (90.0, 10.0), (100.0, 80.0), (0.0, 90.0)];
    let rectangle = rectangle_corners(80.0, 60.0);
    let to_source = solve(&rectangle, &quad).unwrap();
    let to_flat = solve(&quad, &rectangle).unwrap();
    for (&(u, v), &(x, y)) in rectangle.iter().zip(&quad) {
      let (mapped_x, mapped_y) = to_source.map(u, v);
      assert!((mapped_x - x).abs() < 1e-6 && (mapped_y - y).abs() < 1e-6, "{u},{v} -> {mapped_x},{mapped_y}");
      // And the other way round.
      let (back_u, back_v) = to_flat.map(x, y);
      assert!((back_u - u).abs() < 1e-6 && (back_v - v).abs() < 1e-6, "{x},{y} -> {back_u},{back_v}");
    }
  }

  /// The smallest box around the red pixels, as `(left, top, width, height)`, and how many there are.
  fn red_box(p_image: &Image) -> ((u32, u32, u32, u32), usize) {
    let (width, _): (u32, u32) = p_image.dimensions();
    let (mut min, mut max, mut count) = ((u32::MAX, u32::MAX), (0, 0), 0);
    for (index, pixel) in p_image.rgba().chunks(4).enumerate() {
      if pixel[0] > 200 && pixel[1] < 60 && pixel[2] < 60 && pixel[3] > 200 {
        let (x, y) = (index as u32 % width, index as u32 / width);
        min = (min.0.min(x), min.1.min(y));
        max = (max.0.max(x), max.1.max(y));
        count += 1;
      }
    }
    ((min.0, min.1, max.0 - min.0 + 1, max.1 - min.1 + 1), count)
  }

  #[test]
  fn without_a_crop_the_area_becomes_an_upright_box_shaped_like_its_bounding_box() {
    // The first is lopsided, the second like a building photographed from below: the bottom is level and the
    // widest, the right side is upright and the tallest, and the top and left lean in.
    for quad in [
      [[50.0, 40.0], [150.0, 60.0], [140.0, 150.0], [60.0, 130.0]],
      [[50.0, 30.0], [160.0, 20.0], [160.0, 130.0], [30.0, 130.0]],
    ] {
      let mut image = image_with_quad(200, 170, &quad);
      perspective(Area::from_points(&quad)).with_crop(false).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), (200, 170));

      let ((_, _, red_width, red_height), red) = red_box(&image);
      let (red_width, red_height) = (red_width as f64, red_height as f64);
      assert!(red as f64 > 0.95 * red_width * red_height, "red fills only {red} of {red_width}x{red_height}");
      let [(left, top), _, (right, bottom), _] = bounding_box_corners(&quad.map(|[x, y]| (x as f64, y as f64)));
      let expected = (right - left) / (bottom - top);
      assert!((red_width / red_height - expected).abs() < 0.04, "red box is {red_width}x{red_height} for {quad:?}");
    }
  }

  #[test]
  fn a_building_shot_from_its_foot_is_not_squashed_when_cropped() {
    // Level top and bottom, with the sides leaning in sharply towards the top.
    let quad = [[93.0, 42.0], [332.0, 42.0], [405.0, 336.0], [9.0, 336.0]];
    let mut image = image_with_quad(420, 420, &quad);
    perspective(Area::from_points(&quad)).with_crop(true).apply(&mut image);

    // Kept to the area's height, it would be 396 wide and 294 tall. The far end, the top, is foreshortened in the
    // photo, so the building has to come out taller than that.
    let (width, height): (u32, u32) = image.dimensions();
    assert_eq!(width, 396);
    assert!(height as f64 > 396.0 / 0.9, "{width}x{height}");
  }

  /// The corners of a flat `p_aspect` wide, 1 tall rectangle 6 units in front of a camera turned by the given angles
  /// in degrees, with a lens of `p_focal` pixels and the image `p_size` in pixels.
  fn photographed_rectangle(p_aspect: f64, p_angles: [f64; 3], p_focal: f64, p_size: (f64, f64)) -> [(f64, f64); 4] {
    let [pitch, yaw, roll] = p_angles.map(f64::to_radians);
    let rotate = |[x, y, z]: [f64; 3]| {
      let (y, z) = (y * pitch.cos() - z * pitch.sin(), y * pitch.sin() + z * pitch.cos());
      let (x, z) = (x * yaw.cos() + z * yaw.sin(), -x * yaw.sin() + z * yaw.cos());
      [x * roll.cos() - y * roll.sin(), x * roll.sin() + y * roll.cos(), z]
    };
    let (half_width, half_height) = (p_aspect / 2.0, 0.5);
    [
      (-half_width, -half_height),
      (half_width, -half_height),
      (half_width, half_height),
      (-half_width, half_height),
    ]
    .map(|(x, y)| {
      let [x, y, z] = rotate([x, y, 0.0]);
      (p_focal * x / (z + 6.0) + p_size.0 / 2.0, p_focal * y / (z + 6.0) + p_size.1 / 2.0)
    })
  }

  #[test]
  fn a_photographed_rectangle_is_flattened_close_to_its_real_shape() {
    for (aspect, angles, focal) in [
      (2.0, [-25.0, 0.0, 0.0], 800.0),
      (2.0, [-25.0, 15.0, 0.0], 800.0),
      (0.6, [-30.0, 5.0, 2.0], 1000.0),
      (1.5, [0.0, 35.0, 0.0], 700.0),
      (1.0, [-20.0, -20.0, 0.0], 900.0),
      (1.3, [-15.0, 25.0, 3.0], 600.0),
    ] {
      let quad = photographed_rectangle(aspect, angles, focal, (1000.0, 800.0));
      let found = flattened_aspect(&quad).unwrap();
      // Without knowing the lens it cannot be exact, but it must be far closer than the photo's own proportions.
      assert!((found / aspect - 1.0).abs() < 0.13, "{aspect} came out as {found:.3} for {angles:?}");
    }
  }

  #[test]
  fn an_upright_rectangle_keeps_its_shape() {
    let found = flattened_aspect(&[(10.0, 20.0), (90.0, 20.0), (90.0, 60.0), (10.0, 60.0)]).unwrap();
    assert!((found - 2.0).abs() < 1e-9, "{found}");
  }

  #[test]
  fn without_a_crop_the_whole_image_as_the_area_changes_nothing() {
    let bytes: Vec<u8> =
      (0..40 * 30).flat_map(|i| [(i % 251) as u8, (i % 13 * 19) as u8, (i % 7 * 36) as u8, 255]).collect();
    let mut image = Image::from_rgba_bytes(40, 30, &bytes);
    perspective(Area::rect((0, 0), (40, 30))).with_crop(false).apply(&mut image);

    assert_eq!(image.dimensions::<u32>(), (40, 30));
    let worst = image.rgba().iter().zip(&bytes).map(|(&a, &b)| (a as i32 - b as i32).abs()).max().unwrap();
    assert!(worst <= 2, "pixels drifted by {worst}");
  }

  #[test]
  fn without_a_crop_every_pixel_comes_from_inside_the_source() {
    for quad in [
      [(50.0, 40.0), (150.0, 60.0), (140.0, 150.0), (60.0, 130.0)],
      [(50.0, 30.0), (160.0, 20.0), (160.0, 130.0), (30.0, 130.0)],
      // Narrows sharply towards the top, like a tall building shot from its foot.
      [(45.0, 10.0), (155.0, 10.0), (195.0, 160.0), (5.0, 160.0)],
      [(45.0, 5.0), (55.0, 5.0), (100.0, 35.0), (0.0, 35.0)],
    ] {
      let (width, height) = (200.0, 200.0);
      let to_source = solve(&bounding_box_corners(&quad), &quad).unwrap();
      let (left, top, scale) = covered_window(&to_source, (width, height), (width, height), (0.0, 0.0, width, height));
      assert!(scale > 0.0 && scale <= 1.0, "scale {scale} for {quad:?}");
      // The window stays inside the result, so the view only zooms in.
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

  #[test]
  fn without_a_crop_no_transparent_gaps_open_up() {
    for quad in [
      [[50.0, 40.0], [150.0, 60.0], [140.0, 150.0], [60.0, 130.0]],
      [[50.0, 30.0], [160.0, 20.0], [160.0, 130.0], [30.0, 130.0]],
      // So strong that without zooming in, the line where the warp goes to infinity would be inside the image.
      [[45.0, 5.0], [55.0, 5.0], [100.0, 35.0], [0.0, 35.0]],
    ] {
      let mut image = image_with_quad(200, 200, &quad);
      perspective(Area::from_points(&quad)).with_crop(false).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), (200, 200));
      let clear = image.rgba().chunks(4).filter(|pixel| pixel[3] < 255).count();
      assert_eq!(clear, 0, "{clear} see-through pixels for {quad:?}");
    }
  }

  #[test]
  fn every_quality_setting_produces_the_same_shape() {
    let quad = [[50.0, 40.0], [150.0, 60.0], [140.0, 150.0], [60.0, 130.0]];
    for quality in [
      TransformAlgorithm::NearestNeighbor,
      TransformAlgorithm::Bilinear,
      TransformAlgorithm::Bicubic,
      TransformAlgorithm::Lanczos,
      TransformAlgorithm::Auto,
    ] {
      let mut image = image_with_quad(200, 200, &quad);
      perspective(Area::from_points(&quad)).with_crop(true).with_algorithm(quality).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), (102, 108), "{quality:?}");
    }
  }

  /// A checkerboard of 20px squares on a 200x120 sheet, seen through a perspective projection.
  fn photographed_checkerboard(p_quad: [(f64, f64); 4], p_width: u32, p_height: u32) -> Image {
    let sheet = rectangle_corners(200.0, 120.0);
    let to_sheet = solve(&p_quad, &sheet).unwrap();
    let mut bytes = Vec::new();
    for y in 0..p_height {
      for x in 0..p_width {
        let color = match to_sheet.map_finite(x as f64 + 0.5, y as f64 + 0.5) {
          Some((u, v)) if (0.0..200.0).contains(&u) && (0.0..120.0).contains(&v) => {
            if ((u / 20.0).floor() as i32 + (v / 20.0).floor() as i32) % 2 == 0 {
              [255, 255, 255, 255]
            } else {
              [0, 0, 0, 255]
            }
          }
          _ => [128, 128, 128, 255],
        };
        bytes.extend_from_slice(&color);
      }
    }
    Image::from_rgba_bytes(p_width, p_height, &bytes)
  }

  #[test]
  fn a_photographed_checkerboard_is_flattened_to_an_even_grid() {
    let quad = [(60.0, 40.0), (170.0, 55.0), (190.0, 150.0), (30.0, 140.0)];
    let mut image = photographed_checkerboard(quad, 220, 180);
    perspective(Area::from_points(&quad.map(|(x, y)| [x as f32, y as f32]))).with_crop(true).apply(&mut image);

    // The sheet is 10x6 squares, so the corrected image must be too, however its aspect ratio came out.
    let (width, height): (u32, u32) = image.dimensions();
    let pixels = image.rgba();
    let (mut wrong, mut checked) = (0, 0);
    for y in 0..height {
      for x in 0..width {
        let (column, row) = ((x as f64 + 0.5) / width as f64 * 10.0, (y as f64 + 0.5) / height as f64 * 6.0);
        // Skip pixels right at a square's edge, where a small offset legitimately flips the color.
        if (column - column.round()).abs() * (width as f64 / 10.0) < 2.0
          || (row - row.round()).abs() * (height as f64 / 6.0) < 2.0
        {
          continue;
        }
        let expected_white = (column.floor() as i32 + row.floor() as i32) % 2 == 0;
        checked += 1;
        if (pixels[((y * width + x) * 4) as usize] > 127) != expected_white {
          wrong += 1;
        }
      }
    }
    assert!(checked > 1000, "only {checked} pixels checked");
    assert!(wrong * 100 <= checked, "{wrong} of {checked} pixels are the wrong color in {width}x{height}");
  }
}
