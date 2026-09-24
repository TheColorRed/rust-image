//! File system utilities.

/// The file info of an image.
pub(crate) mod file_info;
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
use std::{fs, path::Path};
pub use writer_options::WriterOptions;

/// Creates a directory and all its parent directories if they do not exist.
pub fn mkdirp(p_path: impl Into<String>) -> Result<(), String> {
  let p_path = p_path.into();
  let p_path = Path::new(p_path.as_str());
  if p_path.exists() {
    return Ok(());
  }

  fs::create_dir_all(p_path).map_err(|e| e.to_string())?;
  Ok(())
}
