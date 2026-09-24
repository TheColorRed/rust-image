use crate::fs::WriterOptions;
use crate::{reader, writer};
use primitives::Image as PrimitiveImage;
use std::path::Path;

/// Fallible image I/O methods for [`PrimitiveImage`].
pub trait CoreImageFsExt {
  /// Reads an image, choosing the decoder from its file extension.
  fn read(p_file: impl AsRef<Path>) -> Result<Self, String>
  where
    Self: Sized;
  /// Writes an image, choosing the encoder from its file extension.
  fn write(&self, p_file: impl AsRef<Path>, p_options: impl Into<Option<WriterOptions>>) -> Result<(), String>;
  /// Returns a clone of the image.
  fn as_image(&self) -> PrimitiveImage;
}

impl CoreImageFsExt for PrimitiveImage {
  fn read(p_file: impl AsRef<Path>) -> Result<Self, String> {
    let info = reader(p_file).load()?;
    let mut image = PrimitiveImage::new(0u32, 0u32);
    image.set_new_pixels(&info.pixels, info.width, info.height);
    Ok(image)
  }

  fn write(&self, p_file: impl AsRef<Path>, p_options: impl Into<Option<WriterOptions>>) -> Result<(), String> {
    writer(p_file).with_options(p_options).save(self.clone())
  }

  fn as_image(&self) -> PrimitiveImage {
    self.clone()
  }
}
