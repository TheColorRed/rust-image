use crate::fs::WriterOptions;
use crate::fs::file_info::FileInfo;
use crate::fs::readers::svg::read_svg;
use crate::fs::readers::{gif::read_gif, jpeg::read_jpg, png::read_png, webp::read_webp};
use crate::fs::writers::{gif::write_gif, jpeg::write_jpg, png::write_png, webp::write_webp};
use primitives::Image as PrimitiveImage;

/// Trait providing core-level convenience methods for `Image` (IO methods that used to be inherent).
/// Implemented for `primitives::Image` so callers can use `image.open()` and `image.save()`.
pub trait CoreImageFsExt {
  /// Opens an image from the specified file path, replacing the current image data.
  /// - `file`: The file path to load the image from.
  fn open(&mut self, file: impl Into<String>);
  /// Saves the image to the specified file path.
  /// - `file`: The file path to save the image to.
  /// - `options`: Optional writer options.
  fn save(&self, file: impl Into<String>, options: impl Into<Option<WriterOptions>>);
  /// Creates a new Image by loading it from the specified file path.
  /// - `file`: The file path to load the image from.
  fn new_from_path(file: impl Into<String>) -> Self
  where
    Self: Sized;
}

impl CoreImageFsExt for PrimitiveImage {
  fn new_from_path(file: impl Into<String>) -> Self {
    let mut img = PrimitiveImage::new(0u32, 0u32);
    img.open(file);
    img
  }

  fn open(&mut self, file: impl Into<String>) {
    let file = file.into();
    let info: FileInfo;
    if file.ends_with(".jpg") || file.ends_with(".jpeg") {
      info = read_jpg(&file).unwrap();
    } else if file.ends_with(".webp") {
      info = read_webp(&file).unwrap();
    } else if file.ends_with(".png") {
      info = read_png(&file).unwrap();
    } else if file.ends_with(".gif") {
      info = read_gif(&file).unwrap();
    } else if file.ends_with(".svg") {
      info = read_svg(&file).unwrap();
    } else {
      panic!("Attempting to open unsupported file format");
    }

    self.set_new_pixels(&info.pixels, info.width, info.height);
  }

  fn save(&self, file: impl Into<String>, options: impl Into<Option<WriterOptions>>) {
    let options = options.into();
    let file = file.into();
    if file.ends_with(".jpg") || file.ends_with(".jpeg") {
      write_jpg(&file, &self, &options).unwrap();
    } else if file.ends_with(".webp") {
      write_webp(&file, &self).unwrap();
    } else if file.ends_with(".png") {
      write_png(&file, &self, &options).unwrap();
    } else if file.ends_with(".gif") {
      write_gif(&file, &self, &options).unwrap();
    } else {
      panic!("Attempting to save unsupported file format");
    }
  }
}
