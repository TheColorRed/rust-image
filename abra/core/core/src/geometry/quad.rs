//! Four-cornered shapes.
//!
//! A [`Quad`] is any shape with four straight sides, such as a rectangle, or a rectangle seen at an angle in a photo.
//! Quads are what a perspective transform ([`Homography`]) maps between, so they are how corners are given to warp
//! an image, correct its perspective, or place it onto another shape.

use std::cmp::Ordering;

use super::homography::Homography;
use crate::{Area, PointF, Rect, Segment, Size};

/// A four-cornered shape, with its corners in the order top-left, top-right, bottom-right, bottom-left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
  pub top_left: PointF,
  pub top_right: PointF,
  pub bottom_right: PointF,
  pub bottom_left: PointF,
}

impl Quad {
  /// Creates a quad from its corners, given as top-left, top-right, bottom-right, bottom-left.
  pub fn new(
    p_top_left: impl Into<PointF>, p_top_right: impl Into<PointF>, p_bottom_right: impl Into<PointF>,
    p_bottom_left: impl Into<PointF>,
  ) -> Quad {
    Quad {
      top_left: p_top_left.into(),
      top_right: p_top_right.into(),
      bottom_right: p_bottom_right.into(),
      bottom_left: p_bottom_left.into(),
    }
  }

  /// Creates an upright rectangle with its top-left corner at `p_origin`.
  pub fn rect(p_origin: impl Into<PointF>, p_size: impl Into<Size>) -> Quad {
    let (origin, size) = (p_origin.into(), p_size.into());
    let (right, bottom) = (origin.x + size.width, origin.y + size.height);
    Quad::new(origin, (right, origin.y), (right, bottom), (origin.x, bottom))
  }

  /// The corners in the order top-left, top-right, bottom-right, bottom-left.
  pub fn corners(&self) -> [PointF; 4] {
    [self.top_left, self.top_right, self.bottom_right, self.bottom_left]
  }

  /// The corners as `f64` pairs, for precise calculations.
  pub(crate) fn to_f64_corners(&self) -> [(f64, f64); 4] {
    self.corners().map(|corner| (corner.x as f64, corner.y as f64))
  }

  /// The smallest upright rectangle around the quad.
  pub fn bounds(&self) -> Rect {
    Rect::from_points(self.corners())
  }

  /// The size in whole pixels of the upright rectangle this quad flattens to, when it is a rectangle photographed at
  /// an angle, such as a building, a sign or a page.
  ///
  /// The rectangle is at least as wide and tall as the longer of each pair of opposite sides, grown in one direction
  /// to the shape from [`Quad::flattened_aspect`]. Sized by the sides alone, the far end would come out squashed,
  /// like the top of a building shot from its foot.
  pub fn flattened_size(&self) -> Size {
    let width = self.top_left.distance_to(self.top_right).max(self.bottom_left.distance_to(self.bottom_right));
    let height = self.top_left.distance_to(self.bottom_left).max(self.top_right.distance_to(self.bottom_right));
    let (width, height) = (width.round().max(1.0) as f64, height.round().max(1.0) as f64);
    let (width, height) = match self.flattened_aspect() {
      Some(aspect) => {
        let height = height.max(width / aspect);
        (height * aspect, height)
      }
      None => (width, height),
    };
    Size::from((width.round().max(1.0), height.round().max(1.0)))
  }

  /// The width-to-height ratio of the rectangle this quad is a photo of, or `None` for a quad too degenerate to tell.
  ///
  /// Without knowing the camera, the true shape of the rectangle behind a quad cannot be worked out: with the top
  /// and bottom level, as when a building is shot from its foot, it depends on the lens. So the shape is picked to
  /// keep things undistorted at the corner where the quad's two longest sides meet. That corner is the part nearest
  /// the camera, where the photo is closest to the real proportions.
  pub fn flattened_aspect(&self) -> Option<f64> {
    let [top_left, top_right, bottom_right, bottom_left] = self.to_f64_corners();
    let (vertical, horizontal) = self.longer_sides();
    let (x, y) = match (vertical == Ordering::Less, horizontal == Ordering::Less) {
      (false, false) => bottom_right,
      (false, true) => bottom_left,
      (true, false) => top_right,
      (true, true) => top_left,
    };

    // Flattened to a unit square, a small square in the photo at the corner comes out as a patch whose width and
    // height follow how fast the position across and down the square changes around the corner. Stretching the
    // square to `aspect` wide makes that patch as wide as it is tall, as near as a skewed corner allows.
    let to_square = Homography::from_quads(self, &Quad::rect((0, 0), (1, 1)))?;
    let [a, b, _, d, e, _, g, h] = to_square.coefficients();
    let (across, down) = to_square.map(x, y);
    let divisor = g * x + h * y + 1.0;
    let change_across = ((a - g * across) / divisor).hypot((b - h * across) / divisor);
    let change_down = ((d - g * down) / divisor).hypot((e - h * down) / divisor);
    let aspect = change_down / change_across;
    (aspect.is_finite() && aspect > 0.0).then_some(aspect)
  }

  /// Which of each pair of opposite sides is longer, as `(bottom against top, right against left)`: for example
  /// `Greater` first when the bottom is longer. Sides within a hair of each other count as `Equal`.
  fn longer_sides(&self) -> (Ordering, Ordering) {
    let [top_left, top_right, bottom_right, bottom_left] = self.to_f64_corners();
    let length = |from: (f64, f64), to: (f64, f64)| (to.0 - from.0).hypot(to.1 - from.1);
    let compare = |one: f64, other: f64| {
      if (one - other).abs() <= 1e-6 * one.max(other) { Ordering::Equal } else { one.total_cmp(&other) }
    };
    (
      compare(length(bottom_left, bottom_right), length(top_left, top_right)),
      compare(length(top_right, bottom_right), length(top_left, bottom_left)),
    )
  }
}

impl From<Rect> for Quad {
  fn from(p_rect: Rect) -> Self {
    let [top_left, top_right, bottom_right, bottom_left] = p_rect.corners();
    Quad::new(top_left, top_right, bottom_right, bottom_left)
  }
}

impl Area {
  /// The area as a [`Quad`], when it is a polygon with exactly four corners forming a convex shape, like
  /// `Area::from_points(&[[x, y], ...])`. The corners can be given in any order. Returns `None` for anything else,
  /// such as a circle or a shape with curves.
  pub fn to_quad(&self) -> Option<Quad> {
    // Curves have no corners to pull out.
    if self.segments().iter().any(|segment| !matches!(segment, Segment::Line { .. })) {
      return None;
    }

    let mut points = self.points();
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
    // the same point, or three of them lie on a line, and it is not a quad.
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
    Some(Quad::new(points[top], points[(top + 1) % 4], points[(top + 2) % 4], points[(top + 3) % 4]))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn corners_are_found_however_skewed_or_tilted_the_shape_is() {
    // Narrows so sharply that the bottom-left corner has a smaller x + y than the top-left one.
    let keystone = [[45.0, 5.0], [55.0, 5.0], [100.0, 35.0], [0.0, 35.0]];
    // A rectangle turned by 30 degrees, listed starting from the corner that ends up top-left.
    let tilted = [[30.0, 10.0], [116.6, 60.0], [91.6, 103.3], [5.0, 53.3]];

    for shape in [keystone, tilted] {
      // Try every starting corner and both directions round the shape.
      for start in 0..4 {
        for reversed in [false, true] {
          let mut points: Vec<[f32; 2]> = (0..4).map(|i| shape[(start + i) % 4]).collect();
          if reversed {
            points.reverse();
          }
          let quad = Area::from_points(&points).to_quad().expect("expected four corners");
          let found: Vec<[f32; 2]> = quad.corners().iter().map(|corner| [corner.x, corner.y]).collect();
          assert_eq!(found, shape, "start {start}, reversed {reversed}");
        }
      }
    }
  }

  #[test]
  fn shapes_without_four_straight_convex_corners_are_not_quads() {
    let shapes = [
      ("a circle", Area::circle((50, 50), 30)),
      ("a triangle", Area::from_points(&[[10.0, 10.0], [90.0, 20.0], [50.0, 80.0]])),
      ("a dented shape", Area::from_points(&[[10.0, 10.0], [90.0, 10.0], [50.0, 90.0], [50.0, 30.0]])),
      ("a line", Area::from_points(&[[10.0, 10.0], [20.0, 20.0], [30.0, 30.0], [40.0, 40.0]])),
    ];
    for (name, area) in shapes {
      assert!(area.to_quad().is_none(), "{name}");
    }
  }

  #[test]
  fn bounds_are_the_upright_box_around_the_corners() {
    let quad = Quad::new((50, 40), (150, 60), (140, 150), (60, 130));
    assert_eq!(quad.bounds(), Rect::new((50, 40), (100, 110)));
  }

  /// The corners of a flat `p_aspect` wide, 1 tall rectangle 6 units in front of a camera turned by the given angles
  /// in degrees, with a lens of `p_focal` pixels and the image `p_size` in pixels.
  fn photographed_rectangle(p_aspect: f64, p_angles: [f64; 3], p_focal: f64, p_size: (f64, f64)) -> Quad {
    let [pitch, yaw, roll] = p_angles.map(f64::to_radians);
    let rotate = |[x, y, z]: [f64; 3]| {
      let (y, z) = (y * pitch.cos() - z * pitch.sin(), y * pitch.sin() + z * pitch.cos());
      let (x, z) = (x * yaw.cos() + z * yaw.sin(), -x * yaw.sin() + z * yaw.cos());
      [x * roll.cos() - y * roll.sin(), x * roll.sin() + y * roll.cos(), z]
    };
    let (half_width, half_height) = (p_aspect / 2.0, 0.5);
    let [top_left, top_right, bottom_right, bottom_left] = [
      (-half_width, -half_height),
      (half_width, -half_height),
      (half_width, half_height),
      (-half_width, half_height),
    ]
    .map(|(x, y)| {
      let [x, y, z] = rotate([x, y, 0.0]);
      (p_focal * x / (z + 6.0) + p_size.0 / 2.0, p_focal * y / (z + 6.0) + p_size.1 / 2.0)
    });
    Quad::new(top_left, top_right, bottom_right, bottom_left)
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
      let found = photographed_rectangle(aspect, angles, focal, (1000.0, 800.0)).flattened_aspect().unwrap();
      // Without knowing the lens it cannot be exact, but it must be far closer than the photo's own proportions.
      assert!((found / aspect - 1.0).abs() < 0.13, "{aspect} came out as {found:.3} for {angles:?}");
    }
  }

  #[test]
  fn an_upright_rectangle_keeps_its_shape() {
    let quad = Quad::rect((10, 20), (80, 40));
    assert!((quad.flattened_aspect().unwrap() - 2.0).abs() < 1e-9);
    assert_eq!(quad.flattened_size(), Size::new(80, 40));
  }

  #[test]
  fn a_building_shot_from_its_foot_is_not_squashed() {
    // Level top and bottom, with the sides leaning in sharply towards the top. Kept to the quad's height it would be
    // 396 wide and 294 tall, but the far end, the top, is foreshortened in the photo, so it has to come out taller.
    let size = Quad::new((93, 42), (332, 42), (405, 336), (9, 336)).flattened_size();
    assert_eq!(size.width, 396.0);
    assert!(size.height > 396.0 / 0.9, "{size:?}");
  }
}
