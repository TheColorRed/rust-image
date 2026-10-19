//! The `geometry` module of `abra-core`: points, sizes and rectangles, lines, paths, areas, shapes, strokes and perspective
//! transforms.
//!
//! The library's plain data types are records, whose fields a caller reads and writes directly: `PointF` (in the blend file),
//! `Size` (in the image file), `Rect`, `Quad` and `AspectRatio`. Rust only lets a type's own crate add functions to it, so the
//! functions of these types are free functions named for the type, taking it first: `point_f_dot(a, b)` is `a.dot(b)`, and
//! `rect_intersect(rect, other)` is `rect.intersect(other)`. Types that change in place or hide their fields are handles with
//! the library's functions: `Path`, `Area`, `LineSegment`, `Homography` and the two strokes.
//!
//! What is not exported, and why:
//! - `Area` derefs to its `Path` in Rust, so it has every path function here too, written once by `path_methods!`.
//! - `PathStroke` and `AreaStroke` borrow the shape they stroke, which a handle cannot hold, so each holds a copy of it and the
//!   settings, and `Path::stroke` and `Area::stroke` start one. Their `From` conversions into a `Path` or an `Area` are
//!   `to_path` and `to_area`.
//! - `f32 * PointF`, `f32 * Size` and `f32 / Size` are the same as `point_f_mul`, `size_mul` and a call with the sides swapped.
//! - `Shape::Polygon` counts its sides with a `u32` here, and `Shape::to_area` is `Area::shape`.
//! - `From<f32> for Path` (an angle that gradients read) is `Path::from_degrees`, and `Into<Path> for LineSegment` is
//!   `LineSegment::to_path`.
//! - The `Display` of `Path` and `Area`, and the conversions between areas, paths, images and rectangles that Rust callers use.
//!
//! A tuple of the library is an array here, and an array of the wrong length panics, as the library does for input it cannot
//! use.

use std::sync::{Arc, RwLock};

use abra::abra_core::{
  self, Alignment, AspectRatio, LineCap, LineJoin, Orientation, PointF, PreserveAspectRatio, Quad, Rect, Segment, Size,
};

use crate::Image;

// A rectangle: the top left corner and the size.
#[uniffi::remote(Record)]
pub struct Rect {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

// A shape with four corners that need not be upright: top left, top right, bottom right, bottom left.
#[uniffi::remote(Record)]
pub struct Quad {
  pub top_left: PointF,
  pub top_right: PointF,
  pub bottom_right: PointF,
  pub bottom_left: PointF,
}

// How content is scaled when the source and target rectangles have different aspect ratios.
#[uniffi::remote(Enum)]
pub enum PreserveAspectRatio {
  None,
  Meet,
  Slice,
}

// Where content sits when aspect ratio is preserved.
#[uniffi::remote(Enum)]
pub enum Alignment {
  Min,
  Mid,
  Max,
}

// Complete aspect ratio preservation specification.
#[uniffi::remote(Record)]
pub struct AspectRatio {
  pub mode: PreserveAspectRatio,
  pub align_x: Alignment,
  pub align_y: Alignment,
}

// The direction a line segment can be turned to.
#[uniffi::remote(Enum)]
pub enum Orientation {
  Horizontal,
  Vertical,
  Nearest,
  Angle(f64),
}

// A segment in a path.
#[uniffi::remote(Enum)]
pub enum Segment {
  Line { to: PointF },
  Quadratic { ctrl: PointF, to: PointF },
  Cubic { ctrl1: PointF, ctrl2: PointF, to: PointF },
}

// How the ends of a stroked line are drawn.
#[uniffi::remote(Enum)]
pub enum LineCap {
  Butt,
  Round,
  Square,
}

// How the corners of a stroke are drawn.
#[uniffi::remote(Enum)]
pub enum LineJoin {
  Miter,
  Round,
  Bevel,
}

/// A predefined shape, drawn inside a 100x100 box at the origin. Use `Area::fit` to size it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum Shape {
  /// A heart made of cubic Bezier curves. It is 120 tall, so it overflows the box at the bottom.
  Heart,
  /// A five-pointed star.
  Star,
  /// A regular polygon with the given number of sides, with a corner at the top. Fewer than three sides produces an empty
  /// area.
  Polygon(u32),
}

impl From<Shape> for abra_core::Shape {
  fn from(shape: Shape) -> Self {
    match shape {
      Shape::Heart => abra_core::Shape::Heart,
      Shape::Star => abra_core::Shape::Star,
      Shape::Polygon(sides) => abra_core::Shape::Polygon(sides as usize),
    }
  }
}

/// An array of two numbers as the pair the library takes.
fn pair<T: Copy>(values: &[T], what: &str) -> (T, T) {
  match values {
    [first, second] => (*first, *second),
    _ => panic!("{what} takes 2 values"),
  }
}

// #region: Points, sizes and rectangles

/// A point.
#[uniffi::export]
pub fn point_f_new(x: f32, y: f32) -> PointF {
  PointF::new(x, y)
}

/// The point at the origin.
#[uniffi::export]
pub fn point_f_zero() -> PointF {
  PointF::zero()
}

/// The distance from the origin to the point.
#[uniffi::export]
pub fn point_f_length(point: PointF) -> f32 {
  point.length()
}

/// The square of the length, which is cheaper to work out.
#[uniffi::export]
pub fn point_f_length_squared(point: PointF) -> f32 {
  point.length_squared()
}

/// The point scaled to a length of 1.
#[uniffi::export]
pub fn point_f_normalize(point: PointF) -> PointF {
  point.normalize()
}

/// The dot product of two points taken as vectors.
#[uniffi::export]
pub fn point_f_dot(point: PointF, other: PointF) -> f32 {
  point.dot(other)
}

/// The cross product of two points taken as vectors.
#[uniffi::export]
pub fn point_f_cross(point: PointF, other: PointF) -> f32 {
  point.cross(other)
}

/// The distance between two points.
#[uniffi::export]
pub fn point_f_distance_to(point: PointF, other: PointF) -> f32 {
  point.distance_to(other)
}

/// The point turned a quarter turn, as a vector.
#[uniffi::export]
pub fn point_f_perpendicular(point: PointF) -> PointF {
  point.perpendicular()
}

/// The point `t` of the way to `other`, from 0 to 1.
#[uniffi::export]
pub fn point_f_lerp(point: PointF, other: PointF, t: f32) -> PointF {
  point.lerp(other, t)
}

/// The angle between two points taken as vectors.
#[uniffi::export]
pub fn point_f_angle(point: PointF, other: PointF) -> f32 {
  point.angle(other)
}

/// The sum of two points.
#[uniffi::export]
pub fn point_f_add(point: PointF, other: PointF) -> PointF {
  point + other
}

/// The difference of two points.
#[uniffi::export]
pub fn point_f_sub(point: PointF, other: PointF) -> PointF {
  point - other
}

/// The point with both values multiplied.
#[uniffi::export]
pub fn point_f_mul(point: PointF, factor: f32) -> PointF {
  point * factor
}

/// The point with both values divided.
#[uniffi::export]
pub fn point_f_div(point: PointF, divisor: f32) -> PointF {
  point / divisor
}

/// A size.
#[uniffi::export]
pub fn size_new(width: f32, height: f32) -> Size {
  Size::new(width, height)
}

/// The size as `[width, height]`.
#[uniffi::export]
pub fn size_to_tuple(size: Size) -> Vec<f32> {
  let (width, height) = size.to_tuple::<f32>();
  vec![width, height]
}

/// The largest upright rectangle that fits inside a rectangle of this size after it is rotated by `degrees` around its
/// center, with the width-to-height ratio `aspect` (or whatever shape has the largest area, when there is none).
#[uniffi::export]
pub fn size_inscribed_after_rotation(size: Size, degrees: f64, aspect: Option<f32>) -> Size {
  size.inscribed_after_rotation(degrees, aspect)
}

/// The smallest upright rectangle that holds a rectangle of this size rotated by `degrees`.
#[uniffi::export]
pub fn size_rotated_bounds(size: Size, degrees: f64) -> Size {
  size.rotated_bounds(degrees)
}

/// The width divided by the height, or nothing when the height is zero.
#[uniffi::export]
pub fn size_aspect_ratio(size: Size) -> Option<f32> {
  size.aspect_ratio()
}

/// The sum of two sizes.
#[uniffi::export]
pub fn size_add(size: Size, other: Size) -> Size {
  size + other
}

/// The difference of two sizes.
#[uniffi::export]
pub fn size_sub(size: Size, other: Size) -> Size {
  size - other
}

/// The size with a number added to both sides.
#[uniffi::export]
pub fn size_add_scalar(size: Size, value: f64) -> Size {
  size + value
}

/// The size with a number taken from both sides.
#[uniffi::export]
pub fn size_sub_scalar(size: Size, value: f64) -> Size {
  size - value
}

/// The size with both sides multiplied.
#[uniffi::export]
pub fn size_mul(size: Size, factor: f64) -> Size {
  size * factor
}

/// The size with both sides divided.
#[uniffi::export]
pub fn size_div(size: Size, divisor: f64) -> Size {
  size / divisor
}

/// A rectangle with its top left corner at `origin`.
#[uniffi::export]
pub fn rect_new(origin: PointF, size: Size) -> Rect {
  abra_core::Rect::new(origin, size)
}

/// A rectangle from its four edges. Edges given in the wrong order produce an empty rectangle.
#[uniffi::export]
pub fn rect_from_edges(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
  abra_core::Rect::from_edges(left, top, right, bottom)
}

/// The smallest rectangle containing every point, or an empty rectangle at the origin when there are none.
#[uniffi::export]
pub fn rect_from_points(points: Vec<PointF>) -> Rect {
  abra_core::Rect::from_points(points)
}

/// A rectangle of the given size at the origin.
#[uniffi::export]
pub fn rect_from_size(size: Size) -> Rect {
  size.into()
}

/// The left edge.
#[uniffi::export]
pub fn rect_left(rect: Rect) -> f32 {
  rect.left()
}

/// The top edge.
#[uniffi::export]
pub fn rect_top(rect: Rect) -> f32 {
  rect.top()
}

/// The right edge.
#[uniffi::export]
pub fn rect_right(rect: Rect) -> f32 {
  rect.right()
}

/// The bottom edge.
#[uniffi::export]
pub fn rect_bottom(rect: Rect) -> f32 {
  rect.bottom()
}

/// The top left corner.
#[uniffi::export]
pub fn rect_origin(rect: Rect) -> PointF {
  rect.origin()
}

/// The width and height.
#[uniffi::export]
pub fn rect_size(rect: Rect) -> Size {
  rect.size()
}

/// The center.
#[uniffi::export]
pub fn rect_center(rect: Rect) -> PointF {
  rect.center()
}

/// The edges as `[left, top, right, bottom]`.
#[uniffi::export]
pub fn rect_edges(rect: Rect) -> Vec<f32> {
  let (left, top, right, bottom) = rect.edges::<f32>();
  vec![left, top, right, bottom]
}

/// The corners in the order top left, top right, bottom right, bottom left.
#[uniffi::export]
pub fn rect_corners(rect: Rect) -> Vec<PointF> {
  rect.corners().to_vec()
}

/// Whether the rectangle has no area.
#[uniffi::export]
pub fn rect_is_empty(rect: Rect) -> bool {
  rect.is_empty()
}

/// Whether the point is inside the rectangle. The left and top edges are inside, the right and bottom are not.
#[uniffi::export]
pub fn rect_contains(rect: Rect, point: PointF) -> bool {
  rect.contains(point)
}

/// The overlap of the two rectangles, or an empty rectangle when they do not overlap.
#[uniffi::export]
pub fn rect_intersect(rect: Rect, other: Rect) -> Rect {
  rect.intersect(other)
}

/// The rectangle made `x` times as wide and `y` times as tall, about its center.
#[uniffi::export]
pub fn rect_scaled(rect: Rect, x: f32, y: f32) -> Rect {
  rect.scaled(x, y)
}

/// The rectangle moved by `x` and `y`, with the same size.
#[uniffi::export]
pub fn rect_translated(rect: Rect, x: f32, y: f32) -> Rect {
  rect.translated(x, y)
}

/// The smallest square that holds the rectangle, with the same center.
#[uniffi::export]
pub fn rect_squared(rect: Rect) -> Rect {
  rect.squared()
}

/// The smallest rectangle on whole numbers that holds the rectangle: left and top round down, right and bottom round up.
#[uniffi::export]
pub fn rect_rounded_out(rect: Rect) -> Rect {
  rect.rounded_out()
}

/// Maps a point from the rectangle's coordinate system into a viewport of `viewport` size, the way an SVG `viewBox` maps
/// into its element.
#[uniffi::export]
pub fn rect_map_point(rect: Rect, point: PointF, viewport: Size, aspect_ratio: AspectRatio) -> PointF {
  rect.map_point(point, viewport, aspect_ratio)
}

/// An aspect ratio specification.
#[uniffi::export]
pub fn aspect_ratio_new(mode: PreserveAspectRatio, align_x: Alignment, align_y: Alignment) -> AspectRatio {
  abra_core::AspectRatio::new(mode, align_x, align_y)
}

/// No aspect ratio preservation: stretch to fill.
#[uniffi::export]
pub fn aspect_ratio_none() -> AspectRatio {
  abra_core::AspectRatio::none()
}

/// Preserve the aspect ratio and fit within the viewport, centered.
#[uniffi::export]
pub fn aspect_ratio_meet() -> AspectRatio {
  abra_core::AspectRatio::meet()
}

/// Preserve the aspect ratio and cover the viewport, centered.
#[uniffi::export]
pub fn aspect_ratio_slice() -> AspectRatio {
  abra_core::AspectRatio::slice()
}

/// A quad from its corners: top left, top right, bottom right, bottom left.
#[uniffi::export]
pub fn quad_new(top_left: PointF, top_right: PointF, bottom_right: PointF, bottom_left: PointF) -> Quad {
  abra_core::Quad::new(top_left, top_right, bottom_right, bottom_left)
}

/// An upright rectangle with its top left corner at `origin`.
#[uniffi::export]
pub fn quad_rect(origin: PointF, size: Size) -> Quad {
  abra_core::Quad::rect(origin, size)
}

/// The corners of a rectangle as a quad.
#[uniffi::export]
pub fn quad_from_rect(rect: Rect) -> Quad {
  rect.into()
}

/// The corners in the order top left, top right, bottom right, bottom left.
#[uniffi::export]
pub fn quad_corners(quad: Quad) -> Vec<PointF> {
  quad.corners().to_vec()
}

/// The smallest upright rectangle that holds the quad.
#[uniffi::export]
pub fn quad_bounds(quad: Quad) -> Rect {
  quad.bounds()
}

/// The size of the upright rectangle the quad becomes when its perspective is corrected.
#[uniffi::export]
pub fn quad_flattened_size(quad: Quad) -> Size {
  quad.flattened_size()
}

/// The width-to-height ratio of the quad with its perspective corrected, or nothing when it has no height.
#[uniffi::export]
pub fn quad_flattened_aspect(quad: Quad) -> Option<f64> {
  quad.flattened_aspect()
}

/// The end point of a path segment.
#[uniffi::export]
pub fn segment_end(segment: Segment) -> PointF {
  segment.end()
}

/// The pixels on the straight line between two points, as `[x, y]`, using Bresenham's line algorithm. Both ends are included.
#[uniffi::export]
pub fn bresenham(from: PointF, to: PointF) -> Vec<Vec<i32>> {
  abra_core::bresenham(from, to).into_iter().map(|(x, y)| vec![x, y]).collect()
}

/// Whether the point is inside the closed polygon through `points`, using the even-odd ray-casting rule.
#[uniffi::export]
pub fn polygon_contains(points: Vec<PointF>, point: PointF) -> bool {
  abra_core::polygon_contains(&points, point)
}

/// The union of two shapes, each given as a list of rings. Holes are rings inside an outer ring.
#[uniffi::export]
pub fn polygon_union(a: Vec<Vec<PointF>>, b: Vec<Vec<PointF>>) -> Vec<Vec<PointF>> {
  abra_core::polygon_union(&a, &b)
}

/// The shape `a` with the shape `b` removed. Both are lists of rings, as in `polygon_union`.
#[uniffi::export]
pub fn polygon_difference(a: Vec<Vec<PointF>>, b: Vec<Vec<PointF>>) -> Vec<Vec<PointF>> {
  abra_core::polygon_difference(&a, &b)
}

// #endregion

// #region: Line segments and perspective

/// A straight line segment from one point to another. Angles are measured from the positive x-axis; with y pointing down,
/// positive values are clockwise.
#[derive(Clone, Copy, Debug, PartialEq, uniffi::Object)]
pub struct LineSegment(pub abra_core::LineSegment);

#[uniffi::export]
impl LineSegment {
  /// A line segment from `start` to `end`.
  #[uniffi::constructor]
  pub fn new(start: PointF, end: PointF) -> Self {
    Self(abra_core::LineSegment::new(start, end))
  }

  /// The start point.
  pub fn start(&self) -> PointF {
    self.0.start()
  }

  /// The end point.
  pub fn end(&self) -> PointF {
    self.0.end()
  }

  /// The segment run the other way.
  pub fn reversed(&self) -> Arc<LineSegment> {
    Arc::new(Self(self.0.reversed()))
  }

  /// The end point taken from the start point.
  pub fn delta(&self) -> PointF {
    self.0.delta()
  }

  /// The angle in degrees.
  pub fn degrees(&self) -> f64 {
    self.0.degrees()
  }

  /// The angle in radians.
  pub fn radians(&self) -> f64 {
    self.0.radians()
  }

  /// The angle in degrees, from 0 up to 360.
  pub fn degrees_positive(&self) -> f64 {
    self.0.degrees_positive()
  }

  /// The turn in degrees that brings the segment to an orientation.
  pub fn correction_to(&self, orientation: Orientation) -> f64 {
    self.0.correction_to(orientation)
  }

  /// Whichever of horizontal or vertical the segment is closer to.
  pub fn nearest_orientation(&self) -> Orientation {
    self.0.nearest_orientation()
  }

  /// The length.
  pub fn distance(&self) -> f64 {
    self.0.distance()
  }

  /// The square of the length, which is cheaper to work out.
  pub fn distance_squared(&self) -> f64 {
    self.0.distance_squared()
  }

  /// The square of the distance from a point to the segment.
  pub fn distance_squared_to(&self, point: PointF) -> f32 {
    self.0.distance_squared_to(point)
  }

  /// Whether the start and end are the same point, so the segment has no direction.
  pub fn is_zero_length(&self) -> bool {
    self.0.is_zero_length()
  }

  /// The slope, or nothing for a vertical segment.
  pub fn slope(&self) -> Option<f64> {
    self.0.slope()
  }

  /// The point halfway along.
  pub fn midpoint(&self) -> PointF {
    self.0.midpoint()
  }

  /// The point `t` of the way along, from 0 to 1.
  pub fn point_at(&self, t: f32) -> PointF {
    self.0.point_at(t)
  }

  /// The segment as a path.
  pub fn to_path(&self) -> Arc<Path> {
    Arc::new(Path::from_core(self.0.into()))
  }
}

/// A perspective transform: it maps any four points onto any other four, carrying straight lines to straight lines.
#[derive(Clone, Copy, Debug, PartialEq, uniffi::Object)]
pub struct Homography(pub abra_core::Homography);

#[uniffi::export]
impl Homography {
  /// The eight numbers `a` to `h` of `x' = (a*x + b*y + c) / (g*x + h*y + 1)` and `y' = (d*x + e*y + f) / (g*x + h*y + 1)`.
  pub fn coefficients(&self) -> Vec<f64> {
    self.0.coefficients().to_vec()
  }

  /// Where the transform takes a position, as `[x, y]`.
  pub fn map(&self, x: f64, y: f64) -> Vec<f64> {
    let (x, y) = self.0.map(x, y);
    vec![x, y]
  }

  /// Where the transform takes a position as `[x, y]`, or nothing for one past the horizon of the plane.
  pub fn map_finite(&self, x: f64, y: f64) -> Option<Vec<f64>> {
    self.0.map_finite(x, y).map(|(x, y)| vec![x, y])
  }

  /// The biggest window of `size` `[width, height]` inside `bounds` `[left, top, right, bottom]` that the source
  /// `[width, height]` still covers, as `[left, top, scale]`.
  pub fn covered_window(&self, size: Vec<f64>, source: Vec<f64>, bounds: Vec<f64>) -> Vec<f64> {
    let bounds: [f64; 4] = bounds.try_into().expect("bounds take 4 values");
    let (left, top, scale) = self.0.covered_window(
      pair(&size, "a size"),
      pair(&source, "a source size"),
      (bounds[0], bounds[1], bounds[2], bounds[3]),
    );
    vec![left, top, scale]
  }
}

/// The transform that takes each corner of `from` onto the matching corner of `to`, or nothing if either quad is degenerate.
/// (A uniffi constructor cannot return nothing, so this is a function and not `Homography::from_quads`.)
#[uniffi::export]
pub fn homography_from_quads(from: Quad, to: Quad) -> Option<Arc<Homography>> {
  abra_core::Homography::from_quads(&from, &to).map(|homography| Arc::new(Homography(homography)))
}

/// The transform that takes each of the four `from` points, `[x, y]`, onto the matching `to` point, or nothing if the points
/// are degenerate. (A function and not `Homography::from_points`, for the reason given at `homography_from_quads`.)
#[uniffi::export]
pub fn homography_from_points(from: Vec<Vec<f64>>, to: Vec<Vec<f64>>) -> Option<Arc<Homography>> {
  let corners = |points: Vec<Vec<f64>>| -> [(f64, f64); 4] {
    let corners: Vec<(f64, f64)> = points.iter().map(|point| pair(point, "a point")).collect();
    corners.try_into().expect("a transform takes 4 points")
  };
  abra_core::Homography::from_points(&corners(from), &corners(to)).map(|homography| Arc::new(Homography(homography)))
}

// #endregion

// #region: Paths and areas

/// Lines and curves from a start point. A path is not closed: use an `Area` for a closed shape.
#[derive(Debug, uniffi::Object)]
pub struct Path(RwLock<abra_core::Path>);

/// A closed shape made of lines and curves, used for drawing, filling and effects. It has every function of a path.
#[derive(Debug, uniffi::Object)]
pub struct Area(RwLock<abra_core::Area>);

impl Path {
  /// A handle for a path of the library.
  pub fn from_core(path: abra_core::Path) -> Self {
    Self(RwLock::new(path))
  }

  /// A copy of the path of the library that this holds.
  pub fn core(&self) -> abra_core::Path {
    self.0.read().unwrap().clone()
  }

  /// Changes the path in place.
  pub fn edit<R>(&self, change: impl FnOnce(&mut abra_core::Path) -> R) -> R {
    change(&mut self.0.write().unwrap())
  }
}

impl Area {
  /// A handle for an area of the library.
  pub fn from_core(area: abra_core::Area) -> Self {
    Self(RwLock::new(area))
  }

  /// A copy of the area of the library that this holds.
  pub fn core(&self) -> abra_core::Area {
    self.0.read().unwrap().clone()
  }

  /// Changes the area in place.
  pub fn edit<R>(&self, change: impl FnOnce(&mut abra_core::Area) -> R) -> R {
    change(&mut self.0.write().unwrap())
  }
}

/// Writes the functions that a `Path` and an `Area` share: the library's `Area` derefs to its `Path`, so both have them. The
/// handle is a tuple struct holding the library's value behind a lock, with `edit`.
macro_rules! path_methods {
  ($handle:ident) => {
    #[uniffi::export]
    impl $handle {
      /// Sets the starting point.
      pub fn move_to(&self, start: PointF) {
        self.edit(|path| {
          path.move_to(start);
        });
      }

      /// Adds a straight line to a point.
      pub fn line_to(&self, to: PointF) {
        self.edit(|path| {
          path.line_to(to);
        });
      }

      /// Adds a quadratic Bezier curve with one control point.
      pub fn quad_to(&self, ctrl: PointF, to: PointF) {
        self.edit(|path| {
          path.quad_to(ctrl, to);
        });
      }

      /// Adds a cubic Bezier curve with two control points.
      pub fn cubic_to(&self, ctrl1: PointF, ctrl2: PointF, to: PointF) {
        self.edit(|path| {
          path.cubic_to(ctrl1, ctrl2, to);
        });
      }

      /// The starting point.
      pub fn start(&self) -> PointF {
        self.0.read().unwrap().start()
      }

      /// The ending point.
      pub fn end(&self) -> PointF {
        self.0.read().unwrap().end()
      }

      /// The segments.
      pub fn segments(&self) -> Vec<Segment> {
        self.0.read().unwrap().segments().to_vec()
      }

      /// The direction angle in degrees, if this is a path made by `from_degrees`.
      pub fn angle_degrees(&self) -> Option<f32> {
        self.0.read().unwrap().angle_degrees()
      }

      /// The start point and the end point of every segment.
      pub fn points(&self) -> Vec<PointF> {
        self.0.read().unwrap().points()
      }

      /// The point at `t`, from 0 to 1, along the whole path, spread evenly over the segments (not by length).
      pub fn point_at(&self, t: f32) -> PointF {
        self.0.read().unwrap().point_at(t)
      }

      /// The point at `t`, from 0 to 1, along one segment.
      pub fn point_at_segment(&self, segment: u32, t: f32) -> PointF {
        self.0.read().unwrap().point_at_segment(segment as usize, t)
      }

      /// The path as a polyline; the tolerance is how closely it follows the curves.
      pub fn flatten(&self, tolerance: f32) -> Vec<PointF> {
        self.0.read().unwrap().flatten(tolerance)
      }

      /// The approximate length.
      pub fn length(&self) -> f32 {
        self.0.read().unwrap().length()
      }

      /// The bounding box.
      pub fn bounds(&self) -> Rect {
        self.0.read().unwrap().bounds()
      }

      /// The position, from 0 to 1, of the closest point of the path to these coordinates.
      pub fn closest_time(&self, x: f32, y: f32) -> f32 {
        self.0.read().unwrap().closest_time(x, y)
      }

      /// The closest point of the path to these coordinates.
      pub fn closest_point(&self, x: f32, y: f32) -> PointF {
        self.0.read().unwrap().closest_point(x, y)
      }

      /// The path mapped from the `source` rectangle into a viewport of `viewport` size, the way an SVG `viewBox` maps into
      /// its element.
      pub fn transform_to_viewport(&self, source: Rect, viewport: Size, aspect_ratio: AspectRatio) -> Arc<Path> {
        let path = self.0.read().unwrap().transform_to_viewport(&source, viewport, aspect_ratio);
        Arc::new(Path::from_core(path))
      }

      /// Points evenly spaced along the path; the spacing is a fraction of its length, from 0 to 1.
      pub fn sample_points(&self, spacing_ratio: f32) -> Vec<PointF> {
        self.0.read().unwrap().sample_points(spacing_ratio)
      }
    }
  };
}

path_methods!(Path);
path_methods!(Area);

#[uniffi::export]
impl Path {
  /// An empty path.
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self::from_core(abra_core::Path::new())
  }

  /// A straight line from one point to another.
  #[uniffi::constructor]
  pub fn line(from: PointF, to: PointF) -> Self {
    Self::from_core(abra_core::Path::line(from, to))
  }

  /// A unit direction line from an angle in degrees, which gradients read. Zero points right, and positive angles turn
  /// clockwise.
  #[uniffi::constructor]
  pub fn from_degrees(degrees: f32) -> Self {
    Self::from_core(abra_core::Path::from(degrees))
  }

  /// The path scaled from its own bounds into a box of `size`.
  pub fn fit(&self, size: Size, aspect_ratio: AspectRatio) -> Arc<Path> {
    Arc::new(Self::from_core(self.0.read().unwrap().fit(size, aspect_ratio)))
  }

  /// Starts a stroke of this path as an open line, which has a join, a cap and a miter limit to set.
  pub fn stroke(&self, width: f32) -> Arc<PathStroke> {
    Arc::new(PathStroke {
      path: self.core(),
      width,
      join: None,
      cap: None,
      miter_limit: None,
    })
  }
}

#[uniffi::export]
impl Area {
  /// An empty area.
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self::from_core(abra_core::Area::new())
  }

  /// A rectangular area the size of an image.
  #[uniffi::constructor]
  pub fn new_from_image(image: Arc<Image>) -> Self {
    Self::from_core(abra_core::Area::new_from_image(&image.core()))
  }

  /// A rectangular area.
  #[uniffi::constructor]
  pub fn rect(origin: PointF, size: Size) -> Self {
    Self::from_core(abra_core::Area::rect(origin, size))
  }

  /// A circular area.
  #[uniffi::constructor]
  pub fn circle(center: PointF, radius: f64) -> Self {
    Self::from_core(abra_core::Area::circle(center, radius))
  }

  /// A circular area around a center, as large as the points around it: its radius is the average distance from the
  /// center to the points.
  #[uniffi::constructor]
  pub fn circle_around(center: PointF, points: Vec<PointF>) -> Self {
    Self::from_core(abra_core::Area::circle_around(center, points))
  }

  /// An elliptical area.
  #[uniffi::constructor]
  pub fn ellipse(center: PointF, size: Size) -> Self {
    Self::from_core(abra_core::Area::ellipse(center, size))
  }

  /// A predefined shape in a 100x100 box at the origin. Use `fit` to size it.
  #[uniffi::constructor]
  pub fn shape(shape: Shape) -> Self {
    Self::from_core(abra_core::Area::shape(shape.into()))
  }

  /// An area through these points, each `[x, y]`.
  #[uniffi::constructor]
  pub fn from_points(points: Vec<Vec<f32>>) -> Self {
    let points: Vec<[f32; 2]> =
      points.into_iter().map(|point| point.try_into().expect("a point takes 2 values")).collect();
    Self::from_core(abra_core::Area::from_points(&points))
  }

  /// A rectangular area of this size, at the origin.
  #[uniffi::constructor]
  pub fn from_size(size: Size) -> Self {
    Self::from_core(abra_core::Area::from_size(size))
  }

  /// A copy of the area with this feather radius, in pixels, on its edges.
  pub fn with_feather(&self, feather: u32) -> Arc<Area> {
    Arc::new(Self::from_core(self.core().with_feather(feather)))
  }

  /// The feather radius of the edges.
  pub fn feather(&self) -> u32 {
    self.0.read().unwrap().feather()
  }

  /// Whether a point is inside the area, using the ray-casting algorithm.
  pub fn contains(&self, point: PointF) -> bool {
    self.0.read().unwrap().contains(point)
  }

  /// The area scaled from its own bounds into a box of `size`.
  pub fn fit(&self, size: Size, aspect_ratio: AspectRatio) -> Arc<Area> {
    Arc::new(Self::from_core(self.0.read().unwrap().fit(size, aspect_ratio)))
  }

  /// The area as a quad, when it is a convex polygon with exactly four corners; nothing for anything else.
  pub fn to_quad(&self) -> Option<Quad> {
    self.0.read().unwrap().to_quad()
  }

  /// Starts a stroke along the boundary of this area, which has a join and a miter limit to set.
  pub fn stroke(&self, width: f32) -> Arc<AreaStroke> {
    Arc::new(AreaStroke {
      area: self.core(),
      width,
      join: None,
      miter_limit: None,
    })
  }
}

/// A stroke of an open path that is described but not yet built. Each `with_` function gives a changed copy.
#[derive(Debug, uniffi::Object)]
pub struct PathStroke {
  path: abra_core::Path,
  width: f32,
  join: Option<LineJoin>,
  cap: Option<LineCap>,
  miter_limit: Option<f32>,
}

#[uniffi::export]
impl PathStroke {
  /// A copy that draws corners this way. The library's default is a miter.
  pub fn with_join(&self, join: LineJoin) -> Arc<PathStroke> {
    Arc::new(PathStroke {
      path: self.path.clone(),
      width: self.width,
      join: Some(join),
      cap: self.cap,
      miter_limit: self.miter_limit,
    })
  }

  /// A copy that draws the two ends this way. The library's default is a butt.
  pub fn with_cap(&self, cap: LineCap) -> Arc<PathStroke> {
    Arc::new(PathStroke {
      path: self.path.clone(),
      width: self.width,
      join: self.join,
      cap: Some(cap),
      miter_limit: self.miter_limit,
    })
  }

  /// A copy with how far a miter may reach, as a multiple of half the width, before the corner is beveled.
  pub fn with_miter_limit(&self, limit: f32) -> Arc<PathStroke> {
    Arc::new(PathStroke {
      path: self.path.clone(),
      width: self.width,
      join: self.join,
      cap: self.cap,
      miter_limit: Some(limit),
    })
  }

  /// The outline of the stroke as a closed path.
  pub fn to_path(&self) -> Arc<Path> {
    let mut stroke = self.path.stroke(self.width);
    if let Some(join) = self.join {
      stroke = stroke.with_join(join);
    }
    if let Some(cap) = self.cap {
      stroke = stroke.with_cap(cap);
    }
    if let Some(limit) = self.miter_limit {
      stroke = stroke.with_miter_limit(limit);
    }
    Arc::new(Path::from_core(stroke.to_path()))
  }
}

/// A stroke along the boundary of an area that is described but not yet built. Each `with_` function gives a changed copy.
#[derive(Debug, uniffi::Object)]
pub struct AreaStroke {
  area: abra_core::Area,
  width: f32,
  join: Option<LineJoin>,
  miter_limit: Option<f32>,
}

#[uniffi::export]
impl AreaStroke {
  /// A copy that draws corners this way. The library's default is a miter.
  pub fn with_join(&self, join: LineJoin) -> Arc<AreaStroke> {
    Arc::new(AreaStroke {
      area: self.area.clone(),
      width: self.width,
      join: Some(join),
      miter_limit: self.miter_limit,
    })
  }

  /// A copy with how far a miter may reach, as a multiple of half the width, before the corner is beveled.
  pub fn with_miter_limit(&self, limit: f32) -> Arc<AreaStroke> {
    Arc::new(AreaStroke {
      area: self.area.clone(),
      width: self.width,
      join: self.join,
      miter_limit: Some(limit),
    })
  }

  /// The stroke as a ring-shaped area centered on the boundary.
  pub fn to_area(&self) -> Arc<Area> {
    let mut stroke = self.area.stroke(self.width);
    if let Some(join) = self.join {
      stroke = stroke.with_join(join);
    }
    if let Some(limit) = self.miter_limit {
      stroke = stroke.with_miter_limit(limit);
    }
    Arc::new(Area::from_core(stroke.to_area()))
  }
}

// #endregion

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn points_sizes_and_rectangles_are_records_with_free_functions() {
    assert_eq!(point_f_length(point_f_new(3.0, 4.0)), 5.0);
    assert_eq!(point_f_add(point_f_new(1.0, 2.0), point_f_new(3.0, 4.0)), point_f_new(4.0, 6.0));
    assert_eq!(size_mul(size_new(2.0, 3.0), 2.0), size_new(4.0, 6.0));
    let rect = rect_new(point_f_new(2.0, 3.0), size_new(10.0, 20.0));
    assert_eq!(rect_edges(rect), vec![2.0, 3.0, 12.0, 23.0]);
    assert!(rect_contains(rect, point_f_new(2.0, 3.0)));
    assert!(!rect_contains(rect, point_f_new(12.0, 3.0)));
    let overlap = rect_intersect(rect, rect_from_edges(0.0, 0.0, 5.0, 5.0));
    assert_eq!(rect_edges(overlap), vec![2.0, 3.0, 5.0, 5.0]);
    assert_eq!(quad_bounds(quad_from_rect(rect)), rect);
  }

  #[test]
  fn an_area_has_the_functions_of_a_path() {
    let area = Area::rect(point_f_new(2.0, 3.0), size_new(10.0, 20.0));
    assert_eq!(area.segments().len(), 3);
    assert_eq!(rect_edges(area.bounds()), vec![2.0, 3.0, 12.0, 23.0]);
    assert!(area.contains(point_f_new(5.0, 5.0)));
    assert!(!area.contains(point_f_new(50.0, 5.0)));
    area.line_to(point_f_new(2.0, 3.0));
    assert_eq!(area.segments().len(), 4);
    let fitted = area.fit(size_new(5.0, 5.0), aspect_ratio_meet());
    assert_eq!(size_to_tuple(rect_size(fitted.bounds())), vec![2.5, 5.0]);
    assert_eq!(area.with_feather(4).feather(), 4);
    assert_eq!(area.feather(), 0);
  }

  #[test]
  fn a_path_is_built_in_place_and_a_shape_is_an_area() {
    let path = Path::new();
    path.move_to(point_f_new(0.0, 0.0));
    path.line_to(point_f_new(10.0, 0.0));
    assert_eq!(path.length(), 10.0);
    assert_eq!(path.end(), point_f_new(10.0, 0.0));
    assert_eq!(path.point_at(0.5), point_f_new(5.0, 0.0));
    assert_eq!(Area::shape(Shape::Polygon(6)).segments().len(), 6);
    assert!(Area::shape(Shape::Polygon(2)).segments().is_empty());
    assert_eq!(Path::from_degrees(45.0).angle_degrees(), Some(45.0));
    let square = Area::from_points(vec![vec![0.0, 0.0], vec![4.0, 0.0], vec![4.0, 4.0], vec![0.0, 4.0]]);
    assert!(square.to_quad().is_some());
    assert!(Area::circle(point_f_new(0.0, 0.0), 5.0).to_quad().is_none());
  }

  #[test]
  fn strokes_keep_their_settings_and_leave_the_original_alone() {
    let line = Path::line(point_f_new(0.0, 0.0), point_f_new(10.0, 0.0));
    let stroke = line.stroke(2.0);
    let round = stroke.with_cap(LineCap::Square);
    assert_eq!(rect_edges(stroke.to_path().bounds()), vec![0.0, -1.0, 10.0, 1.0]);
    assert_eq!(rect_edges(round.to_path().bounds()), vec![-1.0, -1.0, 11.0, 1.0]);
    let ring = Area::rect(point_f_new(0.0, 0.0), size_new(10.0, 10.0)).stroke(2.0).with_join(LineJoin::Bevel).to_area();
    assert!(!ring.contains(point_f_new(5.0, 5.0)));
    assert!(ring.contains(point_f_new(0.0, 5.0)));
  }

  #[test]
  fn a_line_segment_reports_its_angle_and_length() {
    let line = LineSegment::new(point_f_new(0.0, 0.0), point_f_new(3.0, 4.0));
    assert_eq!(line.distance(), 5.0);
    assert_eq!(line.midpoint(), point_f_new(1.5, 2.0));
    assert_eq!(line.reversed().start(), point_f_new(3.0, 4.0));
    assert_eq!(line.to_path().length(), 5.0);
    let flat = LineSegment::new(point_f_new(0.0, 0.0), point_f_new(10.0, 0.0));
    assert_eq!(flat.nearest_orientation(), Orientation::Horizontal);
  }

  #[test]
  fn a_homography_maps_one_quad_onto_another() {
    let from = quad_rect(point_f_new(0.0, 0.0), size_new(10.0, 10.0));
    let to = quad_rect(point_f_new(0.0, 0.0), size_new(20.0, 20.0));
    let homography = homography_from_quads(from, to).expect("two squares make a transform");
    let mapped = homography.map(5.0, 5.0);
    assert!((mapped[0] - 10.0).abs() < 1e-6 && (mapped[1] - 10.0).abs() < 1e-6);
    let line = quad_new(point_f_new(0.0, 0.0), point_f_new(1.0, 1.0), point_f_new(2.0, 2.0), point_f_new(3.0, 3.0));
    assert!(homography_from_quads(line, to).is_none());
    assert_eq!(bresenham(point_f_new(0.0, 0.0), point_f_new(2.0, 0.0)), vec![vec![0, 0], vec![1, 0], vec![2, 0]]);
  }
}
