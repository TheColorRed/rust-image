//! Predefined shapes. Create one with [`Area::shape`].

use crate::Area;

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
