//! File system utilities.

/// The file info of an image.
pub mod file_info;
mod writer_options;
/// The supported image reader formats.
pub mod readers {
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
pub mod writers {
  /// Support for writing GIF images.
  pub mod gif;
  /// Support for writing JPEG images.
  pub mod jpeg;
  /// Support for writing PNG images.
  pub mod png;
  /// Support for writing WebP images.
  pub mod webp;
}

use std::{fs, path::Path};
pub use writer_options::WriterOptions;

/// Creates a directory and all its parent directories if they do not exist.
pub fn mkdirp(path: &str) -> Result<(), String> {
  let path = Path::new(path);
  if path.exists() {
    return Ok(());
  }

  fs::create_dir_all(path).map_err(|e| e.to_string())?;
  Ok(())
}
