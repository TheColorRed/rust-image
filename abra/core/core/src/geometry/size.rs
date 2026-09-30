use std::ops::{Add, Div, Mul, Sub};

use crate::{FromF32, IntoNumber};

#[derive(Debug, Clone, Copy, PartialEq)]
/// A width and height.
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

  /// The largest upright rectangle that fits inside a rectangle of this size after it is rotated by `p_degrees`
  /// around its center. The rectangle shares the rotated rectangle's center.
  ///
  /// This is the area to keep after rotating an image so no empty corners show, such as when straightening a
  /// photo or previewing a rotated crop.
  /// # Arguments
  /// - `p_degrees`: The rotation, in degrees. The direction does not matter.
  /// - `p_aspect`: The width-to-height ratio the rectangle must have, such as `16.0 / 9.0`, or the ratio of this
  ///   size to keep its shape. `None` finds the rectangle with the largest area, whatever its shape. A ratio that
  ///   is not a positive number counts as `None`.
  pub fn inscribed_after_rotation(&self, p_degrees: impl IntoNumber, p_aspect: impl Into<Option<f32>>) -> Size {
    let (width, height) = (self.width as f64, self.height as f64);
    let (sin, cos) = p_degrees.into::<f64>().to_radians().sin_cos();
    let (sin, cos) = (sin.abs(), cos.abs());

    if let Some(aspect) = p_aspect.into().map(f64::from).filter(|aspect| aspect.is_finite() && *aspect > 0.0) {
      // A centered rectangle `aspect * b` wide and `b` tall fits inside the rotated one when both
      // aspect*b*|cos| + b*|sin| <= width and aspect*b*|sin| + b*|cos| <= height. Take the tallest `b` that does.
      let inner_height = (width / (aspect * cos + sin)).min(height / (aspect * sin + cos));
      return Size::from((aspect * inner_height, inner_height));
    }

    let width_is_longer = width >= height;
    let (long_side, short_side) = if width_is_longer { (width, height) } else { (height, width) };
    if short_side <= 2.0 * sin * cos * long_side || (sin - cos).abs() < 1e-9 {
      // Half constrained: two corners of the rectangle touch the longer side of the rotated rectangle, and the
      // other two sit on the line down its middle.
      let half = 0.5 * short_side;
      let (inner_width, inner_height) =
        if width_is_longer { (half / sin, half / cos) } else { (half / cos, half / sin) };
      Size::from((inner_width, inner_height))
    } else {
      // Fully constrained: the rectangle touches all four sides of the rotated rectangle.
      let cos_2a = cos * cos - sin * sin;
      Size::from(((width * cos - height * sin) / cos_2a, (height * cos - width * sin) / cos_2a))
    }
  }

  /// The size of the smallest upright rectangle that holds a rectangle of this size after it is rotated by
  /// `p_degrees` around its center. Sides are rounded down to whole pixels and are at least 1.
  ///
  /// This is the canvas a rotation needs to show the whole image, the counterpart of
  /// [`Size::inscribed_after_rotation`].
  /// - `p_degrees`: The rotation, in degrees. The direction does not matter.
  pub fn rotated_bounds(&self, p_degrees: impl IntoNumber) -> Size {
    let (mut width, mut height) = (self.width, self.height);
    let mut degrees = p_degrees.into::<f32>().rem_euclid(180.0);
    if degrees >= 90.0 {
      std::mem::swap(&mut width, &mut height);
      degrees -= 90.0;
    }
    if degrees == 0.0 {
      return Size::new(width.max(1.0), height.max(1.0));
    }
    let (sin, cos) = degrees.to_radians().sin_cos();
    let rotated_width = (width * cos + height * sin).abs().floor();
    let rotated_height = (width * sin + height * cos).abs().floor();
    Size::new(rotated_width.max(1.0), rotated_height.max(1.0))
  }

  /// The width-to-height ratio, or `None` when the height is zero.
  pub fn aspect_ratio(&self) -> Option<f32> {
    (self.height != 0.0).then(|| self.width / self.height)
  }
}

impl<A: IntoNumber, B: IntoNumber> From<(A, B)> for Size {
  fn from(p_size_tuple: (A, B)) -> Self {
    Size::new(p_size_tuple.0, p_size_tuple.1)
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

  /// Divides the scalar by each side: `2.0 / Size::new(4, 8)` is `(0.5, 0.25)`.
  fn div(self, p_rhs: Size) -> Self::Output {
    Size {
      width: self / p_rhs.width,
      height: self / p_rhs.height,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const ROTATIONS: [(f32, f32, f32); 7] = [
    (100.0, 50.0, 10.0),
    (100.0, 50.0, -10.0),
    (50.0, 100.0, 25.0),
    (300.0, 100.0, 30.0),
    (200.0, 200.0, 45.0),
    (100.0, 50.0, 90.0),
    (100.0, 50.0, 135.0),
  ];

  /// Whether a `p_inner` rectangle centered on a `p_outer` rectangle rotated by `p_degrees` fits inside it.
  fn fits_inside_rotation(p_outer: Size, p_inner: Size, p_degrees: f32) -> bool {
    let (sin, cos) = p_degrees.to_radians().sin_cos();
    let (half_width, half_height) = (p_inner.width / 2.0, p_inner.height / 2.0);
    // Turn each corner of the inner rectangle back into the outer rectangle's frame.
    [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)].iter().all(|(sx, sy)| {
      let (x, y) = (sx * half_width, sy * half_height);
      let (back_x, back_y) = (x * cos + y * sin, -x * sin + y * cos);
      back_x.abs() <= p_outer.width / 2.0 + 1e-3 && back_y.abs() <= p_outer.height / 2.0 + 1e-3
    })
  }

  #[test]
  fn no_rotation_keeps_the_whole_size() {
    let size = Size::new(100, 50);
    assert_eq!(size.inscribed_after_rotation(0, None), size);
    assert_eq!(size.inscribed_after_rotation(0, size.aspect_ratio()), size);
  }

  #[test]
  fn a_quarter_turn_swaps_the_sides() {
    let inscribed = Size::new(100, 50).inscribed_after_rotation(90, None);
    assert!((inscribed.width - 50.0).abs() < 1e-3 && (inscribed.height - 100.0).abs() < 1e-3, "{inscribed:?}");
  }

  #[test]
  fn inscribed_rectangles_fit_inside_the_rotation() {
    for (width, height, degrees) in ROTATIONS {
      let size = Size::new(width, height);
      for aspect in [None, size.aspect_ratio(), Some(16.0 / 9.0), Some(0.5)] {
        let inscribed = size.inscribed_after_rotation(degrees, aspect);
        assert!(fits_inside_rotation(size, inscribed, degrees), "{size:?} {degrees} {aspect:?}: {inscribed:?}");
      }
    }
  }

  #[test]
  fn the_aspect_locked_rectangle_keeps_the_aspect_and_is_never_bigger() {
    for (width, height, degrees) in ROTATIONS {
      let size = Size::new(width, height);
      let largest = size.inscribed_after_rotation(degrees, None);
      let locked = size.inscribed_after_rotation(degrees, size.aspect_ratio());
      assert!((locked.width / locked.height - width / height).abs() < 1e-4, "{size:?} {degrees}: {locked:?}");
      assert!(locked.width <= width && locked.height <= height, "{size:?} {degrees}: {locked:?}");
      assert!(
        largest.width * largest.height >= locked.width * locked.height - 1e-2,
        "{size:?} {degrees}: {largest:?} vs {locked:?}"
      );
    }
  }

  #[test]
  fn a_given_aspect_is_kept_and_touches_the_rotated_edges() {
    let size = Size::new(400, 300);
    for degrees in [0.0f32, 8.0, -20.0, 60.0] {
      let inscribed = size.inscribed_after_rotation(degrees, 16.0 / 9.0);
      assert!((inscribed.width / inscribed.height - 16.0 / 9.0).abs() < 1e-4, "{degrees}: {inscribed:?}");
      // As large as it can be: a hair bigger no longer fits.
      assert!(!fits_inside_rotation(size, inscribed * 1.01, degrees), "{degrees}: {inscribed:?} is not the largest");
    }
  }

  #[test]
  fn rotated_bounds_hold_the_whole_rotation() {
    assert_eq!(Size::new(100, 50).rotated_bounds(0), Size::new(100, 50));
    assert_eq!(Size::new(100, 50).rotated_bounds(90), Size::new(50, 100));
    assert_eq!(Size::new(100, 100).rotated_bounds(45).width, 141.0);
    assert_eq!(2.0 / Size::new(4, 8), Size::new(0.5, 0.25));
  }

  #[test]
  fn an_invalid_aspect_finds_the_largest_area() {
    let size = Size::new(100, 50);
    for aspect in [0.0, -1.0, f32::NAN, f32::INFINITY] {
      assert_eq!(size.inscribed_after_rotation(10, aspect), size.inscribed_after_rotation(10, None), "{aspect}");
    }
  }
}
