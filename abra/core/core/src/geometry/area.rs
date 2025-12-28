use std::{
  fmt::Display,
  ops::{Add, Sub},
};

use crate::{AspectRatio, FromF32, Image, Path, Point, PointF, Segment, Size, ViewBox};
// use crate::{
//   Image,
// };

#[derive(Clone, Debug)]
/// An area represents a closed shape made of lines and curves.
/// Areas are used for drawing, filling, effects, and more.
/// An area is a closed shape. Use a Path for open shapes.
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
    let mut path = Path::new();
    path
      .move_to(origin)
      .line_to((origin.x + size.width, origin.y))
      .line_to((origin.x + size.width, origin.y + size.height))
      .line_to((origin.x, origin.y + size.height));
    Area { path, feather: 0 }
  }
  /// Creates a circular area.
  /// - `p_center`: The center point of the circle.
  /// - `p_radius`: The radius of the circle.
  pub fn circle(p_center: impl Into<PointF>, p_radius: impl Into<f64>) -> Area {
    let center = p_center.into();
    let radius = p_radius.into() as f32;
    Area::ellipse(center, (radius * 2.0, radius * 2.0))
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

    let mut path = Path::new();
    path
      .move_to((center.x, center.y - ry))
      .cubic_to((center.x + ox, center.y - ry), (center.x + rx, center.y - oy), (center.x + rx, center.y))
      .cubic_to((center.x + rx, center.y + oy), (center.x + ox, center.y + ry), (center.x, center.y + ry))
      .cubic_to((center.x - ox, center.y + ry), (center.x - rx, center.y + oy), (center.x - rx, center.y))
      .cubic_to((center.x - rx, center.y - oy), (center.x - ox, center.y - ry), (center.x, center.y - ry));

    Area { path, feather: 0 }
  }
  /// Creates an area from a list of points.
  /// - `p_points`: The list of points defining the area.
  pub fn from_points(p_points: &[[f32; 2]]) -> Area {
    let mut path = Path::new();
    if let Some(first) = p_points.first() {
      path.move_to((first[0], first[1]));
      for point in &p_points[1..] {
        path.line_to((point[0], point[1]));
      }
    }
    Area { path, feather: 0 }
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
  /// - `p_point`: The point to test.
  pub fn contains(&self, p_point: impl Into<PointF>) -> bool {
    let point = p_point.into();
    let pts = self.path.flatten(0.5);
    let mut inside = false;
    let n = pts.len();
    let mut j = n - 1;

    for i in 0..n {
      let pi = pts[i];
      let pj = pts[j];
      if (pi.y > point.y) != (pj.y > point.y)
        && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y + 0.00001) + pi.x)
      {
        inside = !inside;
      }
      j = i;
    }

    inside
  }
  /// Sets the starting point of the area's (move to).
  /// - `p_start`: The starting point.
  pub fn move_to(&mut self, p_start: impl Into<PointF>) -> &mut Self {
    self.path.move_to(p_start);
    self
  }
  /// Adds a line to the next point in the area's.
  /// - `p_to`: The next point to add to the area.
  pub fn line_to(&mut self, p_to: impl Into<PointF>) -> &mut Self {
    self.path.line_to(p_to);
    self
  }
  /// Adds a quadratic Bezier segment to the area's.
  /// - `p_ctrl`: The control point for the curve.
  /// - `p_to`: The end point of the curve.
  pub fn quad_to(&mut self, p_ctrl: impl Into<PointF>, p_to: impl Into<PointF>) -> &mut Self {
    self.path.quad_to(p_ctrl, p_to);
    self
  }
  /// Adds a cubic Bezier segment to the area's.
  /// - `p_ctrl1`: The first control point for the curve.
  /// - `p_ctrl2`: The second control point for the curve.
  /// - `p_to`: The end point of the curve.
  pub fn cubic_to(
    &mut self, p_ctrl1: impl Into<PointF>, p_ctrl2: impl Into<PointF>, p_to: impl Into<PointF>,
  ) -> &mut Self {
    self.path.cubic_to(p_ctrl1, p_ctrl2, p_to);
    self
  }
  /// Gets the starting point of the area's path.
  pub fn start(&self) -> PointF {
    self.path.start()
  }
  /// Gets the ending point of the area's path.
  pub fn end(&self) -> PointF {
    self.path.end()
  }
  /// Gets the segments that make up the area's path.
  pub fn segments(&self) -> &[Segment] {
    self.path.segments()
  }
  /// Gets all points in the area's path as a flat list of PointF.
  pub fn points(&self) -> Vec<PointF> {
    self.path.points()
  }
  /// Gets the point at parameter t (0 to 1) along the area's path.
  pub fn point_at(&self, p_t: f32) -> PointF {
    self.path.point_at(p_t)
  }
  /// Gets the point at parameter t within a specific segment of the area's path.
  /// - `p_segment_idx`: The index of the segment.
  /// - `p_t`: The parameter t (0 to 1) within that segment.
  pub fn point_at_segment(&self, p_segment_idx: usize, p_t: f32) -> PointF {
    self.path.point_at_segment(p_segment_idx, p_t)
  }
  /// Flattens the area's path into a polyline (list of points) with the given tolerance.
  /// - `p_tolerance`: The maximum allowed deviation from the original path.
  pub fn flatten(&self, p_tolerance: f32) -> Vec<PointF> {
    self.path.flatten(p_tolerance)
  }
  /// Gets an approximate length of the area's outline path.
  pub fn length(&self) -> f32 {
    self.path.length()
  }
  /// Gets the bounding box of the area's outline as (`min_x`, `min_y`, `max_x`, `max_y`).
  pub fn bounds<S: FromF32>(&self) -> (S, S, S, S) {
    let (min_x, min_y, max_x, max_y) = self.path.bounds();
    (S::from_f32(min_x), S::from_f32(min_y), S::from_f32(max_x), S::from_f32(max_y))
  }
  /// Converts the area's path to a list of integer points (for raster operations).
  /// - `p_tolerance`: The tolerance for flattening curves to points.
  pub fn to_points(&self, p_tolerance: f32) -> Vec<Point> {
    self.path.to_points(p_tolerance)
  }
  /// Finds the closest position on the area's path to the given coordinates and returns t.
  /// - `p_x`: The x-coordinate of the point.
  /// - `p_y`: The y-coordinate of the point.
  pub fn closest_time(&self, p_x: f32, p_y: f32) -> f32 {
    self.path.closest_time(p_x, p_y)
  }
  /// Transforms this area's path to fit within a viewport using a ViewBox.
  /// - `p_viewbox`: The viewbox defining the area to fit.
  /// - `p_viewport_width`: The width of the viewport.
  /// - `p_viewport_height`: The height of the viewport.
  /// - `p_aspect_ratio`: The aspect ratio policy to use.
  pub fn transform_to_viewport(
    &self, p_viewbox: &ViewBox, p_viewport_width: f32, p_viewport_height: f32, p_aspect_ratio: AspectRatio,
  ) -> Path {
    self
      .path
      .transform_to_viewport(p_viewbox, p_viewport_width, p_viewport_height, p_aspect_ratio)
  }
  /// Convenience method to create a ViewBox from the area's bounds.
  pub fn to_viewbox(&self) -> ViewBox {
    self.path.to_viewbox()
  }
  /// Fits this area's path into the given viewport size preserving aspect ratio (Meet).
  /// - `p_size`: The target size to fit into.
  pub fn fit(&self, p_size: impl Into<Size>) -> Area {
    self.path.fit(p_size.into()).into()
  }
  /// Fits this area's path into a square viewport of the given size (Meet).
  /// - `p_size`: The size of the square viewport.
  pub fn fit_square(&self, p_size: impl Into<f32>) -> Area {
    self.path.fit_square(p_size.into()).into()
  }
  /// Fits this area's path into the given viewport using a specific aspect ratio policy.
  /// - `p_size`: The target size to fit into.
  /// - `p_aspect_ratio`: The aspect ratio policy to use.
  pub fn fit_with_aspect(&self, p_size: impl Into<Size>, p_aspect_ratio: AspectRatio) -> Area {
    let size = p_size.into();
    self
      .path
      .fit_with_aspect(size.width, size.height, p_aspect_ratio)
      .into()
  }
  /// Stretches this area's path non-uniformly to fill the viewport (no aspect ratio preservation).
  /// - `p_size`: The target size to stretch into.
  pub fn stretch(&self, p_size: impl Into<Size>) -> Area {
    self.path.stretch(p_size.into()).into()
  }
  /// Scales this area's path uniformly to cover the viewport (may crop) using Slice.
  /// - `p_size`: The target size to cover.
  pub fn cover(&self, p_size: impl Into<Size>) -> Area {
    self.path.cover(p_size.into()).into()
  }
}

impl Display for Area {
  /// Displays the area as a string.
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Area(start: {}, segments: {})", self.path.start(), self.path.segments().len())
  }
}

impl Default for Area {
  fn default() -> Self {
    Area {
      path: Path::default(),
      feather: 0,
    }
  }
}

impl Into<Path> for Area {
  fn into(self) -> Path {
    self.path
  }
}

impl From<Path> for Area {
  fn from(path: Path) -> Self {
    Area { path, feather: 0 }
  }
}

impl From<&Area> for Area {
  fn from(area: &Area) -> Self {
    area.clone()
  }
}

// impl From<Image> for Area {
//   fn from(image: Image) -> Self {
//     let (width, height) = image.dimensions::<u32>();
//     Area::rect((0.0, 0.0), (width as f32, height as f32))
//   }
// }

impl Sub<Area> for Area {
  type Output = Area;

  fn sub(self, _rhs: Area) -> Self::Output {
    Area {
      path: self.path.clone(),
      feather: self.feather,
    }
  }
}

impl<T: Into<f32>> Sub<T> for Area {
  type Output = Area;

  fn sub(self, rhs: T) -> Self::Output {
    let rhs = rhs.into();
    println!("Subtraction {}", rhs);
    Area {
      path: self.path.clone(),
      feather: self.feather,
    }
  }
}

impl Add<Area> for Area {
  type Output = Area;

  fn add(self, _rhs: Area) -> Self::Output {
    Area {
      path: self.path.clone(),
      feather: self.feather,
    }
  }
}

impl Add<f32> for Area {
  type Output = Area;

  fn add(self, _rhs: f32) -> Self::Output {
    Area {
      path: self.path.clone(),
      feather: self.feather,
    }
  }
}
