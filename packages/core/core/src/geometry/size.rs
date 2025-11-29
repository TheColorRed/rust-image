use std::ops::{Add, Div, Mul, Sub};

use crate::FromF32;

#[derive(Debug, Clone, Copy, PartialEq)]
/// A point in 2D space with floating-point coordinates.
/// Used for precise geometric calculations before rasterization.
pub struct Size {
  /// The width component of the size.
  pub width: f32,
  /// The height component of the size.
  pub height: f32,
}

impl Size {
  /// Creates a new size with the given width and height.
  pub fn new(width: impl Into<f64>, height: impl Into<f64>) -> Size {
    Size {
      width: width.into() as f32,
      height: height.into() as f32,
    }
  }

  /// Converts the Size to a tuple of (width, height).
  pub fn to_tuple<S: FromF32>(&self) -> (S, S) {
    (S::from_f32(self.width), S::from_f32(self.height))
  }
}

impl From<(f32, f32)> for Size {
  fn from(size_tuple: (f32, f32)) -> Self {
    Size {
      width: size_tuple.0,
      height: size_tuple.1,
    }
  }
}

impl From<(u32, u32)> for Size {
  fn from(size_tuple: (u32, u32)) -> Self {
    Size {
      width: size_tuple.0 as f32,
      height: size_tuple.1 as f32,
    }
  }
}

impl From<(i32, i32)> for Size {
  fn from(size_tuple: (i32, i32)) -> Self {
    Size {
      width: size_tuple.0 as f32,
      height: size_tuple.1 as f32,
    }
  }
}

impl<T: Into<f64>> Sub<T> for Size {
  type Output = Size;

  fn sub(self, rhs: T) -> Self::Output {
    let rhs = rhs.into();
    Size {
      width: self.width - rhs as f32,
      height: self.height - rhs as f32,
    }
  }
}

impl Sub<Size> for Size {
  type Output = Size;

  fn sub(self, rhs: Size) -> Self::Output {
    Size {
      width: self.width - rhs.width,
      height: self.height - rhs.height,
    }
  }
}

impl<T: Into<f64>> Add<T> for Size {
  type Output = Size;

  fn add(self, rhs: T) -> Self::Output {
    let rhs = rhs.into();
    Size {
      width: self.width + rhs as f32,
      height: self.height + rhs as f32,
    }
  }
}

impl Add<Size> for Size {
  type Output = Size;

  fn add(self, rhs: Size) -> Self::Output {
    Size {
      width: self.width + rhs.width,
      height: self.height + rhs.height,
    }
  }
}

impl<T: Into<f64>> Mul<T> for Size {
  type Output = Size;

  fn mul(self, rhs: T) -> Self::Output {
    let rhs = rhs.into();
    Size {
      width: self.width * rhs as f32,
      height: self.height * rhs as f32,
    }
  }
}

impl Mul<Size> for f32 {
  type Output = Size;

  fn mul(self, rhs: Size) -> Self::Output {
    Size {
      width: rhs.width * self,
      height: rhs.height * self,
    }
  }
}

impl<T: Into<f64>> Div<T> for Size {
  type Output = Size;

  fn div(self, rhs: T) -> Self::Output {
    let rhs = rhs.into();
    Size {
      width: self.width / rhs as f32,
      height: self.height / rhs as f32,
    }
  }
}

impl Div<Size> for f32 {
  type Output = Size;

  fn div(self, rhs: Size) -> Self::Output {
    Size {
      width: rhs.width / self,
      height: rhs.height / self,
    }
  }
}
