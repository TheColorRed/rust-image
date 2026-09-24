use std::ops::{Add, Div, Mul, Sub};

use crate::{FromF32, IntoNumber};

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
  pub fn new(p_width: impl IntoNumber, p_height: impl IntoNumber) -> Size {
    Size {
      width: p_width.into::<f32>(),
      height: p_height.into::<f32>(),
    }
  }

  /// Converts the Size to a tuple of (width, height).
  pub fn to_tuple<S: FromF32>(&self) -> (S, S) {
    (S::from_f32(self.width), S::from_f32(self.height))
  }
}

impl From<(f32, f32)> for Size {
  fn from(p_size_tuple: (f32, f32)) -> Self {
    Size {
      width: p_size_tuple.0,
      height: p_size_tuple.1,
    }
  }
}

impl From<(f64, f64)> for Size {
  fn from(p_size_tuple: (f64, f64)) -> Self {
    Size {
      width: p_size_tuple.0 as f32,
      height: p_size_tuple.1 as f32,
    }
  }
}

impl From<(u32, u32)> for Size {
  fn from(p_size_tuple: (u32, u32)) -> Self {
    Size {
      width: p_size_tuple.0 as f32,
      height: p_size_tuple.1 as f32,
    }
  }
}

impl From<(i32, i32)> for Size {
  fn from(p_size_tuple: (i32, i32)) -> Self {
    Size {
      width: p_size_tuple.0 as f32,
      height: p_size_tuple.1 as f32,
    }
  }
}

impl From<Size> for (f32, f32) {
  fn from(p_size: Size) -> Self {
    (p_size.width, p_size.height)
  }
}

impl<T: Into<f64>> Sub<T> for Size {
  type Output = Size;

  fn sub(self, p_rhs: T) -> Self::Output {
    let p_rhs = p_rhs.into();
    Size {
      width: self.width - p_rhs as f32,
      height: self.height - p_rhs as f32,
    }
  }
}

impl Sub<Size> for Size {
  type Output = Size;

  fn sub(self, p_rhs: Size) -> Self::Output {
    Size {
      width: self.width - p_rhs.width,
      height: self.height - p_rhs.height,
    }
  }
}

impl<T: Into<f64>> Add<T> for Size {
  type Output = Size;

  fn add(self, p_rhs: T) -> Self::Output {
    let p_rhs = p_rhs.into();
    Size {
      width: self.width + p_rhs as f32,
      height: self.height + p_rhs as f32,
    }
  }
}

impl Add<Size> for Size {
  type Output = Size;

  fn add(self, p_rhs: Size) -> Self::Output {
    Size {
      width: self.width + p_rhs.width,
      height: self.height + p_rhs.height,
    }
  }
}

impl<T: Into<f64>> Mul<T> for Size {
  type Output = Size;

  fn mul(self, p_rhs: T) -> Self::Output {
    let p_rhs = p_rhs.into();
    Size {
      width: self.width * p_rhs as f32,
      height: self.height * p_rhs as f32,
    }
  }
}

impl Mul<Size> for f32 {
  type Output = Size;

  fn mul(self, p_rhs: Size) -> Self::Output {
    Size {
      width: p_rhs.width * self,
      height: p_rhs.height * self,
    }
  }
}

impl<T: Into<f64>> Div<T> for Size {
  type Output = Size;

  fn div(self, p_rhs: T) -> Self::Output {
    let p_rhs = p_rhs.into();
    Size {
      width: self.width / p_rhs as f32,
      height: self.height / p_rhs as f32,
    }
  }
}

impl Div<Size> for f32 {
  type Output = Size;

  fn div(self, p_rhs: Size) -> Self::Output {
    Size {
      width: p_rhs.width / self,
      height: p_rhs.height / self,
    }
  }
}
