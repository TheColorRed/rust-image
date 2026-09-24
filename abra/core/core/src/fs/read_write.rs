use crate::fs::file_info::FileInfo;
use crate::fs::readers::gif::read_gif;
use crate::fs::readers::jpeg::read_jpg;
use crate::fs::readers::png::read_png;
use crate::fs::readers::svg::read_svg;
use crate::fs::readers::webp::read_webp;
use crate::fs::writer_options::WriterOptions;
use crate::fs::writers::{gif::write_gif, jpeg::write_jpg, png::write_png, webp::write_webp};
use primitives::Image;
use std::path::Path;

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
  /// Scalable Vector Graphics format. Vector-based image format.
  Svg,
  /// A file format that is not recognized or supported.
  Unknown,
}

impl ImageFormat {
  /// Maps a file extension (case-insensitive, no leading dot) to a format, if supported.
  pub(crate) fn from_extension(p_extension: &str) -> Option<ImageFormat> {
    match p_extension.to_ascii_lowercase().as_str() {
      "png" => Some(Self::Png),
      "jpg" | "jpeg" => Some(Self::Jpeg),
      "gif" => Some(Self::Gif),
      "webp" => Some(Self::Webp),
      "svg" => Some(Self::Svg),
      _ => None,
    }
  }

  fn from_path(p_path: impl AsRef<Path>) -> ImageFormat {
    let path = p_path.as_ref();
    let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or_default();
    Self::from_extension(extension).unwrap_or(Self::Unknown)
  }
}

/// A reader for loading image files.
/// Provides functionality to read image files based on their format.
pub struct Reader {
  path: String,
}
/// A writer for saving image files.
/// Provides functionality to write image files based on their format.
pub struct Writer {
  path: String,
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
  /// Saves the image to the specified path using the appropriate format based on the file extension.
  /// # Arguments
  /// * `p_image` - The image to be saved.
  /// # Returns
  /// * `Result<(), String>` - Ok if the image was saved successfully, Err with an error message otherwise.
  pub fn save(self, p_image: Image) -> Result<(), String> {
    let path = &self.path;
    let format = ImageFormat::from_path(path);
    match format {
      ImageFormat::Jpeg => write_jpg(path, &p_image, &self.options),
      ImageFormat::Webp => write_webp(path, &p_image),
      ImageFormat::Png => write_png(path, &p_image, &self.options),
      ImageFormat::Gif => write_gif(path, &p_image, &self.options),
      _ => Err("This file type is not supported".to_string()),
    }
  }
  /// Checks if the image format of the specified path is supported.
  /// # Returns
  /// * `bool` - True if the image format is supported, false otherwise.
  pub fn is_supported(self) -> bool {
    let path = &self.path;
    match ImageFormat::from_path(path) {
      ImageFormat::Jpeg | ImageFormat::Webp | ImageFormat::Png | ImageFormat::Gif | ImageFormat::Svg => true,
      _ => false,
    }
  }
}

impl Reader {
  /// Loads the image from the specified path using the appropriate format based on the file extension.
  /// # Returns
  /// * `Result<FileInfo, String>` - Ok with the file information if the image was loaded successfully, Err with an error message otherwise.
  pub fn load(self) -> Result<FileInfo, String> {
    let path = &self.path;
    match ImageFormat::from_path(path) {
      ImageFormat::Jpeg => read_jpg(path),
      ImageFormat::Webp => read_webp(path),
      ImageFormat::Png => read_png(path),
      ImageFormat::Gif => read_gif(path),
      ImageFormat::Svg => read_svg(path),
      _ => Err("This file type is not supported".to_string()),
    }
  }
  /// Checks if the image format of the specified path is supported.
  /// # Returns
  /// * `bool` - True if the image format is supported, false otherwise.
  pub fn is_supported(self) -> bool {
    let path = &self.path;
    match ImageFormat::from_path(path) {
      ImageFormat::Jpeg | ImageFormat::Webp | ImageFormat::Png | ImageFormat::Gif | ImageFormat::Svg => true,
      _ => false,
    }
  }
}

pub fn reader(p_path: impl AsRef<Path>) -> Reader {
  Reader {
    path: p_path.as_ref().to_string_lossy().into_owned(),
  }
}

pub fn writer(p_path: impl AsRef<Path>) -> Writer {
  Writer {
    path: p_path.as_ref().to_string_lossy().into_owned(),
    options: None,
  }
}
