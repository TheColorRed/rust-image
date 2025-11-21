use crate::Area;

/// A heart shape defined using cubic Bezier curves.
pub struct Heart;

impl Heart {
  /// Create a new heart shape.
  pub fn new() -> Area {
    let mut path = Area::new();

    path
      .move_to((50.0, 15.0))
      .cubic_to((35.0, 0.0), (0.0, 0.0), (0.0, 37.5))
      .cubic_to((0.0, 75.0), (25.0, 95.0), (50.0, 120.0))
      .cubic_to((75.0, 95.0), (100.0, 75.0), (100.0, 37.5))
      .cubic_to((100.0, 0.0), (65.0, 0.0), (50.0, 15.0));

    path
  }
}
