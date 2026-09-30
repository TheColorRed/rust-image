use crate::fs::readers::gif::read_gif;
use crate::fs::readers::jpeg::read_jpg;
use crate::fs::readers::png::read_png;
use crate::fs::readers::svg::read_svg;
use crate::fs::readers::webp::read_webp;
use crate::fs::writer_options::WriterOptions;
use crate::fs::writers::{gif::write_gif, jpeg::write_jpg, png::write_png, webp::write_webp};
use primitives::Image;
use std::path::{Path, PathBuf};

/// A supported raster or vector image file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
  /// Portable Network Graphics format. Lossless compression and widely supported.
  Png,
  /// Joint Photographic Experts Group format. Lossy compression.
  Jpeg,
  /// Graphics Interchange Format. Supports animation and transparency.
  Gif,
  /// WebP format. Supports both lossy and lossless compression.
  Webp,
  /// Scalable Vector Graphics format. Vector-based image format. Read only.
  Svg,
}

impl ImageFormat {
  /// Maps a file extension (case-insensitive, no leading dot) to a format, if supported.
  pub fn from_extension(p_extension: &str) -> Option<ImageFormat> {
    match p_extension.to_ascii_lowercase().as_str() {
      "png" => Some(Self::Png),
      "jpg" | "jpeg" => Some(Self::Jpeg),
      "gif" => Some(Self::Gif),
      "webp" => Some(Self::Webp),
      "svg" => Some(Self::Svg),
      _ => None,
    }
  }

  /// The format of a file, from its extension.
  pub fn from_path(p_path: impl AsRef<Path>) -> Option<ImageFormat> {
    let extension = p_path.as_ref().extension().and_then(|extension| extension.to_str())?;
    Self::from_extension(extension)
  }

  /// Whether files in this format can be read.
  pub fn can_read(&self) -> bool {
    true
  }

  /// Whether images can be written in this format. SVG is read only.
  pub fn can_write(&self) -> bool {
    !matches!(self, ImageFormat::Svg)
  }
}

/// A reader for loading image files. Create one with [`reader`].
pub struct Reader {
  path: PathBuf,
}

/// A writer for saving image files. Create one with [`writer`].
pub struct Writer {
  path: PathBuf,
  options: Option<WriterOptions>,
}

impl Writer {
  /// Sets the writer options for the image writer.
  /// # Arguments
  /// * `p_options` - The writer options to be applied to the image writer.
  pub fn with_options(mut self, p_options: impl Into<Option<WriterOptions>>) -> Self {
    self.options = p_options.into();
    self
  }

  /// Saves the image, choosing the format from the file extension. Missing parent directories are created.
  /// # Returns
  /// * `Result<(), String>` - Ok if the image was saved successfully, Err with an error message otherwise.
  pub fn save(self, p_image: &Image) -> Result<(), String> {
    let format =
      ImageFormat::from_path(&self.path).filter(ImageFormat::can_write).ok_or("This file type is not supported")?;
    if let Some(parent) = self.path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
      std::fs::create_dir_all(parent).map_err(|e| format!("Error creating directory {}: {e}", parent.display()))?;
    }
    let path = self.path.to_string_lossy().into_owned();
    match format {
      ImageFormat::Jpeg => write_jpg(path, p_image, &self.options),
      ImageFormat::Webp => write_webp(path, p_image),
      ImageFormat::Png => write_png(path, p_image, &self.options),
      ImageFormat::Gif => write_gif(path, p_image, &self.options),
      ImageFormat::Svg => unreachable!("SVG is not writable"),
    }
  }
}

impl Reader {
  /// Loads the image, choosing the decoder from the file extension.
  /// # Returns
  /// * `Result<Image, String>` - Ok with the image if it was loaded successfully, Err with an error message otherwise.
  pub fn load(self) -> Result<Image, String> {
    let path = self.path.to_string_lossy().into_owned();
    match ImageFormat::from_path(&self.path).ok_or("This file type is not supported")? {
      ImageFormat::Jpeg => read_jpg(path),
      ImageFormat::Webp => read_webp(path),
      ImageFormat::Png => read_png(path),
      ImageFormat::Gif => read_gif(path),
      ImageFormat::Svg => read_svg(path),
    }
  }
}

/// Creates a reader for the image file at `p_path`.
pub fn reader(p_path: impl AsRef<Path>) -> Reader {
  Reader {
    path: p_path.as_ref().to_path_buf(),
  }
}

/// Creates a writer for the image file at `p_path`.
pub fn writer(p_path: impl AsRef<Path>) -> Writer {
  Writer {
    path: p_path.as_ref().to_path_buf(),
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn formats_come_from_extensions() {
    assert_eq!(ImageFormat::from_path("a/b.JPG"), Some(ImageFormat::Jpeg));
    assert_eq!(ImageFormat::from_path("a/b"), None);
    assert!(!ImageFormat::Svg.can_write());
    assert!(ImageFormat::Png.can_write());
  }

  #[test]
  fn unsupported_files_are_errors() {
    assert!(reader("image.bmp").load().is_err());
    assert!(writer("image.svg").save(&Image::new(1, 1)).is_err());
  }
}
