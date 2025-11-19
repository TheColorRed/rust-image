use super::point::Point;

/// Bresenham's line algorithm
pub fn bresenham(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
  let mut result = Vec::new();
  let dx = (x1 - x0).abs();
  let dy = -(y1 - y0).abs();
  let sx = if x0 < x1 { 1 } else { -1 };
  let sy = if y0 < y1 { 1 } else { -1 };
  let mut err = dx + dy;
  let mut x = x0;
  let mut y = y0;
  while x != x1 || y != y1 {
    result.push((x, y));
    let e2 = 2 * err;
    if e2 >= dy {
      err += dy;
      x += sx;
    }
    if e2 <= dx {
      err += dx;
      y += sy;
    }
  }
  result.push((x, y));
  result
}

/// Bresenham's line algorithm from points
pub fn bresenham_from_points(p0: Point, p1: Point) -> Vec<(i32, i32)> {
  bresenham(p0.x(), p0.y(), p1.x(), p1.y())
}
