use crate::Path;

use super::pointf::PointF;

/// The direction a line segment can be turned to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
  /// Level with the x-axis (0°).
  Horizontal,
  /// Level with the y-axis (90°).
  Vertical,
  /// A specific angle in degrees, using the same convention as `LineSegment::degrees`.
  Angle(f64),
}

impl Orientation {
  /// The target angle in degrees.
  fn degrees(self) -> f64 {
    match self {
      Orientation::Horizontal => 0.0,
      Orientation::Vertical => 90.0,
      Orientation::Angle(degrees) => degrees,
    }
  }
}

/// A straight line segment running from one point to another, with helpers for its angle, length and points along it.
/// Angles are measured from the positive x-axis; with y pointing down, positive values are clockwise.
///
/// Converts into `f64` degrees so it can be passed anywhere an angle is accepted,
/// e.g. `layer.transform().rotate(LineSegment::new((0, 0), (width, height)), None)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSegment {
  start: PointF,
  end: PointF,
}

impl Into<Path> for LineSegment {
  fn into(self) -> Path {
    Path::line(self.start, self.end)
  }
}

impl LineSegment {
  /// Creates the line from `p_start` to `p_end`.
  pub fn new(p_start: impl Into<PointF>, p_end: impl Into<PointF>) -> Self {
    LineSegment {
      start: p_start.into(),
      end: p_end.into(),
    }
  }

  /// The starting point.
  pub fn start(&self) -> PointF {
    self.start
  }

  /// The ending point.
  pub fn end(&self) -> PointF {
    self.end
  }

  /// The same line running the opposite way.
  pub fn reversed(&self) -> LineSegment {
    LineSegment {
      start: self.end,
      end: self.start,
    }
  }

  /// The offset from the start point to the end point.
  pub fn delta(&self) -> PointF {
    self.end - self.start
  }

  /// The angle in degrees, in the range `(-180, 180]`.
  pub fn degrees(&self) -> f64 {
    self.radians().to_degrees()
  }

  /// The angle in radians, in the range `(-π, π]`.
  pub fn radians(&self) -> f64 {
    let delta = self.delta();
    (delta.y as f64).atan2(delta.x as f64)
  }

  /// The angle in degrees, wrapped into the range `[0, 360)`.
  pub fn degrees_positive(&self) -> f64 {
    self.degrees().rem_euclid(360.0)
  }

  /// The rotation in degrees that turns this line to `p_orientation`.
  /// A line and its reverse are the same line, so the target only matters modulo 180 and the
  /// result is wrapped into `(-90, 90]`.
  pub fn correction_to(&self, p_orientation: Orientation) -> f64 {
    let mut correction = p_orientation.degrees() - self.degrees();
    if correction > 90.0 {
      correction -= 180.0;
    } else if correction <= -90.0 {
      correction += 180.0;
    }
    correction
  }

  /// The distance between the start and end points.
  pub fn distance(&self) -> f64 {
    self.distance_squared().sqrt()
  }

  /// The squared distance between the start and end points. Cheaper than `distance` for comparisons.
  pub fn distance_squared(&self) -> f64 {
    self.delta().length_squared() as f64
  }

  /// Whether the start and end points are the same, so the line has no direction.
  pub fn is_zero_length(&self) -> bool {
    self.start == self.end
  }

  /// The slope (`dy / dx`), or `None` when the line is vertical.
  pub fn slope(&self) -> Option<f64> {
    let delta = self.delta();
    if delta.x == 0.0 { None } else { Some((delta.y / delta.x) as f64) }
  }

  /// The point halfway between the start and end points.
  pub fn midpoint(&self) -> PointF {
    self.point_at(0.5)
  }

  /// The point at `p_t` along the line: `0.0` is the start, `1.0` is the end.
  /// Values outside `0..=1` extend past either end.
  pub fn point_at(&self, p_t: f32) -> PointF {
    self.start.lerp(self.end, p_t)
  }
}

impl From<LineSegment> for f64 {
  fn from(p_angle: LineSegment) -> Self {
    p_angle.degrees()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn degrees_follow_the_line_direction() {
    assert_eq!(LineSegment::new((0, 0), (10, 0)).degrees(), 0.0);
    assert_eq!(LineSegment::new((0, 0), (0, 10)).degrees(), 90.0);
    assert_eq!(LineSegment::new((0, 0), (-10, 0)).degrees(), 180.0);
    assert_eq!(LineSegment::new((0, 0), (0, -10)).degrees(), -90.0);
    assert_eq!(LineSegment::new((0, 0), (0, -10)).degrees_positive(), 270.0);
  }

  #[test]
  fn distance_and_slope() {
    let line = LineSegment::new((0, 0), (3, 4));
    assert_eq!(line.distance(), 5.0);
    assert_eq!(line.distance_squared(), 25.0);
    assert!((line.slope().unwrap() - 4.0 / 3.0).abs() < 1e-6);
    assert_eq!(LineSegment::new((2, 0), (2, 9)).slope(), None);
  }

  #[test]
  fn points_along_the_line() {
    let line = LineSegment::new((0, 0), (10, 20));
    assert_eq!(line.midpoint(), PointF::new(5, 10));
    assert_eq!(line.point_at(0.0), line.start());
    assert_eq!(line.point_at(1.0), line.end());
    assert_eq!(line.reversed().start(), line.end());
    assert!(LineSegment::new((1, 1), (1, 1)).is_zero_length());
  }

  #[test]
  fn correcting_to_horizontal_takes_the_shortest_turn() {
    assert_eq!(LineSegment::new((0, 0), (10, 0)).correction_to(Orientation::Horizontal), 0.0);
    assert!((LineSegment::new((0, 0), (10, 10)).correction_to(Orientation::Horizontal) + 45.0).abs() < 1e-4);
    // Reversing the line must not change the correction.
    let line = LineSegment::new((0, 0), (10, 5));
    assert!((line.correction_to(Orientation::Horizontal) - line.reversed().correction_to(Orientation::Horizontal)).abs() < 1e-4);
    // Vertical lines turn by 90, never -90.
    assert_eq!(LineSegment::new((0, 0), (0, -10)).correction_to(Orientation::Horizontal), 90.0);
  }

  #[test]
  fn correcting_to_vertical_takes_the_shortest_turn() {
    assert_eq!(LineSegment::new((0, 0), (0, 10)).correction_to(Orientation::Vertical), 0.0);
    assert_eq!(LineSegment::new((0, 0), (0, -10)).correction_to(Orientation::Vertical), 0.0);
    assert!((LineSegment::new((0, 0), (10, 10)).correction_to(Orientation::Vertical) - 45.0).abs() < 1e-4);
    assert!((LineSegment::new((0, 0), (-10, 10)).correction_to(Orientation::Vertical) + 45.0).abs() < 1e-4);
    // Horizontal lines are a tie between 90 and -90; the tie goes to +90, matching the vertical-line tie for `Horizontal`.
    assert_eq!(LineSegment::new((0, 0), (10, 0)).correction_to(Orientation::Vertical), 90.0);
    // Reversing the line must not change the correction.
    let line = LineSegment::new((0, 0), (5, 10));
    assert!((line.correction_to(Orientation::Vertical) - line.reversed().correction_to(Orientation::Vertical)).abs() < 1e-4);
  }

  #[test]
  fn applying_the_correction_lands_on_the_target_angle() {
    for end in [(10, 3), (-7, 4), (2, -9), (-5, -5), (0, 10), (10, 0)] {
      let line = LineSegment::new((0, 0), end);
      // After rotating by the correction, the line must be horizontal (or vertical) modulo 180.
      let horizontal = (line.degrees() + line.correction_to(Orientation::Horizontal)).rem_euclid(180.0);
      let vertical = (line.degrees() + line.correction_to(Orientation::Vertical)).rem_euclid(180.0);
      assert!(horizontal < 1e-4 || (180.0 - horizontal) < 1e-4, "horizontal {end:?}: {horizontal}");
      assert!((vertical - 90.0).abs() < 1e-4, "vertical {end:?}: {vertical}");
    }
  }

  #[test]
  fn correction_to_a_custom_angle() {
    // A 45° line turned to 30° needs -15°; horizontal needs -45° and vertical +45°.
    let line = LineSegment::new((0, 0), (10, 10));
    assert!((line.correction_to(Orientation::Angle(30.0)) + 15.0).abs() < 1e-4);
    assert!((line.correction_to(Orientation::Horizontal) + 45.0).abs() < 1e-4);
    assert!((line.correction_to(Orientation::Vertical) - 45.0).abs() < 1e-4);
    // Angles are modulo 180: 210° is the same direction as 30°.
    assert!(
      (line.correction_to(Orientation::Angle(210.0)) - line.correction_to(Orientation::Angle(30.0))).abs() < 1e-4
    );
  }
}
