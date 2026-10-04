use std::sync::{Arc, Mutex};

use abra::prelude::*;
use abra::transform::prelude::{ResizeTarget, TransformAlgorithm, resize};
use base64::{Engine, engine::general_purpose::STANDARD};

use vessel::prelude::{BehaviorSubject, Frame, Observable, Observer, Picture, Subject, Trackable};

use crate::{AbraError, effect_spec::EffectSpec, live_image::LiveImage};

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
  inner: Arc<Mutex<Image>>,
  /// How many edits the image has had, so whatever tracks it draws it again after each.
  edits: BehaviorSubject<u64>,
  /// The effects shown over the image while they are tried out (a slider being dragged, a mood being previewed). They are
  /// not applied to the pixels.
  live: BehaviorSubject<Vec<EffectSpec>>,
}

impl AbraImage {
  pub(crate) fn from_image(p_image: Image) -> Arc<Self> {
    Arc::new(Self {
      inner: Arc::new(Mutex::new(p_image)),
      edits: BehaviorSubject::new(0),
      live: BehaviorSubject::new(Vec::new()),
    })
  }

  /// Runs `p_edit` against the underlying image.
  pub(crate) fn with_image_mut(&self, p_edit: impl FnOnce(&mut Image)) {
    p_edit(&mut self.inner.lock().unwrap());
    self.edits.next(self.edits.peek() + 1);
  }

  /// Shows `p_effects` over the image, in order, in place of any shown before. Nothing is applied to the pixels.
  pub(crate) fn show_live(&self, p_effects: Vec<EffectSpec>) {
    self.live.next(p_effects);
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

  /// Encodes the image as a `data:image/png;base64,...` URI, for showing small images (such as thumbnails) in a
  /// plain React Native `Image`.
  pub fn png_data_uri(&self) -> Result<String, AbraError> {
    let image = self.inner.lock().unwrap();
    let bytes = abra::abra_core::encode_png(&image, None).map_err(|message| AbraError::Io { message })?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
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
    preview_of(&self.inner.lock().unwrap(), max_width, max_height)
  }
}

/// `p_image` as a picture a Vessel component can show, such as the track of a slider.
pub(crate) fn frame_of(p_image: &Image) -> Frame {
  let (width, height) = p_image.dimensions::<u32>();
  Frame {
    width,
    height,
    pixels: p_image.rgba().to_vec(),
  }
}

/// `p_image` downscaled to fit within `p_max_width` x `p_max_height`.
fn preview_of(p_image: &Image, p_max_width: u32, p_max_height: u32) -> PreviewImage {
  let (width, height) = p_image.dimensions::<u32>();
  if width <= p_max_width && height <= p_max_height {
    return PreviewImage {
      width,
      height,
      data: p_image.rgba().to_vec(),
    };
  }
  // Bilinear: the automatic choice for large downscales is Lanczos, which costs far more for a preview.
  let preview = resize(ResizeTarget::Fit(Size::new(p_max_width, p_max_height)))
    .with_algorithm(TransformAlgorithm::Bilinear)
    .resized(p_image);
  let (width, height) = preview.dimensions::<u32>();
  PreviewImage {
    width,
    height,
    data: preview.rgba().to_vec(),
  }
}

/// What a view that shows the image follows: the image at the size the view has, with the live effects over it, drawn
/// again after every edit.
impl Trackable for AbraImage {
  fn pictures(&self, p_size: &BehaviorSubject<(u32, u32)>) -> Subject<Picture> {
    let pixels = Arc::clone(&self.inner);
    // The image, scaled to the size it is drawn at and uploaded once per size and edit.
    let rendering: Mutex<Option<(((u32, u32), u64), LiveImage)>> = Mutex::new(None);
    p_size.combine_latest(&self.edits).combine_latest(&self.live).filter_map(move |((size, edits), live)| {
      // Until the view has a surface it has no size, and there is nothing to draw for.
      if size.0 == 0 || size.1 == 0 {
        return None;
      }
      let mut rendering = rendering.lock().unwrap();
      let key = (*size, *edits);
      if rendering.as_ref().is_none_or(|(rendered_for, _)| *rendered_for != key) {
        let proxy = preview_of(&pixels.lock().unwrap(), size.0, size.1);
        *rendering = Some((key, LiveImage::new(proxy.width, proxy.height, proxy.data).ok()?));
      }
      rendering.as_mut()?.1.picture(live.iter())
    })
  }
}
