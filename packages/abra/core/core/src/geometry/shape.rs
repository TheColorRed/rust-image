//! Predefined shapes. Create one with [`Area::shape`].

use crate::Area;

// The control-point distance that makes a cubic Bezier approximate a quarter circle.
const K: f32 = 0.5523;

/// A predefined shape, drawn inside a 100x100 box at the origin. Use [`Area::fit`] to size it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
  /// A heart made of cubic Bezier curves. It is 120 tall, so it overflows the box at the bottom.
  Heart,
  /// A five-pointed star.
  Star,
  /// A regular polygon with the given number of sides, with a corner at the top. Fewer than three sides produces
  /// an empty area.
  Polygon(usize),
  /// An equilateral triangle with a corner at the top.
  Triangle,
  /// A cut gem: flat on top, with wide shoulders that angle out to the full width and then in to a point at the bottom. It
  /// is about 100 wide and 80 tall.
  Diamond,
  /// An arrow pointing upwards, with a flat base and a pointed triangle at the top wider than the base.
  Arrow,
  /// A half moon shape, with the curved side on the left and the flat side on the right.
  HalfMoon,
  /// A crescent moon shape, with the curved outer side on the left the concave side on the right.
  CrescentMoon,
  /// A cross shape, with equal arms extending from the center.
  Cross,
  /// A bow tie: two triangles, tall at the outer ends and meeting in a narrow waist at the center.
  BowTie,
}

impl Shape {
  /// Builds the shape as an area.
  pub fn to_area(&self) -> Area {
    let mut area = Area::new();
    match *self {
      Shape::Heart => {
        area
          .move_to((50.0, 15.0))
          .cubic_to((35.0, 0.0), (0.0, 0.0), (0.0, 37.5))
          .cubic_to((0.0, 75.0), (25.0, 95.0), (50.0, 120.0))
          .cubic_to((75.0, 95.0), (100.0, 75.0), (100.0, 37.5))
          .cubic_to((100.0, 0.0), (65.0, 0.0), (50.0, 15.0));
      }
      Shape::Diamond => {
        area
          .move_to((20.0, 0.0))
          .line_to((80.0, 0.0))
          .line_to((100.0, 20.0))
          .line_to((50.0, 80.0))
          .line_to((0.0, 20.0))
          .line_to((20.0, 0.0));
      }
      Shape::BowTie => {
        area
          .move_to((0.0, 15.0))
          .line_to((50.0, 40.0))
          .line_to((100.0, 15.0))
          .line_to((100.0, 85.0))
          .line_to((50.0, 60.0))
          .line_to((0.0, 85.0))
          .line_to((0.0, 15.0));
      }
      Shape::Cross => {
        area
          .move_to((40.0, 0.0))
          .line_to((60.0, 0.0))
          .line_to((60.0, 40.0))
          .line_to((100.0, 40.0))
          .line_to((100.0, 60.0))
          .line_to((60.0, 60.0))
          .line_to((60.0, 100.0))
          .line_to((40.0, 100.0))
          .line_to((40.0, 60.0))
          .line_to((0.0, 60.0))
          .line_to((0.0, 40.0))
          .line_to((40.0, 40.0))
          .line_to((40.0, 0.0));
      }
      Shape::Arrow => {
        area
          .move_to((50.0, 0.0))
          .line_to((100.0, 50.0))
          .line_to((75.0, 50.0))
          .line_to((75.0, 100.0))
          .line_to((25.0, 100.0))
          .line_to((25.0, 50.0))
          .line_to((0.0, 50.0))
          .line_to((50.0, 0.0));
      }
      Shape::Star => {
        area.move_to((50.0, 0.0));
        for point in [
          (61.8, 35.1),
          (100.0, 38.2),
          (69.1, 61.8),
          (80.9, 100.0),
          (50.0, 76.4),
          (19.1, 100.0),
          (30.9, 61.8),
          (0.0, 38.2),
          (38.2, 35.1),
          (50.0, 0.0),
        ] {
          area.line_to(point);
        }
      }
      Shape::HalfMoon => {
        let radius = 50.0f32;
        let center = (radius, radius);
        area.move_to((center.0, center.1 - radius));
        area
          .cubic_to(
            (center.0 - K * radius, center.1 - radius),
            (center.0 - radius, center.1 - K * radius),
            (center.0 - radius, center.1),
          )
          .cubic_to(
            (center.0 - radius, center.1 + K * radius),
            (center.0 - K * radius, center.1 + radius),
            (center.0, center.1 + radius),
          )
          .line_to((center.0, center.1 - radius));
      }
      Shape::CrescentMoon => {
        let radius = 50.0f32;
        let inner = radius * 0.6;
        let center = (radius, radius);
        area.move_to((center.0, center.1 - radius));
        area
          .cubic_to(
            (center.0 - K * radius, center.1 - radius),
            (center.0 - radius, center.1 - K * radius),
            (center.0 - radius, center.1),
          )
          .cubic_to(
            (center.0 - radius, center.1 + K * radius),
            (center.0 - K * radius, center.1 + radius),
            (center.0, center.1 + radius),
          )
          .cubic_to(
            (center.0 - K * inner, center.1 + radius),
            (center.0 - inner, center.1 + K * radius),
            (center.0 - inner, center.1),
          )
          .cubic_to(
            (center.0 - inner, center.1 - K * radius),
            (center.0 - K * inner, center.1 - radius),
            (center.0, center.1 - radius),
          );
      }
      Shape::Triangle => {
        let radius = 50.0f32;
        let corner = |i: usize| {
          let angle = (i as f32 * 360.0 / 3.0 - 90.0).to_radians();
          (radius * angle.cos() + radius, radius * angle.sin() + radius)
        };
        area.move_to(corner(0));
        for i in 1..=3 {
          area.line_to(corner(i % 3));
        }
      }
      Shape::Polygon(sides) if sides >= 3 => {
        let radius = 50.0f32;
        let corner = |i: usize| {
          // Start from the top.
          let angle = (i as f32 * 360.0 / sides as f32 - 90.0).to_radians();
          (radius * angle.cos() + radius, radius * angle.sin() + radius)
        };
        area.move_to(corner(0));
        for i in 1..=sides {
          // The last corner repeats the first to close the outline.
          area.line_to(corner(i % sides));
        }
      }
      Shape::Polygon(_) => {}
    }
    area
  }
}

impl From<Shape> for Area {
  fn from(p_shape: Shape) -> Self {
    p_shape.to_area()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn polygon_has_one_segment_per_side_and_closes() {
    let hexagon = Area::shape(Shape::Polygon(6));
    assert_eq!(hexagon.segments().len(), 6);
    assert!(hexagon.start().distance_to(hexagon.end()) < 1e-4);
    assert!(Area::shape(Shape::Polygon(2)).segments().is_empty());
  }
}
