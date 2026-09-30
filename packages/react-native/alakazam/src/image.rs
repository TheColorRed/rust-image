use std::sync::{Arc, Mutex};

use abra::prelude::*;
use abra::transform::prelude::{ResizeTarget, TransformAlgorithm, resize};

use crate::AbraError;

/// RGBA pixels (unpremultiplied, `width * height * 4` bytes) for display.
#[derive(uniffi::Record)]
pub struct PreviewImage {
  pub width: u32,
  pub height: u32,
  pub data: Vec<u8>,
}

/// An RGBA image owned by Rust. JavaScript holds a handle; pixels only cross
/// the bridge when `rgba()` is called.
#[derive(uniffi::Object)]
pub struct AbraImage {
  inner: Mutex<Image>,
}

impl AbraImage {
  pub(crate) fn from_image(p_image: Image) -> Arc<Self> {
    Arc::new(Self {
      inner: Mutex::new(p_image),
    })
  }

  /// Runs `p_edit` against the underlying image.
  pub(crate) fn with_image_mut(&self, p_edit: impl FnOnce(&mut Image)) {
    let mut image = self.inner.lock().unwrap();
    p_edit(&mut image);
  }

  /// Clones the Rust-owned image without transferring its pixels through JavaScript.
  pub(crate) fn clone_image(&self) -> Image {
    self.inner.lock().unwrap().clone()
  }
}

#[uniffi::export]
impl AbraImage {
  /// Creates a transparent image.
  #[uniffi::constructor]
  pub fn new(width: u32, height: u32) -> Arc<Self> {
    Self::from_image(Image::new(width, height))
  }

  /// Loads an image from a file path (not a `file://` URI).
  #[uniffi::constructor]
  pub fn read(path: String) -> Result<Arc<Self>, AbraError> {
    let image = Image::read(&path).map_err(|message| AbraError::Io { message })?;
    Ok(Self::from_image(image))
  }

  /// Creates an image from raw RGBA bytes (`width * height * 4` long).
  #[uniffi::constructor]
  pub fn from_rgba(width: u32, height: u32, data: Vec<u8>) -> Result<Arc<Self>, AbraError> {
    let expected = width as usize * height as usize * 4;
    if data.len() != expected {
      return Err(AbraError::InvalidPixels {
        message: format!("expected {expected} bytes for {width}x{height} RGBA, got {}", data.len()),
      });
    }
    Ok(Self::from_image(Image::new_from_pixels(width, height, data, Channels::RGBA)))
  }

  /// Writes the image to a file path; the format follows the extension.
  pub fn write(&self, path: String) -> Result<(), AbraError> {
    let image = self.inner.lock().unwrap();
    image.write(&path, None).map_err(|message| AbraError::Io { message })
  }

  /// Returns an independent copy of this image.
  pub fn copy(&self) -> Arc<Self> {
    Self::from_image(self.inner.lock().unwrap().clone())
  }

  pub fn width(&self) -> u32 {
    self.inner.lock().unwrap().dimensions::<u32>().0
  }

  pub fn height(&self) -> u32 {
    self.inner.lock().unwrap().dimensions::<u32>().1
  }

  /// Returns the pixels as RGBA bytes.
  pub fn rgba(&self) -> Vec<u8> {
    self.inner.lock().unwrap().rgba().to_vec()
  }

  /// Returns a copy downscaled to fit within `max_width` x `max_height` for display, so full-resolution
  /// pixels never cross into JavaScript. Images that already fit are returned at their own size.
  pub fn preview(&self, max_width: u32, max_height: u32) -> PreviewImage {
    let image = self.inner.lock().unwrap();
    let (width, height) = image.dimensions::<u32>();
    if width <= max_width && height <= max_height {
      return PreviewImage {
        width,
        height,
        data: image.rgba().to_vec(),
      };
    }
    // Bilinear: the automatic choice for large downscales is Lanczos, which costs far more for a preview.
    let preview = resize(ResizeTarget::Fit(Size::new(max_width, max_height)))
      .with_algorithm(TransformAlgorithm::Bilinear)
      .resized(&image);
    let (width, height) = preview.dimensions::<u32>();
    PreviewImage {
      width,
      height,
      data: preview.rgba().to_vec(),
    }
  }
}
