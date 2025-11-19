mod color;
// mod debug;
mod combine;
mod fs;
mod geometry;
mod image;
mod loader;
mod transform;

pub use color::*;
pub use transform::*;
// pub use debug::*;
pub use combine::*;
pub use fs::WriterOptions;
pub use geometry::*;
pub use image::*;
pub use loader::*;

#[derive(Clone, Copy, Eq, PartialEq)]
/// The number of channels in an image
pub enum Channels {
  /// A three channel image (RGB)
  RGB = 3,
  /// A four channel image (RGBA)
  RGBA = 4,
}
