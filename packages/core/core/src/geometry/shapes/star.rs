use crate::Area;

/// A star shape defined using line segments.
pub struct Star;

impl Star {
  /// Create a new star shape.
  pub fn new() -> Area {
    let mut path = Area::new();

    path
      .with_move_to((50.0, 0.0))
      .with_line_to((61.8, 35.1))
      .with_line_to((100.0, 38.2))
      .with_line_to((69.1, 61.8))
      .with_line_to((80.9, 100.0))
      .with_line_to((50.0, 76.4))
      .with_line_to((19.1, 100.0))
      .with_line_to((30.9, 61.8))
      .with_line_to((0.0, 38.2))
      .with_line_to((38.2, 35.1))
      .with_line_to((50.0, 0.0));

    path
  }
}
