use abra_core::{IntoNumber, geometry::*, units::Units};
use drawing::{CoverageMask, PolygonCoverage, RectCoverage, StrokeCoverage};
use filters::repair::heal;

use crate::Tool;

/// The form of a [`Remover`]. A shape has a size but no location: where it lands is set on the [`Remover`], so the
/// same shape can be stamped anywhere.
#[derive(Clone, Debug)]
pub enum RemoverShape {
  Circle(f32),
  Rectangle(f32, f32),
  Area(Area),
}

/// Where a [`RemoverShape`] is placed on the image.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapePosition {
  /// A single point. Shapes are centered on it.
  Point(PointF),
  /// From one point to another. A circle is dragged along them like a brush stroke.
  /// Rectangles and areas can't be dragged, so they are centered on the middle of the two points.
  Line(PointF, PointF),
  /// Through a series of points, joined by straight lines. This is healed as one stroke, not one line at a time,
  /// so it is faster and has no seams. Circles follow it, and rectangles and areas can't be dragged, so
  /// they are centered on the middle of its bounding box.
  Path(Vec<PointF>),
}

impl ShapePosition {
  /// A position that runs from `p_start` to `p_end`.
  pub fn line(p_start: impl Into<PointF>, p_end: impl Into<PointF>) -> Self {
    ShapePosition::Line(p_start.into(), p_end.into())
  }
}

impl From<PointF> for ShapePosition {
  fn from(p_point: PointF) -> Self {
    ShapePosition::Point(p_point)
  }
}

/// A plain number that can be a coordinate of a [`ShapePosition`].
///
/// This exists so a position can be built from either `(x, y)` or `((x1, y1), (x2, y2))`. Both are tuples, and
/// Rust can't tell that a tuple is never an [`IntoNumber`] unless the trait bound is one this crate owns.
pub trait Coordinate: IntoNumber {}

macro_rules! impl_coordinate {
  ($($number:ty),+ $(,)?) => {
    $(impl Coordinate for $number {})+
  };
}

impl_coordinate!(f32, f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

/// A point: `(x, y)`.
impl<A: Coordinate, B: Coordinate> From<(A, B)> for ShapePosition {
  fn from(p_point: (A, B)) -> Self {
    ShapePosition::Point(p_point.into())
  }
}

/// A line between two points: `((x1, y1), (x2, y2))`.
impl<A: Coordinate, B: Coordinate, C: Coordinate, D: Coordinate> From<((A, B), (C, D))> for ShapePosition {
  fn from(p_line: ((A, B), (C, D))) -> Self {
    ShapePosition::Line(p_line.0.into(), p_line.1.into())
  }
}

/// A line between two points.
impl From<(PointF, PointF)> for ShapePosition {
  fn from(p_line: (PointF, PointF)) -> Self {
    ShapePosition::Line(p_line.0, p_line.1)
  }
}

/// Something [`Remover::with_path`] can follow: a list of points, or a [`Path`].
///
/// Points can be in an array, a `Vec` or a slice, as `(x, y)` pairs or [`PointF`]s.
pub trait IntoPathPoints {
  /// The points of the path, in order. Curves are flattened into short straight lines.
  fn into_path_points(self) -> Vec<PointF>;
}

impl<P: Into<PointF>, const N: usize> IntoPathPoints for [P; N] {
  fn into_path_points(self) -> Vec<PointF> {
    self.into_iter().map(Into::into).collect()
  }
}

impl<P: Into<PointF>> IntoPathPoints for Vec<P> {
  fn into_path_points(self) -> Vec<PointF> {
    self.into_iter().map(Into::into).collect()
  }
}

impl<P: Clone + Into<PointF>> IntoPathPoints for &[P] {
  fn into_path_points(self) -> Vec<PointF> {
    self.iter().cloned().map(Into::into).collect()
  }
}

impl IntoPathPoints for &Path {
  fn into_path_points(self) -> Vec<PointF> {
    // Curves are flattened to lines that stay within a quarter of a pixel of the curve.
    self.flatten(0.25)
  }
}

impl IntoPathPoints for Path {
  fn into_path_points(self) -> Vec<PointF> {
    (&self).into_path_points()
  }
}

impl RemoverShape {
  /// Creates a new circle shape with the given radius, centered on the position it is placed at.
  /// # Arguments
  /// - `p_radius`: The radius of the circle.
  pub fn circle(p_radius: impl IntoNumber) -> Self {
    RemoverShape::Circle(p_radius.into())
  }
  /// Creates a new rectangle shape with the given width and height, centered on the position it is placed at.
  /// # Arguments
  /// - `p_width`: The width of the rectangle.
  /// - `p_height`: The height of the rectangle.
  pub fn rectangle(p_width: impl IntoNumber, p_height: impl IntoNumber) -> Self {
    RemoverShape::Rectangle(p_width.into(), p_height.into())
  }
  /// Creates a new area shape. The middle of its bounding box is placed on the position it is placed at.
  /// # Arguments
  /// - `p_area`: The area.
  pub fn area(p_area: Area) -> Self {
    RemoverShape::Area(p_area)
  }
}

/// A healing stamp: a shape and how it heals, ready to be placed anywhere on an image.
///
/// Build it once and reuse it. Each edit only needs somewhere to land, so keep the stamp as a template and clone it
/// with the positions for that edit:
/// ```ignore
/// let stamp = remover(RemoverShape::circle(6)).with_distance(Units::Pixels(4));
/// stamp.clone().with_position((120, 80)).with_position((300, 210)).apply(&mut image);
/// ```
#[derive(Clone, Debug)]
pub struct Remover {
  /// The shape of the remover tool.
  shape: RemoverShape,
  /// The hardness of the remover tool. Determines how strongly the tool affects the image near the edges of the shape.
  hardness: f32,
  /// Where the shape is placed on the image when applied. It is healed once at each, in order. Empty by default,
  /// which heals nothing.
  positions: Vec<ShapePosition>,
  /// The distance between the inner and outer shapes.
  /// The outer shape is used to sample the area around the inner shape.
  /// This value determines how much larger the outer shape is compared to the inner shape.
  /// A circle with a radius of 20px and a distance of 10px would have an outer radius of 30px.
  distance: Units,
}

impl Remover {
  /// Sets the hardness of the remover tool.
  pub fn with_hardness(mut self, p_hardness: impl IntoNumber) -> Self {
    self.hardness = p_hardness.into();
    self
  }
  /// Sets the distance between the inner and outer shapes.
  pub fn with_distance(mut self, p_distance: Units) -> Self {
    self.distance = p_distance;
    self
  }
  /// Adds a position the remover tool is placed at when applied. It heals at each position in the order they were
  /// added, so a later one sees the result of the earlier ones.
  pub fn with_position(mut self, p_position: impl Into<ShapePosition>) -> Self {
    self.positions.push(p_position.into());
    self
  }
  /// Adds a position that follows a path. It is healed as one stroke, so it is faster than a line at a time and has
  /// no seams. The path is either a list of points joined by straight lines, such as `[(0, 0), (10, 5), (20, 0)]`
  /// or a `Vec` or slice of any [`PointF`]-like points, or a [`Path`], curves included.
  ///
  /// One point is a single dot, and no points adds nothing.
  /// - `p_path`: The path to follow.
  pub fn with_path(mut self, p_path: impl IntoPathPoints) -> Self {
    let points = p_path.into_path_points();
    if !points.is_empty() {
      self.positions.push(ShapePosition::Path(points));
    }
    self
  }
  /// Heals the image once with the shape placed at `p_position`.
  fn stamp(&self, p_image: &mut abra_core::Image, p_position: &ShapePosition) {
    let Some(shape) = self.shape.coverage(p_position) else {
      return;
    };
    let reference = self.shape.reference_length();
    let distance = self.distance.to_pixels(reference, p_image.resolution()).max(1.0);
    // The soft edge reaches at most half way to the middle of the shape, so the center is always fully healed.
    let feather = (1.0 - self.hardness.clamp(0.0, 1.0)) * reference * 0.5;
    heal(shape.as_ref()).with_distance(distance).with_feather(feather).apply(p_image);
  }
}

impl Tool for Remover {
  fn apply<'a>(&self, p_image: impl Into<abra_core::ImageRef<'a>>) {
    let mut image = p_image.into();
    for position in &self.positions {
      self.stamp(&mut image, position);
    }
  }
}

impl RemoverShape {
  /// The shape placed at `p_position`, or `None` when it covers nothing.
  fn coverage(&self, p_position: &ShapePosition) -> Option<Box<dyn CoverageMask>> {
    let points = match p_position {
      ShapePosition::Point(point) => vec![*point],
      ShapePosition::Line(start, end) => vec![*start, *end],
      ShapePosition::Path(points) => points.clone(),
    };
    if points.is_empty() {
      return None;
    }
    // Where a shape that can't be dragged along the position is centered: the middle of its bounding box.
    let (min, max) = points.iter().fold((points[0], points[0]), |(min, max), point| {
      (PointF::new(min.x.min(point.x), min.y.min(point.y)), PointF::new(max.x.max(point.x), max.y.max(point.y)))
    });
    let center = PointF::new((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
    Some(match self {
      RemoverShape::Circle(radius) => Box::new(StrokeCoverage::new(&points, *radius)),
      RemoverShape::Rectangle(width, height) => Box::new(RectCoverage::centered(center, (*width, *height))),
      RemoverShape::Area(area) => {
        let outline = area.flatten(0.25);
        if outline.len() < 3 {
          return None;
        }
        // The area is placed so the middle of its bounding box lands on the center point.
        let (min_x, min_y, max_x, max_y) = area.bounds().edges::<f32>();
        let (dx, dy) = (center.x - (min_x + max_x) / 2.0, center.y - (min_y + max_y) / 2.0);
        Box::new(PolygonCoverage::new(outline.iter().map(|p| PointF::new(p.x + dx, p.y + dy)).collect()))
      }
    })
  }

  /// A length that describes the size of the shape, used to resolve percentages and the soft edge: the radius of
  /// a circle, or half the shorter side of anything else.
  fn reference_length(&self) -> f32 {
    match self {
      RemoverShape::Circle(radius) => radius.abs(),
      RemoverShape::Rectangle(width, height) => width.abs().min(height.abs()) / 2.0,
      RemoverShape::Area(area) => {
        let (min_x, min_y, max_x, max_y) = area.bounds().edges::<f32>();
        (max_x - min_x).min(max_y - min_y) / 2.0
      }
    }
  }
}

pub fn remover(p_shape: RemoverShape) -> Remover {
  Remover {
    shape: p_shape,
    distance: Units::Pixels(3),
    positions: Vec::new(),
    hardness: 0.8,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Channels, Image};

  /// An opaque image whose color at each pixel is `color(x, y)`.
  fn image_of(p_width: u32, p_height: u32, p_color: impl Fn(u32, u32) -> [u8; 3]) -> Image {
    let mut bytes = Vec::new();
    for y in 0..p_height {
      for x in 0..p_width {
        let [r, g, b] = p_color(x, y);
        bytes.extend_from_slice(&[r, g, b, 255]);
      }
    }
    Image::new_from_pixels(p_width, p_height, &bytes, Channels::RGBA)
  }

  fn rgb(p_image: &Image, p_x: u32, p_y: u32) -> [u8; 3] {
    let (r, g, b, _) = p_image.get_pixel(p_x, p_y).unwrap();
    [r, g, b]
  }

  fn assert_close(p_actual: [u8; 3], p_expected: [u8; 3], p_tolerance: i32) {
    for c in 0..3 {
      assert!(
        (p_actual[c] as i32 - p_expected[c] as i32).abs() <= p_tolerance,
        "{p_actual:?} is not within {p_tolerance} of {p_expected:?}"
      );
    }
  }

  const BLEMISH: [u8; 3] = [200, 30, 30];

  fn with_blemish(p_x: u32, p_y: u32, p_base: impl Fn(u32, u32) -> [u8; 3]) -> Image {
    // A 7x7 blemish, sitting where the shapes below are centered.
    image_of(100, 100, |x, y| if x.abs_diff(p_x) <= 3 && y.abs_diff(p_y) <= 3 { BLEMISH } else { p_base(x, y) })
  }

  #[test]
  fn removes_a_blemish_from_a_flat_background() {
    let mut image = with_blemish(50, 50, |_, _| [120, 140, 160]);
    remover(RemoverShape::circle(6))
      .with_distance(Units::Pixels(8))
      .with_position((50, 50))
      .with_hardness(1.0)
      .apply(&mut image);
    for (x, y) in [(50, 50), (48, 52), (53, 47), (47, 47), (53, 53)] {
      assert_close(rgb(&image, x, y), [120, 140, 160], 2);
    }
  }

  #[test]
  fn a_soft_edge_leaves_some_of_the_original_near_the_edge() {
    let healed_at_edge = |hardness: f32| {
      let mut image = with_blemish(50, 50, |_, _| [120, 140, 160]);
      remover(RemoverShape::circle(6))
        .with_distance(Units::Pixels(8))
        .with_position((50, 50))
        .with_hardness(hardness)
        .apply(&mut image);
      // The corner of the blemish is close to the edge of the shape.
      rgb(&image, 53, 53)
    };
    assert_close(healed_at_edge(1.0), [120, 140, 160], 2);
    assert!(healed_at_edge(0.0)[0] > 125, "a fully soft edge should leave part of the blemish");
    // The middle is always fully healed, however soft the edge.
    let mut image = with_blemish(50, 50, |_, _| [120, 140, 160]);
    remover(RemoverShape::circle(6))
      .with_distance(Units::Pixels(8))
      .with_position((50, 50))
      .with_hardness(0.0)
      .apply(&mut image);
    assert_close(rgb(&image, 50, 50), [120, 140, 160], 2);
  }

  #[test]
  fn heals_lines_and_areas() {
    let base = |_: u32, _: u32| [90, 90, 90];
    let mut image =
      image_of(100, 100, |x, y| if y.abs_diff(50) <= 1 && (20..=80).contains(&x) { BLEMISH } else { base(x, y) });
    remover(RemoverShape::circle(2))
      .with_distance(Units::Pixels(6))
      .with_position(ShapePosition::line((20, 50), (80, 50)))
      .apply(&mut image);
    assert_close(rgb(&image, 50, 50), [90, 90, 90], 2);

    let mut image = with_blemish(50, 50, base);
    let area = Area::rect((0, 0), (10, 10));
    remover(RemoverShape::area(area)).with_distance(Units::Pixels(6)).with_position((50, 50)).apply(&mut image);
    assert_close(rgb(&image, 50, 50), [90, 90, 90], 2);
  }

  /// Two blemishes on a flat background, at (30, 30) and (70, 60).
  fn two_blemishes() -> Image {
    image_of(100, 100, |x, y| {
      let near = |cx: u32, cy: u32| x.abs_diff(cx) <= 3 && y.abs_diff(cy) <= 3;
      if near(30, 30) || near(70, 60) { BLEMISH } else { [120, 140, 160] }
    })
  }

  #[test]
  fn one_stamp_can_be_placed_anywhere() {
    let stamp = remover(RemoverShape::circle(6)).with_distance(Units::Pixels(8)).with_hardness(1.0);

    // Each clone is placed on its own, and the original template is unaffected.
    let mut image = two_blemishes();
    stamp.clone().with_position((30, 30)).apply(&mut image);
    assert_close(rgb(&image, 30, 30), [120, 140, 160], 2);
    assert_eq!(rgb(&image, 70, 60), BLEMISH, "only the first blemish should be healed");
    stamp.clone().with_position((70, 60)).apply(&mut image);
    assert_close(rgb(&image, 70, 60), [120, 140, 160], 2);

    // A stamp with no position heals nothing.
    let mut image = two_blemishes();
    let before = image.clone();
    stamp.apply(&mut image);
    assert_eq!(image.rgba(), before.rgba());
  }

  #[test]
  fn several_positions_heal_each_in_turn() {
    let stamp = remover(RemoverShape::circle(6)).with_distance(Units::Pixels(8)).with_hardness(1.0);
    for stamp in [
      stamp.clone().with_position((30, 30)).with_position((70, 60)),
      stamp.clone().with_position((30, 30)).with_position(PointF::new(70, 60)),
    ] {
      let mut image = two_blemishes();
      stamp.apply(&mut image);
      assert_close(rgb(&image, 30, 30), [120, 140, 160], 2);
      assert_close(rgb(&image, 70, 60), [120, 140, 160], 2);
    }
  }

  #[test]
  fn a_line_position_sets_where_a_line_runs() {
    let mut image =
      image_of(100, 100, |x, y| if y.abs_diff(50) <= 1 && (40..=60).contains(&x) { BLEMISH } else { [90; 3] });
    let line = remover(RemoverShape::circle(2)).with_distance(Units::Pixels(6));
    // Running the line somewhere that has no blemish leaves the blemish alone.
    line.clone().with_position(ShapePosition::line((38, 20), (62, 20))).apply(&mut image);
    assert_eq!(rgb(&image, 50, 50), BLEMISH);
    line.with_position(ShapePosition::line((38, 50), (62, 50))).apply(&mut image);
    assert_close(rgb(&image, 50, 50), [90; 3], 2);
  }

  #[test]
  fn a_circle_dragged_along_a_line_heals_a_stroke() {
    let mut image =
      image_of(100, 100, |x, y| if y.abs_diff(50) <= 1 && (30..=70).contains(&x) { BLEMISH } else { [90; 3] });
    remover(RemoverShape::circle(4))
      .with_distance(Units::Pixels(6))
      .with_hardness(1.0)
      .with_position(ShapePosition::line((30, 50), (70, 50)))
      .apply(&mut image);
    for x in [30, 40, 50, 60, 70] {
      assert_close(rgb(&image, x, 50), [90; 3], 2);
    }
  }

  #[test]
  fn points_are_added_as_one_path() {
    let point = |x: f32, y: f32| PointF::new(x, y);
    let stamp = remover(RemoverShape::circle(1)).with_path([(0, 1), (2, 3), (4, 5)]);
    assert_eq!(
      stamp.positions,
      vec![ShapePosition::Path(vec![
        point(0.0, 1.0),
        point(2.0, 3.0),
        point(4.0, 5.0)
      ])]
    );
    // A single point is a dot, no points add nothing, and existing positions are kept.
    assert_eq!(remover(RemoverShape::circle(1)).with_path([(7, 8)]).positions.len(), 1);
    assert!(remover(RemoverShape::circle(1)).with_path(Vec::<PointF>::new()).positions.is_empty());
    assert_eq!(remover(RemoverShape::circle(1)).with_position((9, 9)).with_path([(0, 1), (2, 3)]).positions.len(), 2);
  }

  #[test]
  fn a_path_is_flattened_into_points() {
    let mut path = Path::new();
    path.move_to((10, 10)).line_to((20, 10)).line_to((20, 30));
    let stamp = remover(RemoverShape::circle(1)).with_path(&path);
    assert_eq!(
      stamp.positions,
      vec![ShapePosition::Path(vec![
        PointF::new(10, 10),
        PointF::new(20, 10),
        PointF::new(20, 30)
      ])]
    );
  }

  #[test]
  fn a_path_heals_a_bent_stroke() {
    // An L-shaped blemish: along the top, then down the right side.
    let mut image = image_of(100, 100, |x, y| {
      let along_top = y.abs_diff(30) <= 1 && (30..=70).contains(&x);
      let down_side = x.abs_diff(70) <= 1 && (30..=70).contains(&y);
      if along_top || down_side { BLEMISH } else { [90; 3] }
    });
    remover(RemoverShape::circle(4))
      .with_distance(Units::Pixels(6))
      .with_hardness(1.0)
      .with_path([(30, 30), (70, 30), (70, 70)])
      .apply(&mut image);
    for (x, y) in [(30, 30), (50, 30), (70, 30), (70, 50), (70, 70)] {
      assert_close(rgb(&image, x, y), [90; 3], 2);
    }
  }

  #[test]
  fn positions_can_be_written_as_tuples() {
    let point = |x: f32, y: f32| ShapePosition::Point(PointF::new(x, y));
    let line = |x1: f32, y1: f32, x2: f32, y2: f32| ShapePosition::Line(PointF::new(x1, y1), PointF::new(x2, y2));
    assert_eq!(ShapePosition::from((100, 200)), point(100.0, 200.0));
    assert_eq!(ShapePosition::from((1.5f32, 2u8)), point(1.5, 2.0));
    assert_eq!(ShapePosition::from(((100, 100), (200, 200))), line(100.0, 100.0, 200.0, 200.0));
    assert_eq!(ShapePosition::from((PointF::new(1, 2), PointF::new(3, 4))), line(1.0, 2.0, 3.0, 4.0));
    assert_eq!(ShapePosition::from(PointF::new(5, 6)), point(5.0, 6.0));
  }

  #[test]
  fn percent_distance_scales_with_the_shape() {
    let resolution = abra_core::Resolution::SCREEN;
    assert_eq!(Units::Percent(50).to_pixels(20.0, resolution), 10.0);
    assert_eq!(Units::Pixels(7).to_pixels(20.0, resolution), 7.0);
    assert_eq!(Units::Inches(1).to_pixels(20.0, resolution), 96.0);
  }
}
