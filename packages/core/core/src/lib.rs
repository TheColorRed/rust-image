pub mod color;
// mod debug;
mod combine;
mod fs;
pub mod geometry;
pub mod image;
mod loader;
pub mod settings;
pub mod transform;

pub use color::*;
pub use settings::Settings;
pub use transform::*;
// pub use debug::*;
pub use combine::*;
pub use fs::WriterOptions;
// Re-export selected I/O helpers so other crates (e.g., abra wrapper) can access them
pub use fs::file_info::FileInfo;
// Explicitly export reader and writer functions to avoid ambiguous glob re-exports.
pub use fs::readers::gif::read_gif;
pub use fs::readers::jpeg::read_jpg;
pub use fs::readers::png::read_png;
pub use fs::readers::svg::read_svg;
pub use fs::readers::webp::read_webp;
pub use fs::writers::gif::write_gif;
pub use fs::writers::jpeg::write_jpg;
pub use fs::writers::png::write_png;
pub use fs::writers::webp::write_webp;
pub use geometry::*;
// `image` module content moved to `primitives` crate and re-exported below.
pub use loader::*;
// Re-export primitives Image for workspace users. This replaces the core-defined Image type
// so consumers can continue to use `use abra_core::Image;` with the new primitives implementation.
pub use primitives::Channels;
pub use primitives::Color;
pub use primitives::Image;

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
