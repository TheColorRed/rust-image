use std::fmt::Display;
use std::ops::{Add, Mul};

use crate::IntoNumber;

#[derive(Debug, Clone, Copy, PartialEq)]
/// A point in a 2D space.
pub struct Point {
  /// The x-coordinate of the point.
  x: i32,
  /// The y-coordinate of the point.
  y: i32,
}

impl Display for Point {
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(p_f, "({}, {})", self.x, self.y)
  }
}

impl<A: IntoNumber, B: IntoNumber> From<(A, B)> for Point {
  fn from(p_tuple: (A, B)) -> Self {
    Point {
      x: p_tuple.0.into::<i32>(),
      y: p_tuple.1.into::<i32>(),
    }
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

/// Multiply a point by a scalar
impl Mul<f32> for Point {
  type Output = Point;

  fn mul(self, p_rhs: f32) -> Point {
    Point::new((self.x() as f32 * p_rhs) as i32, (self.y() as f32 * p_rhs) as i32)
  }
}

impl Add<Point> for Point {
  type Output = Point;

  fn add(self, p_rhs: Point) -> Point {
    Point::new(self.x() + p_rhs.x(), self.y() + p_rhs.y())
  }
}

impl Add<i32> for Point {
  type Output = Point;

  fn add(self, p_rhs: i32) -> Point {
    Point::new(self.x() + p_rhs, self.y() + p_rhs)
  }
}

/// Multiply two points together.
impl Mul<Point> for Point {
  type Output = Point;

  fn mul(self, p_rhs: Point) -> Point {
    Point::new(self.x() * p_rhs.x(), self.y() * p_rhs.y())
  }
}

impl Point {
  /// Creates a new point with the given coordinates.
  pub fn new(p_x: i32, p_y: i32) -> Point {
    Point { x: p_x, y: p_y }
  }
  /// Creates an array of points from a vector of points or tuples.
  pub fn array(p_points: Vec<impl Into<Point>>) -> Vec<Point> {
    p_points.into_iter().map(|p| p.into()).collect()
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
