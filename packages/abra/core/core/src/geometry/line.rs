use super::pointf::PointF;

/// The pixels on the straight line between two points, using Bresenham's line algorithm. Both ends are included.
/// Points are rounded to the nearest pixel.
/// - `p_from`: The start point.
/// - `p_to`: The end point.
pub fn bresenham(p_from: impl Into<PointF>, p_to: impl Into<PointF>) -> Vec<(i32, i32)> {
  let ((x0, y0), (x1, y1)): ((i32, i32), (i32, i32)) = (p_from.into().into(), p_to.into().into());
  let mut result = Vec::new();
  let dx = (x1 - x0).abs();
  let dy = -(y1 - y0).abs();
  let sx = if x0 < x1 { 1 } else { -1 };
  let sy = if y0 < y1 { 1 } else { -1 };
  let mut err = dx + dy;
  let (mut x, mut y) = (x0, y0);
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
