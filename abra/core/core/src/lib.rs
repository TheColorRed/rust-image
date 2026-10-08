// The scaffolding other languages bind to; the types that carry `cfg_attr(feature = "uniffi", ...)` export through it.
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub mod color;
mod combine;
mod fs;
pub mod geometry;
pub mod image;
mod image_loader;
mod loader;
pub mod performance;
pub mod settings;
pub mod transform;
pub mod units;

pub use color::*;
pub use combine::*;
pub use fs::WriterOptions;
pub use fs::path::IntoGlobPatterns;
pub use fs::path::{get_paths_from_folders, get_paths_from_glob};
pub use fs::{ImageFormat, encode_png, reader, writer};
pub use geometry::*;
pub use image::image_ext::{ImageExt, ImageRef};
pub use image::{GrayPlane, rgba_to_gray};
pub use image_loader::*;
pub use loader::*;
pub use performance::{Performance, PerformanceOptions};
pub use primitives::Color;
pub use primitives::Image;
pub use primitives::Resolution;
pub use primitives::{Channel, Channels};
pub use settings::Settings;
pub use transform::*;
pub use units::*;

/// Converts primitive numeric inputs to a requested numeric type.
pub trait IntoNumber {
  fn into<T: FromF32>(self) -> T;
}

macro_rules! impl_into_number_trait {
  ($($number:ty),+ $(,)?) => {
    $(
      impl IntoNumber for $number {
        fn into<T: FromF32>(self) -> T {
          T::from_f32(self as f32)
        }
      }
    )+
  };
}

impl_into_number_trait!(f32, f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

/// Implements conversion from primitive numeric values into a local target type.
///
/// The target must implement `From<f32>`; integer and `f64` values are converted
/// through `f32`. Invoke this macro in the crate that owns the target type.
#[macro_export]
macro_rules! impl_into_number {
  ($target:ty) => {
    impl From<f64> for $target {
      fn from(p_value: f64) -> Self {
        Self::from(p_value as f32)
      }
    }

    impl $crate::FromF32 for $target {
      fn from_f32(p_value: f32) -> Self {
        Self::from(p_value)
      }
    }

    $crate::impl_into_number!(@integers $target; i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
  };
  (@integers $target:ty; $($number:ty),+ $(,)?) => {
    $(
      impl From<$number> for $target {
        fn from(p_value: $number) -> Self {
          Self::from(p_value as f32)
        }
      }
    )+
  };
}

/// Converts an `f32` value into a target type.
pub trait FromF32 {
  fn from_f32(p_v: f32) -> Self;
}

impl FromF32 for f32 {
  fn from_f32(p_v: f32) -> Self {
    p_v
  }
}

impl FromF32 for f64 {
  fn from_f32(p_v: f32) -> Self {
    p_v as f64
  }
}

macro_rules! impl_from_f32_rounding {
  ($($number:ty),+ $(,)?) => {
    $(
      impl FromF32 for $number {
        fn from_f32(p_v: f32) -> Self {
          p_v.round() as Self
        }
      }
    )+
  };
}

impl_from_f32_rounding!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

#[cfg(test)]
mod tests {
  use super::IntoNumber;

  #[test]
  fn numeric_conversion_preserves_pixel_dimensions_above_255() {
    let width: u32 = IntoNumber::into(800_u32);
    let height: u32 = IntoNumber::into(1201_u32);

    assert_eq!((width, height), (800, 1201));
  }
}

/// Picks an item based on if/else if/else. This should support unlimited "else if" statements.
/// Example: `pick!(p_radius >= 96 => 8, p_radius >= 48 => 4, else => 2);`
/// Expands to:
/// ```ignore
/// if p_radius >= 96 {
///   8
/// } else if p_radius >= 48 {
///   4
/// } else {
///   2
/// }
/// ```
#[macro_export]
macro_rules! if_pick {
  ($cond:expr => $val:expr, $( $rest:tt )* ) => {
    if $cond {
      $val
    } else {
      $crate::if_pick!( $( $rest )* )
    }
  };
  (else => $val:expr) => {
    $val
  };
}
