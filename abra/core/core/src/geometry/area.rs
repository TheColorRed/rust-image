use std::fmt::Display;
use std::ops::{Deref, DerefMut};

use crate::{AspectRatio, Image, Path, PointF, Shape, Size};

#[derive(Clone, Debug, Default)]
/// An area represents a closed shape made of lines and curves.
/// Areas are used for drawing, filling, effects, and more.
/// An area is a closed shape. Use a Path for open shapes.
///
/// An area derefs to its outline [`Path`], so every path method (`move_to`, `line_to`, `flatten`, `bounds`, ...)
/// can be called on it directly.
pub struct Area {
  /// The underlying path outline that defines this closed area.
  pub path: Path,
  /// The feather amount for the area edges.
  pub feather: u32,
}

impl Area {
  /// Creates a new empty area.
  pub fn new() -> Area {
    Area::default()
  }
  /// Creates a rectangular area that matches the dimensions of the given image.
  /// - `p_image`: The image to create the area from.
  pub fn new_from_image(p_image: &Image) -> Area {
    let (width, height) = p_image.dimensions::<u32>();
    Area::rect((0.0, 0.0), (width as f32, height as f32))
  }
  /// Creates a rectangular area.
  /// - `p_origin`: The top-left corner of the rectangle.
  /// - `p_size`: The size (width, height) of the rectangle.
  pub fn rect(p_origin: impl Into<PointF>, p_size: impl Into<Size>) -> Area {
    let origin: PointF = p_origin.into();
    let size: Size = p_size.into();
    let mut area = Area::new();
    area
      .move_to(origin)
      .line_to((origin.x + size.width, origin.y))
      .line_to((origin.x + size.width, origin.y + size.height))
      .line_to((origin.x, origin.y + size.height));
    area
  }
  /// Creates a circular area.
  /// - `p_center`: The center point of the circle.
  /// - `p_radius`: The radius of the circle.
  pub fn circle(p_center: impl Into<PointF>, p_radius: impl Into<f64>) -> Area {
    let radius = p_radius.into() as f32;
    Area::ellipse(p_center, (radius * 2.0, radius * 2.0))
  }
  /// Creates an elliptical area.
  /// - `p_center`: The center point of the ellipse.
  /// - `p_size`: The size (width, height) of the ellipse.
  pub fn ellipse(p_center: impl Into<PointF>, p_size: impl Into<Size>) -> Area {
    let center = p_center.into();
    let size = p_size.into();
    let rx = size.width / 2.0;
    let ry = size.height / 2.0;

    // Approximate ellipse with cubic Bezier curves (4 arcs)
    let kappa = 0.5522847498; // magic number for circle approximation
    let ox = rx * kappa;
    let oy = ry * kappa;

    let mut area = Area::new();
    area
      .move_to((center.x, center.y - ry))
      .cubic_to((center.x + ox, center.y - ry), (center.x + rx, center.y - oy), (center.x + rx, center.y))
      .cubic_to((center.x + rx, center.y + oy), (center.x + ox, center.y + ry), (center.x, center.y + ry))
      .cubic_to((center.x - ox, center.y + ry), (center.x - rx, center.y + oy), (center.x - rx, center.y))
      .cubic_to((center.x - rx, center.y - oy), (center.x - ox, center.y - ry), (center.x, center.y - ry));
    area
  }
  /// Creates a predefined shape inside a 100x100 box at the origin. Use [`Area::fit`] to size it.
  /// - `p_shape`: The shape to create.
  ///
  /// ```ignore
  /// let star = Area::shape(Shape::Star).fit((200, 200), AspectRatio::meet());
  /// ```
  pub fn shape(p_shape: Shape) -> Area {
    p_shape.to_area()
  }
  /// Creates an area from a list of points.
  /// - `p_points`: The list of points defining the area.
  pub fn from_points(p_points: &[[f32; 2]]) -> Area {
    let mut area = Area::new();
    if let Some(first) = p_points.first() {
      area.move_to((first[0], first[1]));
      for point in &p_points[1..] {
        area.line_to((point[0], point[1]));
      }
    }
    area
  }
  pub fn from_size(p_size: impl Into<Size>) -> Area {
    let size = p_size.into();
    Area::from_points(&[
      [0.0, 0.0],
      [size.width, 0.0],
      [size.width, size.height],
      [0.0, size.height],
    ])
  }
  /// Sets the feather amount for the area edges.
  /// - `p_feather`: The feather radius in pixels.
  pub fn with_feather(mut self, p_feather: u32) -> Self {
    self.feather = p_feather;
    self
  }
  /// Gets the feather amount for this Area.
  pub fn feather(&self) -> u32 {
    self.feather
  }
  /// Determines if a point is inside the area using the ray-casting algorithm.
  ///
  /// This flattens the outline on every call. To test many points, flatten once with
  /// [`Path::flatten`] and use [`polygon_contains`].
  /// - `p_point`: The point to test.
  pub fn contains(&self, p_point: impl Into<PointF>) -> bool {
    polygon_contains(&self.path.flatten(0.5), p_point.into())
  }
  /// Scales this area from its own bounds into a box of `p_size`. See [`Path::fit`].
  /// - `p_size`: The target size.
  /// - `p_aspect_ratio`: How to scale when the shapes differ.
  pub fn fit(&self, p_size: impl Into<Size>, p_aspect_ratio: AspectRatio) -> Area {
    Area {
      path: self.path.fit(p_size, p_aspect_ratio),
      feather: self.feather,
    }
  }
}

/// Whether `p_point` is inside the closed polygon through `p_points`, using the even-odd ray-casting rule.
/// An empty polygon contains nothing.
pub fn polygon_contains(p_points: &[PointF], p_point: PointF) -> bool {
  let Some(mut previous) = p_points.last().copied() else {
    return false;
  };
  let mut inside = false;
  for &current in p_points {
    if (current.y > p_point.y) != (previous.y > p_point.y)
      && p_point.x < (previous.x - current.x) * (p_point.y - current.y) / (previous.y - current.y) + current.x
    {
      inside = !inside;
    }
    previous = current;
  }
  inside
}

impl Deref for Area {
  type Target = Path;
  fn deref(&self) -> &Path {
    &self.path
  }
}

impl DerefMut for Area {
  fn deref_mut(&mut self) -> &mut Path {
    &mut self.path
  }
}

impl Display for Area {
  /// Displays the area as a string.
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(p_f, "Area(start: {}, segments: {})", self.path.start(), self.path.segments().len())
  }
}

impl From<Area> for Path {
  fn from(p_area: Area) -> Self {
    p_area.path
  }
}

impl From<Path> for Area {
  fn from(p_path: Path) -> Self {
    Area {
      path: p_path,
      feather: 0,
    }
  }
}

impl From<&Area> for Area {
  fn from(p_area: &Area) -> Self {
    p_area.clone()
  }
}

impl From<Image> for Area {
  fn from(p_image: Image) -> Self {
    let (width, height) = p_image.dimensions::<u32>();
    let mut path = Path::new();
    path.move_to((0, 0)).line_to((width, 0)).line_to((width, height)).line_to((0, height));
    Area { path, feather: 0 }
  }
}

impl From<&Image> for Area {
  fn from(p_image: &Image) -> Self {
    let (width, height) = p_image.dimensions::<u32>();
    let mut path = Path::new();
    path.move_to((0, 0)).line_to((width, 0)).line_to((width, height)).line_to((0, height));
    Area { path, feather: 0 }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn contains_tests_inside_and_outside() {
    let area = Area::rect((0, 0), (10, 10));
    assert!(area.contains((5, 5)));
    assert!(!area.contains((15, 5)));
    assert!(!Area::new().contains((0, 0)));
    assert!(!polygon_contains(&[], PointF::zero()));
  }

  #[test]
  fn path_methods_are_available_on_areas() {
    let area = Area::rect((2, 3), (10, 20));
    assert_eq!(area.bounds().edges::<i32>(), (2, 3, 12, 23));
    let fitted = area.fit((5, 5), AspectRatio::meet());
    assert_eq!(fitted.bounds().size(), Size::new(2.5, 5));
  }
}
