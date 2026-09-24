use abra_core::{IntoNumber, geometry::*, units::Units};
use rayon::prelude::*;

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
  fn stamp(&self, p_image: &mut abra_core::ImageRef<'_>, p_position: &ShapePosition) {
    let (width, height): (u32, u32) = p_image.dimensions();
    let reference = self.shape.reference_length();
    let distance = self.distance.to_pixels(reference, p_image.resolution()).max(1.0);
    let hardness = self.hardness.clamp(0.0, 1.0);
    // The soft edge reaches at most half way to the middle of the shape, so the center is always fully healed.
    let feather = (1.0 - hardness) * reference * 0.5;

    let Some(healed) = heal(p_image.rgba(), width, height, &self.shape, p_position, distance, feather) else {
      return;
    };

    let pixels = p_image.colors();
    for (index, color) in healed {
      let offset = index * 4;
      pixels[offset] = color[0];
      pixels[offset + 1] = color[1];
      pixels[offset + 2] = color[2];
      // Alpha is left alone; only color is healed.
    }
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

/// The inner shape as something that can be tested a point at a time.
enum Region {
  /// Every point within `half_width` of a path of straight lines. A path of one point is a circle, and one of two
  /// is a line with round ends.
  Stroke {
    points: Vec<PointF>,
    half_width: f32,
  },
  Rectangle {
    min: PointF,
    max: PointF,
  },
  Polygon {
    points: Vec<PointF>,
  },
}

impl Region {
  fn contains(&self, p_x: f32, p_y: f32) -> bool {
    match self {
      Region::Stroke { points, half_width } => {
        let limit = half_width * half_width;
        if points.len() == 1 {
          return distance_squared_to_segment(p_x, p_y, points[0], points[0]) <= limit;
        }
        points.windows(2).any(|pair| distance_squared_to_segment(p_x, p_y, pair[0], pair[1]) <= limit)
      }
      Region::Rectangle { min, max } => p_x >= min.x && p_x <= max.x && p_y >= min.y && p_y <= max.y,
      Region::Polygon { points } => {
        // Ray casting: count how many edges a ray to the right of the point crosses.
        let mut inside = false;
        let mut previous = points[points.len() - 1];
        for &current in points {
          if (current.y > p_y) != (previous.y > p_y)
            && p_x < (previous.x - current.x) * (p_y - current.y) / (previous.y - current.y) + current.x
          {
            inside = !inside;
          }
          previous = current;
        }
        inside
      }
    }
  }

  /// The bounding box as `(min_x, min_y, max_x, max_y)`.
  fn bounds(&self) -> (f32, f32, f32, f32) {
    match self {
      Region::Stroke { points, half_width } => {
        let (min_x, min_y, max_x, max_y) = bounds_of(points);
        (min_x - half_width, min_y - half_width, max_x + half_width, max_y + half_width)
      }
      Region::Rectangle { min, max } => (min.x, min.y, max.x, max.y),
      Region::Polygon { points } => bounds_of(points),
    }
  }
}

/// The bounding box of some points as `(min_x, min_y, max_x, max_y)`.
fn bounds_of(p_points: &[PointF]) -> (f32, f32, f32, f32) {
  p_points.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |(min_x, min_y, max_x, max_y), p| {
    (min_x.min(p.x), min_y.min(p.y), max_x.max(p.x), max_y.max(p.y))
  })
}

/// The squared distance from a point to the nearest point of the segment from `p_start` to `p_end`.
fn distance_squared_to_segment(p_x: f32, p_y: f32, p_start: PointF, p_end: PointF) -> f32 {
  let (sx, sy) = (p_end.x - p_start.x, p_end.y - p_start.y);
  let length_squared = sx * sx + sy * sy;
  let t = if length_squared > 0.0 {
    (((p_x - p_start.x) * sx + (p_y - p_start.y) * sy) / length_squared).clamp(0.0, 1.0)
  } else {
    0.0
  };
  let (dx, dy) = (p_x - (p_start.x + sx * t), p_y - (p_start.y + sy * t));
  dx * dx + dy * dy
}

impl RemoverShape {
  /// The shape placed at `p_position` as a [`Region`], or `None` when it covers nothing.
  fn region(&self, p_position: &ShapePosition) -> Option<Region> {
    let points = match p_position {
      ShapePosition::Point(point) => vec![*point],
      ShapePosition::Line(start, end) => vec![*start, *end],
      ShapePosition::Path(points) => points.clone(),
    };
    if points.is_empty() {
      return None;
    }
    // Where a shape that can't be dragged along the position is centered: the middle of its bounding box.
    let (min_x, min_y, max_x, max_y) = bounds_of(&points);
    let center = PointF::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
    let region = match self {
      RemoverShape::Circle(radius) => Region::Stroke {
        points,
        half_width: *radius,
      },
      RemoverShape::Rectangle(width, height) => Region::Rectangle {
        min: PointF::new(center.x - width / 2.0, center.y - height / 2.0),
        max: PointF::new(center.x + width / 2.0, center.y + height / 2.0),
      },
      RemoverShape::Area(area) => {
        let points = area.flatten(0.25);
        if points.len() < 3 {
          return None;
        }
        // The area is placed so the middle of its bounding box lands on the center point.
        let (min_x, min_y, max_x, max_y) = area.bounds::<f32>();
        let (dx, dy) = (center.x - (min_x + max_x) / 2.0, center.y - (min_y + max_y) / 2.0);
        Region::Polygon {
          points: points.iter().map(|p| PointF::new(p.x + dx, p.y + dy)).collect(),
        }
      }
    };
    let (min_x, min_y, max_x, max_y) = region.bounds();
    (max_x > min_x && max_y > min_y).then_some(region)
  }

  /// A length that describes the size of the shape, used to resolve percentages and the soft edge: the radius of
  /// a circle, or half the shorter side of anything else.
  fn reference_length(&self) -> f32 {
    match self {
      RemoverShape::Circle(radius) => radius.abs(),
      RemoverShape::Rectangle(width, height) => width.abs().min(height.abs()) / 2.0,
      RemoverShape::Area(area) => {
        let (min_x, min_y, max_x, max_y) = area.bounds::<f32>();
        (max_x - min_x).min(max_y - min_y) / 2.0
      }
    }
  }
}

/// A rectangle of the image, in pixels, that the heal works within.
struct Window {
  x: i32,
  y: i32,
  width: usize,
  height: usize,
}

impl Window {
  fn index(&self, p_x: usize, p_y: usize) -> usize {
    p_y * self.width + p_x
  }
}

/// Heals the shape out of an RGBA buffer.
///
/// The steps are:
/// 1. Rasterize the shape into an anti-aliased coverage mask over the part of the image it touches.
/// 2. Find the ring of pixels within `p_distance` around the shape. That is the context the fill is matched to.
/// 3. Search nearby for the offset whose ring looks most like this one, to borrow texture from. If nothing fits,
///    the hole is filled smoothly from the ring alone.
/// 4. Solve for pixels inside the shape whose detail comes from the borrowed patch, and whose edge matches the
///    surrounding pixels exactly, so the color and lighting come from the destination.
/// 5. Blend the result over the original, softening the inside edge by `p_feather` pixels.
///
/// Returns `(pixel index, healed r/g/b)` for every pixel that changed, or `None` when the shape does not touch
/// the image.
fn heal(
  p_rgba: &[u8], p_width: u32, p_height: u32, p_shape: &RemoverShape, p_position: &ShapePosition, p_distance: f32,
  p_feather: f32,
) -> Option<Vec<(usize, [u8; 3])>> {
  let (image_width, image_height) = (p_width as i32, p_height as i32);
  let region = p_shape.region(p_position)?;
  let (min_x, min_y, max_x, max_y) = region.bounds();

  // The area of the shape plus its ring, before it is clipped to the image. A patch borrowed from elsewhere needs
  // all of this to be in the image, so its size is kept.
  let outer = Window {
    x: (min_x - p_distance).floor() as i32,
    y: (min_y - p_distance).floor() as i32,
    width: ((max_x - min_x) + 2.0 * p_distance).ceil() as usize + 2,
    height: ((max_y - min_y) + 2.0 * p_distance).ceil() as usize + 2,
  };

  // Clip to the image.
  let x0 = outer.x.max(0);
  let y0 = outer.y.max(0);
  let x1 = (outer.x + outer.width as i32).min(image_width);
  let y1 = (outer.y + outer.height as i32).min(image_height);
  if x1 <= x0 || y1 <= y0 {
    return None;
  }
  let window = Window {
    x: x0,
    y: y0,
    width: (x1 - x0) as usize,
    height: (y1 - y0) as usize,
  };

  // 1. Coverage mask.
  let coverage = rasterize(&region, &window);
  let hole: Vec<bool> = coverage.iter().map(|&c| c > 0.0).collect();
  if !hole.iter().any(|&h| h) {
    return None;
  }

  // 2. Distance from the shape (outside) and from the edge (inside).
  let outside = distance_to(&hole, window.width, window.height, true);
  let inside = distance_to(&hole, window.width, window.height, false);
  let ring: Vec<usize> = (0..hole.len()).filter(|&i| !hole[i] && outside[i] <= p_distance).collect();

  // Reads past the edge of the image repeat the edge, so a patch that overhangs it is still safe to read.
  let pixel = |p_x: i32, p_y: i32| -> [f32; 3] {
    let (p_x, p_y) = (p_x.clamp(0, image_width - 1), p_y.clamp(0, image_height - 1));
    let offset = ((p_y * image_width + p_x) as usize) * 4;
    [
      p_rgba[offset] as f32,
      p_rgba[offset + 1] as f32,
      p_rgba[offset + 2] as f32,
    ]
  };

  // 3. Where to borrow texture from.
  let source = find_source(&ring, &hole, &window, &outer, (image_width, image_height), &pixel);

  // 4. Solve.
  let solved = solve(&hole, &window, &pixel, source);

  // 5. Blend.
  let mut pixels = Vec::new();
  for local_y in 0..window.height {
    for local_x in 0..window.width {
      let local = window.index(local_x, local_y);
      let mut alpha = coverage[local];
      if alpha <= 0.0 {
        continue;
      }
      if p_feather > 0.5 {
        // Distance 1 is the pixel on the edge, so measure from the middle of that pixel.
        let t = ((inside[local] - 0.5) / p_feather).clamp(0.0, 1.0);
        alpha *= t * t * (3.0 - 2.0 * t);
      }
      let (x, y) = (window.x + local_x as i32, window.y + local_y as i32);
      let original = pixel(x, y);
      let healed = solved[local];
      let mut color = [0u8; 3];
      for channel in 0..3 {
        let value = original[channel] + (healed[channel] - original[channel]) * alpha;
        color[channel] = value.round().clamp(0.0, 255.0) as u8;
      }
      pixels.push(((y * image_width + x) as usize, color));
    }
  }
  Some(pixels)
}

/// How much of each pixel in the window the shape covers, from 0 to 1, by testing a grid of points in each pixel.
fn rasterize(p_region: &Region, p_window: &Window) -> Vec<f32> {
  const SAMPLES: usize = 4;
  let (min_x, min_y, max_x, max_y) = p_region.bounds();
  let mut coverage = vec![0.0; p_window.width * p_window.height];
  for y in 0..p_window.height {
    for x in 0..p_window.width {
      // Most of a long or thin shape's window is empty, and testing a stroke is not free.
      let (left, top) = (p_window.x as f32 + x as f32, p_window.y as f32 + y as f32);
      if left + 1.0 < min_x || left > max_x || top + 1.0 < min_y || top > max_y {
        continue;
      }
      let mut hits = 0;
      for sy in 0..SAMPLES {
        for sx in 0..SAMPLES {
          // Pixel (x, y) spans x..x+1, and its samples sit in the middle of each cell.
          let point_x = p_window.x as f32 + x as f32 + (sx as f32 + 0.5) / SAMPLES as f32;
          let point_y = p_window.y as f32 + y as f32 + (sy as f32 + 0.5) / SAMPLES as f32;
          if p_region.contains(point_x, point_y) {
            hits += 1;
          }
        }
      }
      coverage[y * p_window.width + x] = hits as f32 / (SAMPLES * SAMPLES) as f32;
    }
  }
  coverage
}

/// For every pixel, the distance to the nearest pixel of a set, approximated with a two-pass sweep.
/// - `p_in_hole`: Marks the hole.
/// - `p_to_hole`: Measure to the hole's pixels when `true`, or to the pixels outside it when `false`.
fn distance_to(p_in_hole: &[bool], p_width: usize, p_height: usize, p_to_hole: bool) -> Vec<f32> {
  const DIAGONAL: f32 = std::f32::consts::SQRT_2;
  let mut distance: Vec<f32> = p_in_hole.iter().map(|&h| if h == p_to_hole { 0.0 } else { f32::MAX / 4.0 }).collect();

  let mut relax = |x: usize, y: usize, dx: i32, dy: i32, cost: f32| {
    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
    if nx >= 0 && ny >= 0 && (nx as usize) < p_width && (ny as usize) < p_height {
      let candidate = distance[ny as usize * p_width + nx as usize] + cost;
      let slot = &mut distance[y * p_width + x];
      if candidate < *slot {
        *slot = candidate;
      }
    }
  };

  for y in 0..p_height {
    for x in 0..p_width {
      relax(x, y, -1, 0, 1.0);
      relax(x, y, -1, -1, DIAGONAL);
      relax(x, y, 0, -1, 1.0);
      relax(x, y, 1, -1, DIAGONAL);
    }
  }
  for y in (0..p_height).rev() {
    for x in (0..p_width).rev() {
      relax(x, y, 1, 0, 1.0);
      relax(x, y, 1, 1, DIAGONAL);
      relax(x, y, 0, 1, 1.0);
      relax(x, y, -1, 1, DIAGONAL);
    }
  }
  distance
}

/// Finds the offset `(dx, dy)` such that the pixels at `ring + offset` look most like the ring, or `None` when
/// there is no place in the image to borrow from.
///
/// A candidate must lie entirely inside the image and must not land on the hole, so it never borrows from the
/// blemish. How well it matches is how alike the two rings' pixels are, mostly ignoring an overall brightness or
/// color shift, because solving for the fill corrects that.
/// - `p_ring`: The ring, as indices into the window.
/// - `p_hole`: Which pixels of the window are the hole.
fn find_source(
  p_ring: &[usize], p_hole: &[bool], p_window: &Window, p_outer: &Window, p_image: (i32, i32),
  p_pixel: &(impl Fn(i32, i32) -> [f32; 3] + Sync),
) -> Option<(i32, i32)> {
  if p_ring.is_empty() {
    return None;
  }

  // Comparing every ring pixel is unnecessary; a spread of them is enough.
  const MAX_SAMPLES: usize = 400;
  let stride = p_ring.len().div_ceil(MAX_SAMPLES);
  let samples: Vec<(i32, i32, [f32; 3])> = p_ring
    .iter()
    .step_by(stride)
    .map(|&local| {
      let (x, y) = (p_window.x + (local % p_window.width) as i32, p_window.y + (local / p_window.width) as i32);
      (x, y, p_pixel(x, y))
    })
    .collect();

  // A spread of the hole's pixels, to check a candidate does not land on it.
  const MAX_HOLE_SAMPLES: usize = 1000;
  let hole_pixels: Vec<usize> = (0..p_hole.len()).filter(|&i| p_hole[i]).collect();
  let hole_stride = hole_pixels.len().div_ceil(MAX_HOLE_SAMPLES).max(1);
  let hole_samples: Vec<(i32, i32)> = hole_pixels
    .iter()
    .step_by(hole_stride)
    .map(|&local| (p_window.x + (local % p_window.width) as i32, p_window.y + (local / p_window.width) as i32))
    .collect();
  let on_hole = |x: i32, y: i32| {
    let (local_x, local_y) = (x - p_window.x, y - p_window.y);
    local_x >= 0
      && local_y >= 0
      && (local_x as usize) < p_window.width
      && (local_y as usize) < p_window.height
      && p_hole[local_y as usize * p_window.width + local_x as usize]
  };

  let (outer_width, outer_height) = (p_outer.width as i32, p_outer.height as i32);
  // Where the pixels of the hole and ring would come from must be in the image and not be the hole. It is the
  // pixels that matter, not the bounding box: a long diagonal stroke's box covers most of the image, but the
  // stroke itself leaves plenty of room beside it.
  let usable = |x: i32, y: i32| x >= 0 && y >= 0 && x < p_image.0 && y < p_image.1 && !on_hole(x, y);
  let valid = |dx: i32, dy: i32| {
    hole_samples.iter().all(|&(x, y)| usable(x + dx, y + dy)) && samples.iter().all(|&(x, y, _)| usable(x + dx, y + dy))
  };

  let cost = |dx: i32, dy: i32| -> f32 {
    let mut sum = [0.0f32; 3];
    let mut sum_squares = [0.0f32; 3];
    for &(x, y, color) in &samples {
      let other = p_pixel(x + dx, y + dy);
      for c in 0..3 {
        let difference = other[c] - color[c];
        sum[c] += difference;
        sum_squares[c] += difference * difference;
      }
    }
    let count = samples.len() as f32;
    (0..3)
      .map(|c| {
        let mean = sum[c] / count;
        let variance = sum_squares[c] / count - mean * mean;
        // Mostly texture, but a candidate that is also the right color is preferred.
        variance + 0.25 * mean * mean
      })
      .sum()
  };

  // Rank by cost, then by offset, so equal costs always resolve the same way.
  let best_of = |candidates: Vec<(i32, i32)>| -> Option<(i32, i32)> {
    candidates
      .into_par_iter()
      .filter(|&(dx, dy)| valid(dx, dy))
      .map(|(dx, dy)| (cost(dx, dy), dx, dy))
      .min_by(|a, b| a.0.total_cmp(&b.0).then((a.1, a.2).cmp(&(b.1, b.2))))
      .map(|(_, dx, dy)| (dx, dy))
  };

  // Search within a reach that keeps the borrowed patch nearby, and never further than the image is wide.
  let reach = 3 * outer_width.max(outer_height);
  let (max_dx, max_dy) = (reach.min(p_image.0), reach.min(p_image.1));
  let (min_dx, min_dy) = (-max_dx, -max_dy);

  // Coarse search over a grid of offsets. Most of the grid is rejected at once, and the offsets that are left can
  // be a narrow band beside a long stroke, so the grid is fine, but kept to a fixed size for a large image.
  const GRID: i32 = 96;
  let span = (max_dx - min_dx).max(max_dy - min_dy);
  let mut step = ((span + GRID - 1) / GRID).max(1);
  let mut coarse = Vec::new();
  let mut dy = min_dy;
  while dy <= max_dy {
    let mut dx = min_dx;
    while dx <= max_dx {
      coarse.push((dx, dy));
      dx += step;
    }
    dy += step;
  }
  let mut best = best_of(coarse)?;

  // Refine around the best one on a finer grid, until it is down to single pixels.
  while step > 1 {
    let spacing = (step / 4).max(1);
    let mut fine = Vec::new();
    let mut dy = -step;
    while dy <= step {
      let mut dx = -step;
      while dx <= step {
        fine.push((best.0 + dx, best.1 + dy));
        dx += spacing;
      }
      dy += spacing;
    }
    best = best_of(fine).unwrap_or(best);
    step = spacing;
  }
  Some(best)
}

/// Solves for the color of every pixel of the hole, one channel at a time.
///
/// Each hole pixel differs from its neighbors the way the borrowed pixels differ from theirs, and pixels just
/// outside the hole are held at their real color. Without a borrowed patch nothing differs, and this fills the
/// hole with a smooth blend of its surroundings.
///
/// Returns colors for the whole window; only those of hole pixels are meaningful.
fn solve(
  p_hole: &[bool], p_window: &Window, p_pixel: &impl Fn(i32, i32) -> [f32; 3], p_source: Option<(i32, i32)>,
) -> Vec<[f32; 3]> {
  let (width, height) = (p_window.width, p_window.height);
  let at = |x: usize, y: usize| p_pixel(p_window.x + x as i32, p_window.y + y as i32);
  // The borrowed pixel at a position, or zero when there is no patch, which leaves nothing to differ.
  let guide = |x: usize, y: usize| -> [f32; 3] {
    match p_source {
      Some((dx, dy)) => p_pixel(p_window.x + x as i32 + dx, p_window.y + y as i32 + dy),
      None => [0.0; 3],
    }
  };

  const NEIGHBORS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
  let neighbors = |x: usize, y: usize| {
    NEIGHBORS.iter().filter_map(move |&(dx, dy)| {
      let (nx, ny) = (x as i32 + dx, y as i32 + dy);
      // Pixels outside the window are outside the image or beyond the ring; treat both as not there.
      (nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height).then_some((nx as usize, ny as usize))
    })
  };

  let mut colors: Vec<[f32; 3]> = (0..width * height).map(|i| at(i % width, i / width)).collect();
  // For each hole pixel: what it is pulled toward (the fixed neighbors' colors plus how it differs from the
  // borrowed ones), and how many neighbors it has.
  let mut targets: Vec<([f32; 3], f32)> = vec![([0.0; 3], 0.0); width * height];
  let mut hole_pixels = Vec::new();
  let mut edge_sum = [0.0f32; 3];
  let mut edge_count = 0.0f32;

  for y in 0..height {
    for x in 0..width {
      let local = y * width + x;
      if !p_hole[local] {
        continue;
      }
      let (mut fixed, mut count) = ([0.0f32; 3], 0.0f32);
      let own_guide = guide(x, y);
      for (nx, ny) in neighbors(x, y) {
        count += 1.0;
        let neighbor_guide = guide(nx, ny);
        for c in 0..3 {
          fixed[c] += own_guide[c] - neighbor_guide[c];
        }
        if !p_hole[ny * width + nx] {
          let color = colors[ny * width + nx];
          for c in 0..3 {
            fixed[c] += color[c];
            edge_sum[c] += color[c];
          }
          edge_count += 1.0;
        }
      }
      targets[local] = (fixed, count);
      hole_pixels.push((x, y));
    }
  }
  if edge_count == 0.0 {
    // The hole has no surroundings to match: leave it as it is.
    return colors;
  }

  // Start from the average surrounding color, so the solver only has to smooth the detail in.
  let average = [
    edge_sum[0] / edge_count,
    edge_sum[1] / edge_count,
    edge_sum[2] / edge_count,
  ];
  for &(x, y) in &hole_pixels {
    colors[y * width + x] = average;
  }

  // Gauss-Seidel with over-relaxation. Information travels a pixel per pass, so a bigger hole needs more of them.
  const OVER_RELAXATION: f32 = 1.9;
  const TOLERANCE: f32 = 0.05;
  let max_passes = (4 * width.max(height)).clamp(200, 4000);
  for _ in 0..max_passes {
    let mut largest_change = 0.0f32;
    for &(x, y) in &hole_pixels {
      let local = y * width + x;
      let (fixed, count) = targets[local];
      let mut sums = fixed;
      for (nx, ny) in neighbors(x, y) {
        if p_hole[ny * width + nx] {
          let neighbor = colors[ny * width + nx];
          for c in 0..3 {
            sums[c] += neighbor[c];
          }
        }
      }
      for c in 0..3 {
        let current = colors[local][c];
        // Colors far outside the displayable range only make the neighbors' updates worse.
        let updated = (current + OVER_RELAXATION * (sums[c] / count - current)).clamp(-32.0, 288.0);
        largest_change = largest_change.max((updated - current).abs());
        colors[local][c] = updated;
      }
    }
    if largest_change < TOLERANCE {
      break;
    }
  }
  colors
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
  use abra_core::Image;

  /// An opaque image whose color at each pixel is `color(x, y)`.
  fn image_of(p_width: u32, p_height: u32, p_color: impl Fn(u32, u32) -> [u8; 3]) -> Image {
    let mut bytes = Vec::new();
    for y in 0..p_height {
      for x in 0..p_width {
        let [r, g, b] = p_color(x, y);
        bytes.extend_from_slice(&[r, g, b, 255]);
      }
    }
    Image::from_rgba_bytes(p_width, p_height, &bytes)
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
  fn follows_the_lighting_of_a_gradient() {
    let base = |x: u32, _y: u32| [(x * 2) as u8, 100, 100];
    let mut image = with_blemish(50, 50, base);
    remover(RemoverShape::circle(6)).with_distance(Units::Pixels(8)).with_position((50, 50)).apply(&mut image);
    // Inside the blemish the red channel should continue the ramp rather than settle on an average.
    assert_close(rgb(&image, 47, 50), base(47, 50), 6);
    assert_close(rgb(&image, 53, 50), base(53, 50), 6);
  }

  #[test]
  fn borrows_texture_when_there_is_some() {
    // Vertical stripes with a period of 6 pixels: filling flat would leave no stripes at all.
    let base = |x: u32, _y: u32| if x % 6 < 3 { [60, 60, 60] } else { [180, 180, 180] };
    let mut image = with_blemish(50, 50, base);
    remover(RemoverShape::circle(6)).with_distance(Units::Pixels(8)).with_position((50, 50)).apply(&mut image);
    let values: Vec<u8> = (46..=54).map(|x| rgb(&image, x, 50)[0]).collect();
    let spread = values.iter().max().unwrap() - values.iter().min().unwrap();
    assert!(spread > 60, "stripes were smoothed away: {values:?}");
  }

  #[test]
  fn only_changes_color_inside_the_shape() {
    let mut image = with_blemish(50, 50, |x, y| [(x * 2) as u8, (y * 2) as u8, 100]);
    let before = image.clone();
    remover(RemoverShape::circle(6)).with_distance(Units::Pixels(8)).with_position((50, 50)).apply(&mut image);
    for y in 0..100 {
      for x in 0..100 {
        let distance = (((x as f32 - 50.0).powi(2) + (y as f32 - 50.0).powi(2)) as f32).sqrt();
        if distance > 7.5 {
          assert_eq!(image.get_pixel(x, y), before.get_pixel(x, y), "pixel ({x}, {y}) changed");
        }
      }
    }
  }

  #[test]
  fn keeps_alpha() {
    let mut bytes = vec![0u8; 100 * 100 * 4];
    for (i, pixel) in bytes.chunks_mut(4).enumerate() {
      pixel.copy_from_slice(&[100, 100, 100, (i % 200) as u8 + 20]);
    }
    let mut image = Image::from_rgba_bytes(100, 100, &bytes);
    remover(RemoverShape::rectangle(10, 10)).with_distance(Units::Pixels(6)).with_position((50, 50)).apply(&mut image);
    for i in 0..100 * 100 {
      assert_eq!(image.rgba()[i * 4 + 3], bytes[i * 4 + 3]);
    }
  }

  #[test]
  fn handles_shapes_at_and_past_the_image_edge() {
    let mut image = image_of(40, 40, |x, y| [(x * 5) as u8, (y * 5) as u8, 0]);
    remover(RemoverShape::circle(8)).with_distance(Units::Pixels(6)).with_position((0, 0)).apply(&mut image);
    remover(RemoverShape::circle(8)).with_distance(Units::Pixels(6)).with_position((39, 20)).apply(&mut image);
    remover(RemoverShape::circle(3))
      .with_distance(Units::Pixels(6))
      .with_position(ShapePosition::line((5, 39), (35, 45)))
      .apply(&mut image);
    // Covers the whole image, so no neighboring patch can fit.
    remover(RemoverShape::rectangle(80, 80)).with_distance(Units::Pixels(6)).with_position((20, 20)).apply(&mut image);
  }

  #[test]
  fn ignores_shapes_that_miss_the_image() {
    let mut image = image_of(40, 40, |x, y| [x as u8, y as u8, 7]);
    let before = image.clone();
    remover(RemoverShape::circle(8)).with_distance(Units::Pixels(6)).with_position((500, 500)).apply(&mut image);
    remover(RemoverShape::circle(0)).with_distance(Units::Pixels(6)).with_position((20, 20)).apply(&mut image);
    assert_eq!(image.rgba(), before.rgba());
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
    assert_eq!(stamp.positions, vec![ShapePosition::Path(vec![point(0.0, 1.0), point(2.0, 3.0), point(4.0, 5.0)])]);
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
      vec![ShapePosition::Path(vec![PointF::new(10, 10), PointF::new(20, 10), PointF::new(20, 30)])]
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
  fn a_long_diagonal_stroke_still_finds_texture_to_borrow() {
    // Stripes, with a diagonal blemish across the middle. Its bounding box covers most of the image, so there is
    // nowhere for a bounding box to fit beside it, but the pixels beside the stroke itself are free to borrow.
    let base = |x: u32, _y: u32| if x % 8 < 4 { [60, 60, 60] } else { [180, 180, 180] };
    let mut image = image_of(160, 160, |x, y| if x.abs_diff(y) <= 1 && (30..=130).contains(&x) { BLEMISH } else { base(x, y) });
    remover(RemoverShape::circle(3))
      .with_distance(Units::Pixels(6))
      .with_hardness(1.0)
      .with_path([(30, 30), (130, 130)])
      .apply(&mut image);
    for i in [40u32, 60, 90, 120] {
      let healed = rgb(&image, i, i)[0] as i32;
      let expected = base(i, i)[0] as i32;
      assert!((healed - expected).abs() <= 40, "at ({i}, {i}) got {healed}, wanted about {expected}");
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

