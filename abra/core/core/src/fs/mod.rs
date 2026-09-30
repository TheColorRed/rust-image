//! File system utilities.

pub(crate) mod path;
mod read_write;
mod writer_options;
/// The supported image reader formats.
pub(crate) mod readers {
  /// Support for reading GIF images.
  pub mod gif;
  /// Support for reading JPEG images.
  pub mod jpeg;
  /// Support for reading PNG images.
  pub mod png;
  /// Support for reading SVG images.
  pub mod svg;
  /// Support for reading WebP images.
  pub mod webp;
}
/// The supported image writer formats.
pub(crate) mod writers {
  /// Support for writing GIF images.
  pub mod gif;
  /// Support for writing JPEG images.
  pub mod jpeg;
  /// Support for writing PNG images.
  pub mod png;
  /// Support for writing WebP images.
  pub mod webp;
}
pub use read_write::{ImageFormat, reader, writer};
pub use writer_options::WriterOptions;
