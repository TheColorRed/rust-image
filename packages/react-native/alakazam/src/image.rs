use std::sync::{Arc, Mutex};

use abra::prelude::*;
use abra::transform::prelude::{ResizeTarget, resize};
use abra_body_segmentation::Mask;
use abra_find_person::{PersonInstance, person_skin_mask};
use base64::{Engine, engine::general_purpose::STANDARD};

use vessel::prelude::{
  BehaviorSubject, Frame, Observable, Observer, Offscreen, Picture, RenderedFrame, Subject, Trackable,
};

use crate::{AbraError, effect_spec::EffectSpec, live_image::LiveImage, models};

/// RGBA pixels (unpremultiplied, `width * height * 4` bytes) for display.
#[derive(uniffi::Record)]
pub struct PreviewImage {
  pub width: u32,
  pub height: u32,
  pub data: Vec<u8>,
}

/// The bounds and confidence of a detected person, in source-image pixels.
#[derive(uniffi::Record)]
pub struct PersonDetection {
  pub id: u32,
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
  pub confidence: f64,
}

/// An RGBA image owned by Rust. JavaScript holds a handle; pixels only cross
/// the bridge when `rgba()` is called.
#[derive(uniffi::Object)]
pub struct AbraImage {
  inner: Arc<Mutex<Image>>,
  /// Skin segmentation computed when this image is loaded, shared with copies made by the editor.
  skin_mask: Arc<Mutex<Option<Mask>>>,
  /// Person masks and the selected person's skin mask are shared across editor copies.
  people: Arc<Mutex<Option<Vec<PersonInstance>>>>,
  selected_person: Arc<Mutex<Option<u32>>>,
  selected_skin_mask: Arc<BehaviorSubject<Option<Arc<Mask>>>>,
  /// How many edits the image has had, so whatever tracks it draws it again after each.
  edits: BehaviorSubject<u64>,
  /// The effects shown over the image while they are tried out (a slider being dragged, a mood being previewed). They are
  /// not applied to the pixels.
  live: BehaviorSubject<Vec<EffectSpec>>,
  /// An image a tool written in TypeScript made, shown in place of `live` while it is set.
  tool_image: BehaviorSubject<Option<Arc<Image>>>,
  /// The image at the last size a tool asked for, so a drag resizes the photo once and not on every tick.
  tool_source: Mutex<Option<((u32, u32, u64), Arc<Image>)>>,
}

impl AbraImage {
  pub(crate) fn from_image(p_image: Image) -> Arc<Self> {
    Self::from_image_with_skin_mask(p_image, None)
  }

  fn from_image_with_skin_mask(p_image: Image, p_skin_mask: Option<Mask>) -> Arc<Self> {
    Self::from_image_with_shared_state(
      p_image,
      Arc::new(Mutex::new(p_skin_mask)),
      Arc::new(Mutex::new(None)),
      Arc::new(Mutex::new(None)),
      Arc::new(BehaviorSubject::new(None)),
    )
  }

  fn from_image_with_shared_state(
    p_image: Image, p_skin_mask: Arc<Mutex<Option<Mask>>>, p_people: Arc<Mutex<Option<Vec<PersonInstance>>>>,
    p_selected_person: Arc<Mutex<Option<u32>>>, p_selected_skin_mask: Arc<BehaviorSubject<Option<Arc<Mask>>>>,
  ) -> Arc<Self> {
    Arc::new(Self {
      inner: Arc::new(Mutex::new(p_image)),
      skin_mask: p_skin_mask,
      people: p_people,
      selected_person: p_selected_person,
      selected_skin_mask: p_selected_skin_mask,
      edits: BehaviorSubject::new(0),
      live: BehaviorSubject::new(Vec::new()),
      tool_image: BehaviorSubject::new(None),
      tool_source: Mutex::new(None),
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
    if self.tool_image.peek().is_some() {
      self.tool_image.next(None);
    }
  }

  /// Shows `p_image`, which a tool made, in place of the photo and any live effects. Nothing is applied to the pixels.
  pub(crate) fn show_tool_image(&self, p_image: Arc<Image>) {
    self.tool_image.next(Some(p_image));
  }

  /// The image shrunk to fit within `p_max_width` x `p_max_height`, or as it is when it already fits. A tool runs on this
  /// while a slider is dragged. The same size of the same edit gives the same image, so a drag resizes once.
  pub(crate) fn preview_image(&self, p_max_width: u32, p_max_height: u32) -> Arc<Image> {
    let key = (p_max_width, p_max_height, self.edits.peek());
    let mut cache = self.tool_source.lock().unwrap();
    if let Some((cached, image)) = cache.as_ref()
      && *cached == key
    {
      return Arc::clone(image);
    }
    let image = self.inner.lock().unwrap();
    let (width, height) = image.dimensions::<u32>();
    let preview = if width <= p_max_width && height <= p_max_height {
      image.clone()
    } else {
      resize(ResizeTarget::Fit(Size::new(p_max_width, p_max_height))).resized(&image)
    };
    let preview = Arc::new(preview);
    *cache = Some((key, Arc::clone(&preview)));
    preview
  }

  /// Clones the Rust-owned image without transferring its pixels through JavaScript.
  pub(crate) fn clone_image(&self) -> Image {
    self.inner.lock().unwrap().clone()
  }

  /// Completed edits as CPU pixels for composed thumbnail backgrounds; excludes transient live effects.
  pub(crate) fn thumbnail_pictures(&self, p_size: &BehaviorSubject<(u32, u32)>) -> Subject<Picture> {
    let pixels = Arc::clone(&self.inner);
    p_size.combine_latest(&self.edits).filter_map(move |(size, _)| {
      if size.0 == 0 || size.1 == 0 {
        return None;
      }
      let preview = preview_of(&pixels.lock().unwrap(), size.0, size.1);
      Some(Picture::Pixels(Arc::new(Frame {
        width: preview.width,
        height: preview.height,
        pixels: preview.data.into(),
      })))
    })
  }

  pub(crate) fn skin_mask(&self) -> Option<Mask> {
    self.selected_skin_mask.peek().map(|mask| (*mask).clone()).or_else(|| self.skin_mask.lock().unwrap().clone())
  }

  fn detected_people(&self) -> Result<Vec<PersonInstance>, AbraError> {
    let mut people = self.people.lock().unwrap();
    if people.is_none() {
      let image = self.inner.lock().unwrap().clone();
      *people = Some(models::person_segmenter()?.process(&image).map_err(|error| AbraError::Ai {
        message: error.to_string(),
      })?);
    }
    Ok(people.as_ref().unwrap().clone())
  }

  fn choose_person(&self, p_id: Option<u32>) -> Result<(), AbraError> {
    let mask = self.skin_mask_for_person(p_id)?;
    *self.selected_person.lock().unwrap() = p_id;
    self.selected_skin_mask.next(mask.map(Arc::new));
    Ok(())
  }

  pub(crate) fn skin_mask_for_person(&self, p_id: Option<u32>) -> Result<Option<Mask>, AbraError> {
    let Some(id) = p_id else {
      return Ok(self.skin_mask.lock().unwrap().clone());
    };
    let people = self.detected_people()?;
    let person = people.get(id as usize).ok_or(AbraError::InvalidPersonSelection { index: id })?;
    let cached_skin = self.skin_mask.lock().unwrap().clone();
    let skin_mask = if let Some(mask) = cached_skin {
      mask
    } else {
      let image = self.inner.lock().unwrap().clone();
      let ai_mask = models::skin_segmenter().and_then(|segmenter| {
        segmenter.process(&image).map_err(|error| AbraError::Ai {
          message: error.to_string(),
        })
      });
      match ai_mask {
        Ok(mask) => {
          *self.skin_mask.lock().unwrap() = Some(mask.clone());
          mask
        }
        // Without the skin model, color-based detection still finds the skin; it is limited to the body below.
        // Not cached: the cache holds the AI mask, which can arrive later.
        Err(error) => {
          eprintln!("[image] skin segmentation unavailable, using color detection inside the body: {error}");
          abra::humanoid::prelude::skin_mask(&image, 0.0)
        }
      }
    };
    let selected_skin = person_skin_mask(&person.mask, &skin_mask).map_err(|error| AbraError::Ai {
      message: error.to_string(),
    })?;
    Ok(Some(selected_skin))
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
    let skin_mask = match models::skin_segmenter().and_then(|segmenter| {
      segmenter.process(&image).map_err(|error| AbraError::Ai {
        message: error.to_string(),
      })
    }) {
      Ok(mask) => Some(mask),
      Err(error) => {
        eprintln!("[image] skin segmentation unavailable: {error}");
        None
      }
    };
    Ok(Self::from_image_with_skin_mask(image, skin_mask))
  }

  /// Loads an image like `read`, but takes its skin mask from a file saved by `write_skin_mask` instead of running the
  /// skin model. The mask must be the same size as the image.
  #[uniffi::constructor]
  pub fn read_with_skin_mask(path: String, skin_mask_path: String) -> Result<Arc<Self>, AbraError> {
    let image = Image::read(&path).map_err(|message| AbraError::Io { message })?;
    let mask = Mask::from_image(Image::read(&skin_mask_path).map_err(|message| AbraError::Io { message })?);
    let (image_size, mask_size) = (image.dimensions::<u32>(), mask.dimensions::<u32>());
    if image_size != mask_size {
      return Err(AbraError::InvalidPixels {
        message: format!(
          "the saved skin mask is {}x{} but the image is {}x{}",
          mask_size.0, mask_size.1, image_size.0, image_size.1
        ),
      });
    }
    Ok(Self::from_image_with_skin_mask(image, Some(mask)))
  }

  /// Writes the skin mask computed for this image to an image file, as gray pixels (use a `.webp` or `.png` name: both are
  /// lossless, so the values load back unchanged; WebP is the smaller). Returns `false`, writing nothing, when the image has no skin mask: the skin model is not on this device.
  pub fn write_skin_mask(&self, path: String) -> Result<bool, AbraError> {
    let Some(mask) = self.skin_mask.lock().unwrap().clone() else {
      return Ok(false);
    };
    mask.to_image().write(&path, None).map_err(|message| AbraError::Io { message })?;
    Ok(true)
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

  /// Writes the image like `write`, at `quality` (0 to 100) for the formats that have one. JPEG uses it, and so does
  /// WebP, which is lossless without it: a lower quality gives a much smaller file. PNG and GIF ignore it.
  pub fn write_with_quality(&self, path: String, quality: u8) -> Result<(), AbraError> {
    let image = self.inner.lock().unwrap();
    image.write(&path, WriterOptions { quality }).map_err(|message| AbraError::Io { message })
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
    Self::from_image_with_shared_state(
      self.inner.lock().unwrap().clone(),
      Arc::clone(&self.skin_mask),
      Arc::clone(&self.people),
      Arc::clone(&self.selected_person),
      Arc::clone(&self.selected_skin_mask),
    )
  }

  /// The image as the library's own `Image`, at full size and without crossing the bridge, for a tool written against it.
  /// Changing the result does not change this image; give a tool's result back with `replace_with`.
  pub fn to_image(&self) -> Arc<Image> {
    Arc::new(self.inner.lock().unwrap().clone())
  }

  /// Replaces the pixels with `image`, as an edit does. An image a tool left unmade is made now, so the edited photo holds
  /// pixels and not the recipe for them.
  pub fn replace_with(&self, image: Arc<Image>) {
    let (width, height) = image.dimensions::<u32>();
    let pixels = image.to_rgba_vec();
    self.with_image_mut(|current| *current = Image::new_from_pixels(width, height, pixels, Channels::RGBA));
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

  /// Detects people in the image and returns their source-image bounding boxes.
  pub fn find_people(&self) -> Result<Vec<PersonDetection>, AbraError> {
    self.detected_people().map(|people| person_detections(&people))
  }

  /// Selects a detected person for skin effects; passing `None` restores whole-image skin effects.
  pub fn select_person(&self, id: Option<u32>) -> Result<(), AbraError> {
    self.choose_person(id)
  }

  /// Selects the most confident detected person whose box contains the source-image point.
  pub fn select_person_at(&self, x: f64, y: f64) -> Result<Option<u32>, AbraError> {
    if !x.is_finite() || !y.is_finite() {
      return Err(AbraError::InvalidPixels {
        message: "person selection coordinates must be finite".to_owned(),
      });
    }
    let (width, height) = self.inner.lock().unwrap().dimensions::<u32>();
    if x < 0.0 || y < 0.0 || x >= width as f64 || y >= height as f64 {
      return Ok(None);
    }
    let people = self.detected_people()?;
    // Bodies can overlap (a baby held by a parent). Prefer one whose outline is under the tap, then the smallest, so the
    // inner body can be picked.
    let mask_index = y as usize * width as usize + x as usize;
    let selected = people
      .iter()
      .enumerate()
      .filter(|(_, person)| {
        x >= person.bounds.x as f64
          && x < (person.bounds.x + person.bounds.width) as f64
          && y >= person.bounds.y as f64
          && y < (person.bounds.y + person.bounds.height) as f64
      })
      .min_by_key(|(_, person)| {
        let under_outline = person.mask.values().get(mask_index).is_some_and(|value| *value > 127);
        (!under_outline, person.bounds.width as u64 * person.bounds.height as u64)
      })
      .map(|(id, _)| id as u32);
    if let Some(id) = selected {
      self.choose_person(Some(id))?;
    }
    Ok(selected)
  }

  /// The selected person index, or `None` when skin effects apply to everyone.
  pub fn selected_person(&self) -> Option<u32> {
    *self.selected_person.lock().unwrap()
  }
}

#[uniffi::export(async_runtime = "tokio")]
impl AbraImage {
  /// Decodes a photo and computes its skin mask on the bounded image worker pool.
  #[uniffi::constructor]
  pub async fn read_async(path: String) -> Result<Arc<Self>, AbraError> {
    crate::image_workers::run(move || Self::read(path)).await
  }

  /// `read_with_skin_mask` on the bounded image worker pool.
  #[uniffi::constructor]
  pub async fn read_with_skin_mask_async(path: String, skin_mask_path: String) -> Result<Arc<Self>, AbraError> {
    crate::image_workers::run(move || Self::read_with_skin_mask(path, skin_mask_path)).await
  }

  /// Downscales once in native memory, without transferring pixels through JavaScript.
  pub async fn thumbnail_source_async(&self, width: u32, height: u32) -> Result<Arc<Self>, AbraError> {
    if width == 0 || height == 0 {
      return Err(AbraError::InvalidPixels {
        message: "Thumbnail dimensions must be positive".to_owned(),
      });
    }
    let pixels = Arc::clone(&self.inner);
    crate::image_workers::run(move || {
      let pixels = pixels.lock().map_err(|_| AbraError::Render {
        message: "Image lock poisoned".to_owned(),
      })?;
      let preview = preview_of(&pixels, width, height);
      Self::from_rgba(preview.width, preview.height, preview.data)
    })
    .await
  }

  /// Copies a small native source and applies its control's edits away from the JavaScript thread.
  pub async fn render_thumbnail_async(
    &self, effects: Vec<EffectSpec>, operation: Option<crate::image_operation::ImageOperation>,
  ) -> Result<Arc<Self>, AbraError> {
    if operation.is_some() && !effects.is_empty() {
      return Err(AbraError::Render {
        message: "A thumbnail must specify effects or an operation, not both".to_owned(),
      });
    }
    let pixels = Arc::clone(&self.inner);
    crate::image_workers::run(move || {
      let image = Self::from_image(
        pixels
          .lock()
          .map_err(|_| AbraError::Render {
            message: "Thumbnail source lock poisoned".to_owned(),
          })?
          .clone(),
      );
      if let Some(operation) = operation {
        image.apply_operation(operation);
      } else {
        for effect in effects {
          image.apply_effect(effect);
        }
      }
      Ok(image)
    })
    .await
  }

  /// Computes the AI skin mask for this image if it has none yet, so skin effects use it instead of color detection.
  /// Call after downloading the skin model.
  pub async fn detect_skin_async(&self) -> Result<(), AbraError> {
    let image = self.clone_image();
    let skin_mask = Arc::clone(&self.skin_mask);
    crate::image_workers::run(move || {
      let mut cached = skin_mask.lock().unwrap();
      if cached.is_none() {
        *cached = Some(models::skin_segmenter()?.process(&image).map_err(|error| AbraError::Ai {
          message: error.to_string(),
        })?);
      }
      Ok(())
    })
    .await
  }

  /// Detects people away from the calling thread, so opening a skin control does not block the editor.
  pub async fn find_people_async(&self) -> Result<Vec<PersonDetection>, AbraError> {
    let image = self.clone_image();
    let people_cache = Arc::clone(&self.people);
    let (sender, receiver) = futures::channel::oneshot::channel();
    std::thread::Builder::new()
      .name("abra-find-people".to_owned())
      .spawn(move || {
        let result = (|| {
          let mut cached = people_cache.lock().map_err(|_| AbraError::Ai {
            message: "Person detection cache lock poisoned".to_owned(),
          })?;
          if cached.is_none() {
            *cached = Some(models::person_segmenter()?.process(&image).map_err(|error| AbraError::Ai {
              message: error.to_string(),
            })?);
          }
          Ok(person_detections(cached.as_ref().unwrap()))
        })();
        let _ = sender.send(result);
      })
      .map_err(|error| AbraError::Ai {
        message: format!("Could not start person detection: {error}"),
      })?;
    receiver.await.map_err(|_| AbraError::Ai {
      message: "Person detection worker stopped before returning a result".to_owned(),
    })?
  }
}

fn person_detections(p_people: &[PersonInstance]) -> Vec<PersonDetection> {
  p_people
    .iter()
    .enumerate()
    .map(|(id, person)| PersonDetection {
      id: id as u32,
      x: person.bounds.x,
      y: person.bounds.y,
      width: person.bounds.width,
      height: person.bounds.height,
      confidence: person.confidence as f64,
    })
    .collect()
}

/// `p_image` as a picture a Vessel component can show, such as the track of a slider.
pub(crate) fn frame_of(p_image: &Image) -> Frame {
  let (width, height) = p_image.dimensions::<u32>();
  Frame {
    width,
    height,
    pixels: p_image.rgba().to_vec().into(),
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
  let preview = resize(ResizeTarget::Fit(Size::new(p_max_width, p_max_height))).resized(p_image);
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
    let skin_mask = self.skin_mask.lock().unwrap().clone();
    let selected_skin_mask = Arc::clone(&self.selected_skin_mask);
    // The image, scaled to the size it is drawn at and uploaded once per size and edit.
    let mut rendering: Option<(((u32, u32), u64), LiveImage)> = None;
    // The picture a tool starts from, uploaded once while the tool's image keeps starting from the same pixels.
    let mut tool_rendering: Option<(Image, LiveImage)> = None;
    let changes =
      p_size.combine_latest(&self.edits).combine_latest(&self.live).combine_latest(selected_skin_mask.as_ref());
    let changes = changes.combine_latest(&self.tool_image);
    Offscreen::new(&changes, move |((((size, edits), live), selected_skin), tool)| {
      // Until the view has a surface it has no size, and there is nothing to draw for.
      if size.0 == 0 || size.1 == 0 {
        return None;
      }
      // A tool's image that is still a recipe is drawn by the GPU straight to the view's texture, without making its pixels.
      // One that has its pixels, or is not drawn that way, goes to the view as the pixels it is.
      if let Some(image) = tool {
        if let Some((root, passes)) = abra::abra_core::image::recipe::gpu_run(image) {
          let reuse = tool_rendering.as_ref().is_some_and(|(held, _)| held.shares_pixels_with(&root));
          if !reuse {
            let (width, height) = root.dimensions::<u32>();
            tool_rendering = LiveImage::new(width, height, root.to_rgba_vec()).ok().map(|live| (root, live));
          }
          if let Some((_, live)) = tool_rendering.as_mut()
            && let Some(picture) = live.picture_of_passes(passes)
          {
            return Some(match picture {
              Picture::Pixels(frame) => RenderedFrame::Pixels(frame),
              Picture::Gpu(frame) => RenderedFrame::Gpu(frame),
            });
          }
        }
        return Some(RenderedFrame::Pixels(Arc::new(frame_of(image))));
      }
      let key = (*size, *edits);
      if rendering.as_ref().is_none_or(|(rendered_for, _)| *rendered_for != key) {
        let proxy = preview_of(&pixels.lock().unwrap(), size.0, size.1);
        match LiveImage::new(proxy.width, proxy.height, proxy.data) {
          Ok(image) => rendering = Some((key, image)),
          Err(error) => {
            eprintln!("[image] offscreen preview failed: {error}");
            return None;
          }
        }
      }
      let picture =
        rendering.as_mut()?.1.picture_with_skin_mask(live.iter(), selected_skin.as_deref().or(skin_mask.as_ref()))?;
      Some(match picture {
        Picture::Pixels(frame) => RenderedFrame::Pixels(frame),
        Picture::Gpu(frame) => RenderedFrame::Gpu(frame),
      })
    })
    .map(|frame| match frame {
      RenderedFrame::Pixels(frame) => Picture::Pixels(Arc::clone(frame)),
      RenderedFrame::Gpu(frame) => Picture::Gpu(Arc::clone(frame)),
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_find_person::PersonBounds;
  use std::sync::atomic::{AtomicUsize, Ordering};

  #[test]
  fn a_lower_quality_makes_a_smaller_webp_and_none_keeps_it_lossless() {
    let (width, height) = (64, 64);
    // Noisy pixels, which are hard to compress without losing detail (a smooth gradient compresses well either way).
    let mut seed = 12345u32;
    let mut noise = move || {
      seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
      (seed >> 24) as u8
    };
    let pixels: Vec<u8> = (0..width * height).flat_map(|_| [noise(), noise(), noise(), 255]).collect();
    let image = AbraImage::from_image(Image::new_from_pixels(width, height, pixels.clone(), Channels::RGBA));
    let directory = std::env::temp_dir().join(format!("alakazam-webp-quality-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = |name: &str| directory.join(name).to_string_lossy().into_owned();

    image.write(path("lossless.webp")).unwrap();
    image.write_with_quality(path("low.webp"), 30).unwrap();
    image.write_with_quality(path("high.webp"), 90).unwrap();
    let size = |name: &str| std::fs::metadata(directory.join(name)).unwrap().len();
    assert!(size("low.webp") < size("high.webp"), "a lower quality gives a smaller file");
    assert!(size("low.webp") < size("lossless.webp"), "a low quality is smaller than lossless");

    // Lossless keeps every pixel; lossy still decodes to an image of the same size.
    let lossless = Image::read(&path("lossless.webp")).unwrap();
    assert_eq!(lossless.rgba(), pixels.as_slice());
    assert_eq!(Image::read(&path("low.webp")).unwrap().dimensions::<u32>(), (width, height));
    std::fs::remove_dir_all(&directory).unwrap();
  }

  #[test]
  fn a_lossless_webp_mask_is_exact_and_smaller_than_png() {
    // A soft-edged blob, like a skin mask.
    let (width, height) = (160u32, 160u32);
    let values: Vec<u8> = (0..width * height)
      .map(|i| {
        let (x, y) = ((i % width) as f32 - 80.0, (i / width) as f32 - 80.0);
        (255.0 - ((x * x + y * y).sqrt() - 40.0).clamp(0.0, 20.0) * 12.75) as u8
      })
      .collect();
    let mask = Mask::from_values(width, height, values.clone());
    let directory = std::env::temp_dir().join(format!("alakazam-mask-format-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = |name: &str| directory.join(name).to_string_lossy().into_owned();
    mask.to_image().write(path("mask.png"), None).unwrap();
    mask.to_image().write(path("mask.webp"), None).unwrap();
    let size = |name: &str| std::fs::metadata(directory.join(name)).unwrap().len();
    eprintln!("mask png: {} bytes, lossless webp: {} bytes", size("mask.png"), size("mask.webp"));

    let loaded = Mask::from_image(Image::read(&path("mask.webp")).unwrap());
    assert_eq!(loaded.values(), values.as_slice(), "lossless WebP keeps every value");
    assert!(size("mask.webp") < size("mask.png"));
    std::fs::remove_dir_all(&directory).unwrap();
  }

  #[test]
  fn a_webp_that_cannot_be_encoded_is_an_error_and_leaves_the_image_usable() {
    // WebP sides are limited to 16383 pixels. Encoding must fail with an error, not panic with the image locked.
    let image = AbraImage::from_image(Image::new_from_color(16384, 1, Color::white()));
    let path = std::env::temp_dir().join(format!("alakazam-too-wide-{}.webp", std::process::id()));
    let result = image.write_with_quality(path.to_string_lossy().into_owned(), 85);
    assert!(result.is_err(), "an image too wide for WebP is an error");
    assert_eq!(image.width(), 16384, "the image is still usable afterwards");
    let _ = std::fs::remove_file(path);
  }

  #[test]
  fn a_saved_skin_mask_loads_back_unchanged() {
    let (width, height) = (12, 8);
    let values: Vec<u8> = (0..width * height).map(|i| (i * 7 % 256) as u8).collect();
    let directory = std::env::temp_dir().join(format!("alakazam-skin-mask-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let image_path = directory.join("image.png").to_string_lossy().into_owned();
    let mask_path = directory.join("skin-mask.webp").to_string_lossy().into_owned();
    Image::new_from_color(width, height, Color::white()).write(&image_path, None).unwrap();

    let source = AbraImage::from_image_with_skin_mask(
      Image::new_from_color(width, height, Color::white()),
      Some(Mask::from_values(width, height, values.clone())),
    );
    assert!(source.write_skin_mask(mask_path.clone()).unwrap());

    let loaded = AbraImage::read_with_skin_mask(image_path.clone(), mask_path.clone()).unwrap();
    assert_eq!(loaded.skin_mask().unwrap().values(), values.as_slice());

    // An image with no skin mask has nothing to write.
    let without = AbraImage::from_image(Image::new_from_color(width, height, Color::white()));
    assert!(!without.write_skin_mask(directory.join("none.png").to_string_lossy().into_owned()).unwrap());

    // A mask for a differently sized image is refused, not applied out of place.
    Image::new_from_color(width + 1, height, Color::white()).write(&image_path, None).unwrap();
    assert!(AbraImage::read_with_skin_mask(image_path, mask_path).is_err());
    std::fs::remove_dir_all(&directory).unwrap();
  }

  #[test]
  fn tapping_inside_an_overlapping_body_picks_the_inner_one() {
    let (width, height) = (20, 20);
    let body = |x: u32, y: u32, w: u32, h: u32, confidence: f32| {
      let pixels: Vec<u8> = (0..height)
        .flat_map(|row| {
          (0..width).flat_map(move |col| {
            let value = if (x..x + w).contains(&col) && (y..y + h).contains(&row) { 255 } else { 0 };
            [value, value, value, 255]
          })
        })
        .collect();
      PersonInstance {
        bounds: PersonBounds {
          x,
          y,
          width: w,
          height: h,
        },
        confidence,
        mask: Mask::from_image(Image::new_from_pixels(width, height, pixels, Channels::RGBA)),
      }
    };
    let image = AbraImage::from_image_with_skin_mask(Image::new_from_color(width, height, Color::white()), None);
    // The outer body is the more confident one and its box holds the inner body, as with a parent holding a baby.
    *image.people.lock().unwrap() = Some(vec![body(0, 0, 20, 20, 0.95), body(8, 8, 6, 6, 0.6)]);

    assert_eq!(image.select_person_at(10.0, 10.0).unwrap(), Some(1), "the tap is on the inner body");
    assert_eq!(image.select_person_at(2.0, 2.0).unwrap(), Some(0), "the tap is only on the outer body");
  }

  #[test]
  fn targeted_replay_preserves_live_selection_and_other_people() {
    let width = 16;
    let height = 16;
    let source = Image::new_from_color(width, height, Color::from_rgba(60, 100, 200, 255));
    let skin = Mask::from_image(Image::new_from_color(width, height, Color::white()));
    let image = AbraImage::from_image_with_skin_mask(source.clone(), Some(skin));
    let people = (0..2)
      .map(|id| {
        let pixels: Vec<u8> = (0..height)
          .flat_map(|_| {
            (0..width).flat_map(move |x| {
              let value = if x / 8 == id { 255 } else { 0 };
              [value, value, value, 255]
            })
          })
          .collect();
        PersonInstance {
          bounds: PersonBounds {
            x: id * 8,
            y: 0,
            width: 8,
            height,
          },
          confidence: 1.0,
          mask: Mask::from_image(Image::new_from_pixels(width, height, pixels, Channels::RGBA)),
        }
      })
      .collect();
    *image.people.lock().unwrap() = Some(people);
    image.select_person(Some(1)).unwrap();
    let selected_mask = image.selected_skin_mask.peek().unwrap().values().to_vec();
    let updates = Arc::new(AtomicUsize::new(0));
    let listener_updates = Arc::clone(&updates);
    let _subscription = image.selected_skin_mask.subscribe(move |_| {
      listener_updates.fetch_add(1, Ordering::SeqCst);
    });
    updates.store(0, Ordering::SeqCst);

    image.apply_effect_to_person(EffectSpec::SkinTan { offset: 0.7 }, Some(0)).unwrap();

    assert_eq!(image.selected_person(), Some(1));
    assert_eq!(image.selected_skin_mask.peek().unwrap().values(), selected_mask);
    let first = image.clone_image();
    for y in 0..height {
      for x in 0..width {
        let i = ((y * width + x) * 4) as usize;
        if x < 8 {
          assert_ne!(&first.rgba()[i..i + 3], &source.rgba()[i..i + 3]);
        } else if x > 8 {
          assert_eq!(&first.rgba()[i..i + 4], &source.rgba()[i..i + 4]);
        }
      }
    }
    image.apply_effect_to_person(EffectSpec::SkinTan { offset: 0.3 }, Some(1)).unwrap();
    assert_eq!(updates.load(Ordering::SeqCst), 0, "replay must not redraw the live selection");
    let second = image.clone_image();
    for y in 0..height {
      let start = (y * width * 4) as usize;
      assert_eq!(&second.rgba()[start..start + 28], &first.rgba()[start..start + 28]);
    }

    let size = BehaviorSubject::new((0, 0));
    let pictures = image.pictures(&size);
    let (sender, receiver) = std::sync::mpsc::channel();
    let _preview = pictures.subscribe(move |picture| sender.send(picture.clone()).unwrap());
    size.next((width, height));
    let frame = match receiver.recv_timeout(std::time::Duration::from_secs(5)).unwrap() {
      Picture::Pixels(frame) => (*frame).clone(),
      Picture::Gpu(frame) => frame.read_back().unwrap(),
    };
    assert_eq!((frame.width, frame.height), (width, height));
    assert_eq!(
      frame.pixels.as_slice(),
      second.rgba(),
      "the offscreen preview must contain both completed person edits"
    );
    assert_eq!(updates.load(Ordering::SeqCst), 0);
  }
}
