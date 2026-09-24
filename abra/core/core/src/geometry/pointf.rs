use std::fmt::Display;
use std::ops::{Add, Div, Mul, Sub};

use crate::{IntoNumber, LineSegment};

use super::point::Point;

#[derive(Clone, Copy, Debug, PartialEq)]
/// A point in 2D space with floating-point coordinates.
/// Used for precise geometric calculations before rasterization.
pub struct PointF {
  /// The x-coordinate of the point.
  pub x: f32,
  /// The y-coordinate of the point.
  pub y: f32,
}

impl Display for PointF {
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(p_f, "({}, {})", self.x, self.y)
  }
}

impl Default for PointF {
  fn default() -> Self {
    PointF { x: 0.0, y: 0.0 }
  }
}

impl<A: IntoNumber, B: IntoNumber> From<(A, B)> for PointF {
  fn from(p_tuple: (A, B)) -> Self {
    PointF {
      x: p_tuple.0.into::<f32>(),
      y: p_tuple.1.into::<f32>(),
    }
  }
}

// Conversions to tuples
impl From<PointF> for (f32, f32) {
  fn from(p_p: PointF) -> Self {
    (p_p.x, p_p.y)
  }
}

impl From<PointF> for (i32, i32) {
  fn from(p_p: PointF) -> Self {
    (p_p.x.round() as i32, p_p.y.round() as i32)
  }
}

impl From<PointF> for (u32, u32) {
  fn from(p_p: PointF) -> Self {
    (p_p.x.round() as u32, p_p.y.round() as u32)
  }
}

impl From<PointF> for (f64, f64) {
  fn from(p_p: PointF) -> Self {
    (p_p.x as f64, p_p.y as f64)
  }
}

// Conversions from/to Point (integer)
impl From<Point> for PointF {
  fn from(p_p: Point) -> Self {
    PointF {
      x: p_p.x() as f32,
      y: p_p.y() as f32,
    }
  }
}

impl From<PointF> for Point {
  fn from(p_p: PointF) -> Self {
    Point::new(p_p.x.round() as i32, p_p.y.round() as i32)
  }
}

// Arithmetic operations
impl Add for PointF {
  type Output = PointF;

  fn add(self, p_rhs: PointF) -> PointF {
    PointF {
      x: self.x + p_rhs.x,
      y: self.y + p_rhs.y,
    }
  }
}

impl Sub for PointF {
  type Output = PointF;

  fn sub(self, p_rhs: PointF) -> PointF {
    PointF {
      x: self.x - p_rhs.x,
      y: self.y - p_rhs.y,
    }
  }
}

impl Mul<f32> for PointF {
  type Output = PointF;

  fn mul(self, p_scalar: f32) -> PointF {
    PointF {
      x: self.x * p_scalar,
      y: self.y * p_scalar,
    }
  }
}

impl Mul<PointF> for f32 {
  type Output = PointF;

  fn mul(self, p_point: PointF) -> PointF {
    PointF {
      x: self * p_point.x,
      y: self * p_point.y,
    }
  }
}

impl Div<f32> for PointF {
  type Output = PointF;

  fn div(self, p_scalar: f32) -> PointF {
    PointF {
      x: self.x / p_scalar,
      y: self.y / p_scalar,
    }
  }
}

impl PointF {
  /// Creates a new point with the given coordinates.
  pub fn new(p_x: impl IntoNumber, p_y: impl IntoNumber) -> PointF {
    PointF {
      x: p_x.into::<f32>(),
      y: p_y.into::<f32>(),
    }
  }

  /// Creates a new point at the origin (0, 0).
  pub fn zero() -> PointF {
    PointF { x: 0.0, y: 0.0 }
  }

  /// Returns the length (magnitude) of the vector from origin to this point.
  pub fn length(&self) -> f32 {
    (self.x * self.x + self.y * self.y).sqrt()
  }

  /// Returns the squared length (avoids sqrt for performance).
  pub fn length_squared(&self) -> f32 {
    self.x * self.x + self.y * self.y
  }

  /// Returns a normalized version of this point (unit vector).
  pub fn normalize(&self) -> PointF {
    let len = self.length();
    if len > 0.0 {
      PointF {
        x: self.x / len,
        y: self.y / len,
      }
    } else {
      PointF::zero()
    }
  }

  /// Returns the dot product with another point.
  pub fn dot(&self, p_other: PointF) -> f32 {
    self.x * p_other.x + self.y * p_other.y
  }

  /// Returns the cross product magnitude (z-component in 2D).
  pub fn cross(&self, p_other: PointF) -> f32 {
    self.x * p_other.y - self.y * p_other.x
  }

  /// Returns the distance to another point.
  pub fn distance_to(&self, p_other: PointF) -> f32 {
    (*self - p_other).length()
  }

  /// Returns the perpendicular vector (rotated 90 degrees counter-clockwise).
  pub fn perpendicular(&self) -> PointF {
    PointF { x: -self.y, y: self.x }
  }

  /// Linear interpolation between this point and another.
  pub fn lerp(&self, p_other: PointF, p_t: f32) -> PointF {
    *self + (p_other - *self) * p_t
  }

  pub fn angle(&self, p_other: PointF) -> f32 {
    LineSegment::new(*self, p_other).degrees() as f32
  }
}
