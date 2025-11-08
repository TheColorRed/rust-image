#![allow(unused_imports, unused_variables, unused_mut)]
use crate::{
  color::Color,
  geometry::{
    line::{bresenham, bresenham_from_points},
    path::{BezierCurve, CurveType},
    point::Point,
  },
  image::Image,
};

/// Draws a curved line following the given path at the given starting point.
/// - `image` - The image to draw on.
/// - `start` - The starting point of where the curve should start.
/// - `path` - The path to follow relative to the starting point.
/// - `color` - The color of the line.
pub fn curve_from<T>(image: &mut Image, start: Point, path: T, color: Color)
where
  T: BezierCurve + Sync,
{
  let mut pixels = image.rgba();
  let (width, height) = image.dimensions::<u32>();
  let (start_x, start_y): (f32, f32) = start.into();
  let mut last = start;
  let curve_type = path.get_type();

  if curve_type == CurveType::Three {
    let points = get_curve_points_three(image, start, path);
    println!("{:?}", points.len());
    image.set_rgba(pixels);
  }
}

/// Draws a curved line following the given path.
/// - `image` - The image to draw on.
/// - `path` - The path to follow relative to the starting point.
/// - `color` - The color of the line.
pub fn curve<T>(image: &mut Image, path: T, color: Color)
where
  T: BezierCurve + Sync,
{
  curve_from(image, Point::new(0, 0), path, color);
}

fn get_curve_points_three<T>(image: &mut Image, start: Point, path: T) -> Vec<Point>
where
  T: BezierCurve + Sync,
{
  let (width, height) = image.dimensions::<u32>();
  let (start_x, start_y): (f32, f32) = start.into();
  let curve_type = path.get_type();

  vec![]
}
