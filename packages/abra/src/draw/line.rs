#![allow(unused_imports, unused_variables, unused_mut)]
use crate::{
  color::{Color, Fill},
  draw::shapes,
  geometry::{
    line::{bresenham, bresenham_from_points},
    path::{BezierCurve, CurveType, Path, Rect},
    point::Point,
  },
  image::Image,
};

#[derive(Clone)]
/// Line cap styles.
pub enum LineCap {
  /// The line ends exactly at the endpoint.
  Butt,
  /// The line ends with a rounded cap.
  Round,
  /// The line ends with a square cap extending beyond the endpoint.
  Square,
}

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
    println!("points {:?}", points.len());
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

/// Draws a straight line following the given path at the given starting point.
/// - `image` - The image to draw on.
/// - `start` - The starting point of where the line should start.
/// - `path` - The path to follow relative to the starting point.
/// - `color` - The color of the line.
pub fn line(image: &mut Image, start: Point, path: Path, fill: Fill, width: u32, cap: Option<LineCap>) {
  let mut last = start;
  // let c = fill.as_u8();
  let c = match fill {
    Fill::Solid(color) => color,
    _ => Color::black(),
  }
  .as_u8();
  let (img_width, img_height) = image.dimensions::<u32>();
  let path_points = path.get_points().clone();

  for point in path_points.iter() {
    let p: Point = *point + start;
    let dx = p.x() - last.x();
    let dy = p.y() - last.y();
    if dx == 0 && dy == 0 {
      last = p;
      continue;
    }
    let perp_x = -dy as f32;
    let perp_y = dx as f32;
    let perp_length = (perp_x * perp_x + perp_y * perp_y).sqrt();
    let unit_perp_x = perp_x / perp_length;
    let unit_perp_y = perp_y / perp_length;
    for offset in -(width as i32 / 2)..=(width as i32 / 2) {
      let offset_x = unit_perp_x * offset as f32;
      let offset_y = unit_perp_y * offset as f32;
      let start_offset =
        Point::new((last.x() as f32 + offset_x).round() as i32, (last.y() as f32 + offset_y).round() as i32);
      let end_offset = Point::new((p.x() as f32 + offset_x).round() as i32, (p.y() as f32 + offset_y).round() as i32);
      let new_points = bresenham_from_points(start_offset, end_offset);
      for (x, y) in new_points.iter() {
        if *x >= 0 && *x < img_width as i32 && *y >= 0 && *y < img_height as i32 {
          image.set_pixel(*x as u32, *y as u32, c);
        }
      }
    }
    last = p;
  }

  // Add caps only at the start and end of the entire path
  let cap_type = cap.clone().unwrap_or(LineCap::Butt);
  if path_points.len() > 0 {
    let path_start = *path_points.first().unwrap() + start;
    let path_end = last;
    draw_caps(image, path_start, path_end, width, cap_type, fill);
  }
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

fn draw_caps(image: &mut Image, start: Point, end: Point, width: u32, cap: LineCap, fill: Fill) {
  if width < 1 {
    return;
  }
  let radius = (width / 2) as u32;
  let size = Path::new_rect(radius * 2, radius * 2);
  match cap {
    LineCap::Round => {
      // ellipse_filled expects the center point directly
      shapes::ellipse_filled(image, start, size.clone(), fill.clone());
      shapes::ellipse_filled(image, end, size, fill.clone());
    }
    LineCap::Square => {
      let half_width = (width as i32) / 2;
      let rect = Path::new(vec![
        Point::new(0, 0),
        Point::new(width as i32, 0),
        Point::new(width as i32, width as i32),
        Point::new(0, width as i32),
      ]);
      let start_point = Point::new(start.x() - half_width, start.y() - half_width);
      let end_point = Point::new(end.x() - half_width, end.y() - half_width);
      shapes::rect(image, start_point, rect.clone(), fill.clone());
      shapes::rect(image, end_point, rect.clone(), fill.clone());
    }
    LineCap::Butt => {
      // No additional drawing needed for butt caps
    }
  }
}
