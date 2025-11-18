use super::super::Area;

/// A heart shape defined using cubic Bezier curves.
pub struct Polygon;

impl Polygon {
  /// Create a new polygon shape.
  /// - `p_sides`: The number of sides of the polygon.
  pub fn new(p_sides: usize) -> Area {
    if p_sides < 3 {
      panic!("A polygon must have at least 3 sides");
    }

    let mut path = Area::new();

    let angle_step = 360.0 / p_sides as f32;
    let radius = 50.0;

    let mut first: Option<(f32, f32)> = None;
    for i in 0..p_sides {
      let angle_deg = i as f32 * angle_step - 90.0; // Start from the top
      let angle_rad = angle_deg.to_radians();
      let x = radius * angle_rad.cos() + radius;
      let y = radius * angle_rad.sin() + radius;

      if i == 0 {
        path.with_move_to((x, y));
        first = Some((x, y));
      } else {
        path.with_line_to((x, y));
      }
    }

    // Explicitly close by returning to the first point
    if let Some((fx, fy)) = first {
      path.with_line_to((fx, fy));
    }

    path
  }
}
