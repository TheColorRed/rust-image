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
    Self::from_image_with_shared_state(
      self.inner.lock().unwrap().clone(),
      Arc::clone(&self.skin_mask),
      Arc::clone(&self.people),
      Arc::clone(&self.selected_person),
      Arc::clone(&self.selected_skin_mask),
    )
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
    let changes =
      p_size.combine_latest(&self.edits).combine_latest(&self.live).combine_latest(selected_skin_mask.as_ref());
    Offscreen::new(&changes, move |(((size, edits), live), selected_skin)| {
      // Until the view has a surface it has no size, and there is nothing to draw for.
      if size.0 == 0 || size.1 == 0 {
        return None;
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
