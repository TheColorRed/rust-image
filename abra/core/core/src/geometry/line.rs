use super::point::Point;

/// Bresenham's line algorithm
pub fn bresenham(p_x0: i32, p_y0: i32, p_x1: i32, p_y1: i32) -> Vec<(i32, i32)> {
  let mut result = Vec::new();
  let dx = (p_x1 - p_x0).abs();
  let dy = -(p_y1 - p_y0).abs();
  let sx = if p_x0 < p_x1 { 1 } else { -1 };
  let sy = if p_y0 < p_y1 { 1 } else { -1 };
  let mut err = dx + dy;
  let mut x = p_x0;
  let mut y = p_y0;
  while x != p_x1 || y != p_y1 {
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
pub fn bresenham_from_points(p_p0: Point, p_p1: Point) -> Vec<(i32, i32)> {
  bresenham(p_p0.x(), p_p0.y(), p_p1.x(), p_p1.y())
}
