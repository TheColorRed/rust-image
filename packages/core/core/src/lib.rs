mod color;
// mod debug;
mod combine;
mod fs;
mod geometry;
pub mod image;
mod loader;
mod transform;

pub use color::*;
pub use transform::*;
// pub use debug::*;
pub use combine::*;
pub use fs::WriterOptions;
pub use geometry::*;
pub use image::image::*;
pub use loader::*;

#[derive(Clone, Copy, Eq, PartialEq)]
/// The number of channels in an image
pub enum Channels {
  /// A three channel image (RGB)
  RGB = 3,
  /// A four channel image (RGBA)
  RGBA = 4,
}

// lib.rs or geometry/mod.rs (a public crate-local trait)
pub trait FromF32 {
  fn from_f32(v: f32) -> Self;
}

impl FromF32 for f32 {
  fn from_f32(v: f32) -> Self {
    v
  }
}

impl FromF32 for i32 {
  fn from_f32(v: f32) -> Self {
    v.round() as _
  } // or floor(), or trunc()
}

impl FromF32 for u8 {
  fn from_f32(v: f32) -> Self {
    v.round().clamp(0.0, 255.0) as _
  }
}
impl FromF32 for u32 {
  fn from_f32(v: f32) -> Self {
    v.round().clamp(0.0, 255.0) as _
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
macro_rules! pick {
  ($cond:expr => $val:expr, $( $rest:tt )* ) => {
    if $cond {
      $val
    } else {
      $crate::pick!( $( $rest )* )
    }
  };
  (else => $val:expr) => {
    $val
  };
}
