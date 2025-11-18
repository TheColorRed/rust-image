use core::{
  fmt::Display,
  ops::{Add, Sub},
};

use crate::{
  Image,
  color::Fill,
  draw::core::{PolygonCoverage, Rasterizer, SampleGrid, SourceOverCompositor, shader_from_fill},
  geometry::{AspectRatio, Path, Point, PointF, Segment, Size, ViewBox},
};

#[derive(Clone, Debug)]
/// An area represents a closed shape made of lines and curves.
/// Areas are used for drawing, filling, effects, and more.
/// An area is a closed shape. Use a Path for open shapes.
pub struct Area {
  /// The underlying path outline that defines this closed area.
  pub path: Path,
}

impl Area {
  /// Creates a new empty area.
  pub fn new() -> Area {
    Area::default()
  }
  /// Creates a rectangular area.
  pub fn rect(p_origin: impl Into<PointF>, p_size: impl Into<Size>) -> Area {
    let origin: PointF = p_origin.into();
    let size: Size = p_size.into();
    let mut path = Path::new();
    path
      .with_move_to(origin)
      .with_line_to((origin.x + size.width, origin.y))
      .with_line_to((origin.x + size.width, origin.y + size.height))
      .with_line_to((origin.x, origin.y + size.height));
    Area { path }
  }
  /// Creates a circular area.
  /// - `center`: The center point of the circle.
  /// - `radius`: The radius of the circle.
  pub fn circle(p_center: impl Into<PointF>, p_radius: impl Into<f64>) -> Area {
    let center = p_center.into();
    let radius = p_radius.into() as f32;
    Area::ellipse(center, (radius * 2.0, radius * 2.0))
  }
  /// Creates an elliptical area.
  /// - `center`: The center point of the ellipse.
  /// - `size`: The size (width, height) of the ellipse.
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
      .with_move_to((center.x, center.y - ry))
      .with_cubic_to((center.x + ox, center.y - ry), (center.x + rx, center.y - oy), (center.x + rx, center.y))
      .with_cubic_to((center.x + rx, center.y + oy), (center.x + ox, center.y + ry), (center.x, center.y + ry))
      .with_cubic_to((center.x - ox, center.y + ry), (center.x - rx, center.y + oy), (center.x - rx, center.y))
      .with_cubic_to((center.x - rx, center.y - oy), (center.x - ox, center.y - ry), (center.x, center.y - ry));

    Area { path }
  }
  /// Fills the area with the specified fill style.
  /// - `fill`: The fill style to apply to the area.
  pub fn fill(&self, p_fill: impl Into<Fill>) -> Image {
    let fill = p_fill.into();
    let (min_x, min_y, max_x, max_y) = self.bounds();
    let width = (max_x - min_x).ceil();
    let height = (max_y - min_y).ceil();

    if width <= 0.0 || height <= 0.0 {
      return Image::new(1, 1);
    }

    let mut image = Image::new(width as u32, height as u32);

    // Flatten the path and translate to image-local coordinates
    let tolerance = 0.5;
    let flattened: Vec<PointF> = self
      .path
      .flatten(tolerance)
      .iter()
      .map(|p| PointF::new(p.x - min_x, p.y - min_y))
      .collect();

    // Build coverage mask
    let coverage = PolygonCoverage::new(flattened);

    // Build shader from fill
    let shader = shader_from_fill(&fill);

    // Use source-over compositing
    let compositor = SourceOverCompositor;

    // Use anti-aliasing level from image
    let sample_grid = SampleGrid::from_aa_level(image.anti_aliasing_level);

    // Rasterize
    let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, sample_grid);
    rasterizer.rasterize(&mut image);

    image
  }
  /// Determines if a point is inside the area using the ray-casting algorithm.
  /// - `point`: The point to test.
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

  /// Sets the starting point of the area's outline (move to).
  pub fn with_move_to(&mut self, p_start: impl Into<PointF>) -> &mut Self {
    self.path.with_move_to(p_start);
    self
  }
  /// Adds a straight line segment to the area's outline.
  pub fn with_line_to(&mut self, p_to: impl Into<PointF>) -> &mut Self {
    self.path.with_line_to(p_to);
    self
  }
  /// Adds a quadratic Bezier segment to the area's outline.
  pub fn with_quad_to(&mut self, p_ctrl: impl Into<PointF>, p_to: impl Into<PointF>) -> &mut Self {
    self.path.with_quad_to(p_ctrl, p_to);
    self
  }
  /// Adds a cubic Bezier segment to the area's outline.
  pub fn with_cubic_to(
    &mut self, p_ctrl1: impl Into<PointF>, p_ctrl2: impl Into<PointF>, p_to: impl Into<PointF>,
  ) -> &mut Self {
    self.path.with_cubic_to(p_ctrl1, p_ctrl2, p_to);
    self
  }
  /// Returns the starting point of the area's path.
  pub fn start(&self) -> PointF {
    self.path.start()
  }
  /// Returns the segments that make up the area's path.
  pub fn segments(&self) -> &[Segment] {
    self.path.segments()
  }
  /// Returns all points in the area's path as a flat list of PointF.
  pub fn points(&self) -> Vec<PointF> {
    self.path.points()
  }
  /// Returns the point at parameter t (0 to 1) along the area's path.
  pub fn point_at(&self, p_t: f32) -> PointF {
    self.path.point_at(p_t)
  }
  /// Returns the point at parameter t within a specific segment of the area's path.
  pub fn point_at_segment(&self, p_segment_idx: usize, p_t: f32) -> PointF {
    self.path.point_at_segment(p_segment_idx, p_t)
  }
  /// Flattens the area's path into a polyline (list of points) with the given tolerance.
  pub fn flatten(&self, p_tolerance: f32) -> Vec<PointF> {
    self.path.flatten(p_tolerance)
  }
  /// Returns an approximate length of the area's outline path.
  pub fn length(&self) -> f32 {
    self.path.length()
  }
  /// Returns the bounding box of the area's outline as (min_x, min_y, max_x, max_y).
  pub fn bounds(&self) -> (f32, f32, f32, f32) {
    self.path.bounds()
  }
  /// Converts the area's path to a list of integer points (for raster operations).
  pub fn to_points(&self, p_tolerance: f32) -> Vec<Point> {
    self.path.to_points(p_tolerance)
  }
  /// Finds the closest position on the area's path to the given coordinates and returns t.
  pub fn closest_time(&self, p_x: f32, p_y: f32) -> f32 {
    self.path.closest_time(p_x, p_y)
  }
  /// Transforms this area's path to fit within a viewport using a ViewBox.
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
  pub fn fit(&self, p_size: impl Into<Size>) -> Area {
    self.path.fit(p_size.into()).into()
  }
  /// Fits this area's path into a square viewport of the given size (Meet).
  pub fn fit_square(&self, p_size: impl Into<f32>) -> Area {
    self.path.fit_square(p_size.into()).into()
  }
  /// Fits this area's path into the given viewport using a specific aspect ratio policy.
  pub fn fit_with_aspect(&self, p_size: impl Into<Size>, p_aspect_ratio: AspectRatio) -> Area {
    let size = p_size.into();
    self
      .path
      .fit_with_aspect(size.width, size.height, p_aspect_ratio)
      .into()
  }
  /// Stretches this area's path non-uniformly to fill the viewport (no aspect ratio preservation).
  pub fn stretch(&self, p_size: impl Into<Size>) -> Area {
    self.path.stretch(p_size.into()).into()
  }
  /// Scales this area's path uniformly to cover the viewport (may crop) using Slice.
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
    Area { path: Path::default() }
  }
}

impl Into<Path> for Area {
  fn into(self) -> Path {
    self.path
  }
}

impl From<Path> for Area {
  fn from(path: Path) -> Self {
    Area { path }
  }
}

impl From<Image> for Area {
  fn from(image: Image) -> Self {
    let (width, height) = image.dimensions::<u32>();
    Area::rect((0.0, 0.0), (width as f32, height as f32))
  }
}

impl Sub<Area> for Area {
  type Output = Area;

  fn sub(self, rhs: Area) -> Self::Output {
    Area {
      path: self.path.clone(),
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
    }
  }
}

impl Add<Area> for Area {
  type Output = Area;

  fn add(self, rhs: Area) -> Self::Output {
    Area {
      path: self.path.clone(),
    }
  }
}

impl Add<f32> for Area {
  type Output = Area;

  fn add(self, rhs: f32) -> Self::Output {
    Area {
      path: self.path.clone(),
    }
  }
}
