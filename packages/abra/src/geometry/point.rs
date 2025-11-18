use std::fmt::Display;
use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy, PartialEq)]
/// A point in a 2D space.
pub struct Point {
  /// The x-coordinate of the point.
  x: i32,
  /// The y-coordinate of the point.
  y: i32,
}

impl Display for Point {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({}, {})", self.x, self.y)
  }
}

impl Into<(i32, i32)> for Point {
  fn into(self) -> (i32, i32) {
    (self.x, self.y)
  }
}

impl Into<(f32, f32)> for Point {
  fn into(self) -> (f32, f32) {
    (self.x as f32, self.y as f32)
  }
}

impl From<(i32, i32)> for Point {
  fn from(coords: (i32, i32)) -> Point {
    Point::new(coords.0, coords.1)
  }
}

impl From<(f32, f32)> for Point {
  fn from(coords: (f32, f32)) -> Point {
    Point::new(coords.0 as i32, coords.1 as i32)
  }
}

/// Multiply a point by a scalar
impl Mul<f32> for Point {
  type Output = Point;

  fn mul(self, rhs: f32) -> Point {
    Point::new((self.x() as f32 * rhs) as i32, (self.y() as f32 * rhs) as i32)
  }
}

impl Add<Point> for Point {
  type Output = Point;

  fn add(self, rhs: Point) -> Point {
    Point::new(self.x() + rhs.x(), self.y() + rhs.y())
  }
}

impl Add<i32> for Point {
  type Output = Point;

  fn add(self, rhs: i32) -> Point {
    Point::new(self.x() + rhs, self.y() + rhs)
  }
}

/// Multiply two points together.
impl Mul<Point> for Point {
  type Output = Point;

  fn mul(self, rhs: Point) -> Point {
    Point::new(self.x() * rhs.x(), self.y() * rhs.y())
  }
}

impl Point {
  /// Creates a new point with the given coordinates.
  pub fn new(x: i32, y: i32) -> Point {
    Point { x, y }
  }

  /// Creates a new point at the origin (0, 0).
  pub fn default() -> Point {
    Point { x: 0, y: 0 }
  }

  /// Gets the x-coordinate of the point.
  pub fn x(&self) -> i32 {
    self.x
  }

  /// Gets the y-coordinate of the point.
  pub fn y(&self) -> i32 {
    self.y
  }

  /// Gets the dimensions of the point.
  pub fn dimensions(&self) -> (i32, i32) {
    (self.x, self.y)
  }
}
