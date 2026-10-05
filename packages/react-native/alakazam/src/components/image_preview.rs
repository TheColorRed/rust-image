//! The photo being edited: a component that React creates and the Vessel engine draws.
//!
//! `LiveView` keeps a Vessel `Image` and gives it something to follow: the pictures made from the messages React sends.
//! The chain of effects is the state those messages add up to, and a picture is that chain rendered over the photo. On a
//! GPU the picture is a texture that goes to the screen without being copied to the CPU. Everything else (when to draw, the
//! native view, the GPU target) is the engine's.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use vessel::prelude::*;

use crate::{AbraImage, live_effects};

/// The photo being edited: an image that follows the photo and the controls React touches. A `VesselView` shows it on a
/// native view, and the Vessel engine does the rest.
#[derive(uniffi::Object, Component)]
pub struct ImagePreview {
  image: Image,
  presented: Arc<AtomicBool>,
}

/// Everything React can tell a view.
#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum Message {
  /// The value of the slider and the key it affects (brightness, contrast, etc.).
  SliderMove(String, f64),
  /// An action triggered by the user, identified by a string key (mood-warm, mood-cool, etc.).
  Action(String),
}

#[uniffi::export]
impl ImagePreview {
  /// A view of `image`.
  #[uniffi::constructor]
  pub fn new(p_image: Arc<AbraImage>) -> Arc<Self> {
    let image = Image::new().track(&p_image);
    let presented = Arc::new(AtomicBool::new(false));
    image.subject::<Drawn>().subscribe({
      let presented = Arc::clone(&presented);
      move |_| presented.store(true, Ordering::SeqCst)
    });
    let messages = image.subject::<Message>();

    messages.filter(|message| matches!(message, Message::SliderMove(..))).subscribe({
      let p_image = Arc::clone(&p_image);
      move |message| {
        if let Message::SliderMove(key, value) = message
          && let Some(effects) = live_effects::slider(key, *value)
        {
          p_image.show_live(effects);
        }
      }
    });

    messages.filter(|message| matches!(message, Message::Action(_))).subscribe({
      let p_image = Arc::clone(&p_image);
      move |message| {
        if let Message::Action(key) = message
          && let Some(effects) = live_effects::action(key)
        {
          p_image.show_live(effects);
        }
      }
    });

    Arc::new(Self { image, presented })
  }

  /// Tells the view what changed.
  pub fn send(&self, message: Message) {
    self.image.subject::<Message>().next(message);
  }

  /// Whether a completed photo frame has reached the mounted surface. Placeholder frames do not count.
  pub fn has_frame(&self) -> bool {
    self.presented.load(Ordering::SeqCst)
  }
}

#[cfg(test)]
mod tests {
  use std::{
    any::Any,
    sync::Mutex,
    time::{Duration, Instant},
  };

  use abra::abra_core::{Channels, Image as AbraPixels};
  use vessel::surface;

  use super::*;

  /// A surface that records the red value of the first pixel of each frame drawn on it, and fails if any pixel is
  /// transparent, since the native view shows the alpha byte.
  struct Recorder(Mutex<Vec<u8>>);

  impl surface::Surface for Recorder {
    fn is_alive(&self) -> bool {
      true
    }

    fn size(&self) -> (u32, u32) {
      (4, 4)
    }

    fn resize(&self, _width: u32, _height: u32) {}

    fn draw_rgba(&self, _width: u32, _height: u32, p_rgba: &[u8]) -> bool {
      // Before the photo is ready the view draws nothing at all, which is fully transparent and not a frame to check.
      if p_rgba.chunks_exact(4).all(|pixel| pixel[3] == 0) {
        return true;
      }
      assert!(p_rgba.chunks_exact(4).all(|pixel| pixel[3] == 255), "the frame has transparent pixels");
      self.0.lock().unwrap().push(p_rgba[0]);
      true
    }

    fn retire(&self) {}

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
      self
    }
  }

  /// The native view `p_id` gets its surface. Each test uses its own view, since the registry is shared by the process.
  fn surface_of(p_id: i32) -> Arc<Recorder> {
    let recorder = Arc::new(Recorder(Mutex::new(Vec::new())));
    surface::register_surface(p_id, recorder.clone());
    recorder
  }

  /// The newest red value drawn, once `p_ready` accepts one.
  fn newest(p_surface: &Recorder, p_ready: impl Fn(u8) -> bool) -> u8 {
    let start = Instant::now();
    loop {
      if let Some(pixel) = p_surface.0.lock().unwrap().last().copied().filter(|&pixel| p_ready(pixel)) {
        return pixel;
      }
      assert!(start.elapsed() < Duration::from_secs(5), "no matching frame arrived");
      std::thread::sleep(Duration::from_millis(5));
    }
  }

  fn view(p_id: i32) -> Arc<ImagePreview> {
    let image =
      AbraImage::from_image(AbraPixels::new_from_pixels(4, 4, [100u8, 100, 100, 255].repeat(16), Channels::RGBA));
    let view = ImagePreview::new(image);
    view.mount(p_id);
    view
  }

  #[test]
  fn the_photo_is_shown_as_it_is_and_controls_change_it_live() {
    // The view appears after the component is mounted on it, which is how a React Native view mounts.
    let view = view(7001);
    assert!(!view.has_frame());
    let surface = surface_of(7001);
    assert_eq!(newest(&surface, |_| true), 100, "the photo as it is");
    let start = Instant::now();
    while !view.has_frame() {
      assert!(start.elapsed() < Duration::from_secs(5), "presentation was not acknowledged");
      std::thread::sleep(Duration::from_millis(5));
    }

    view.send(Message::SliderMove("action-brightness".to_string(), 50.0));
    assert_eq!(newest(&surface, |pixel| pixel == 150), 150);

    view.send(Message::SliderMove("action-brightness".to_string(), 0.0));
    assert_eq!(newest(&surface, |pixel| pixel == 100), 100);

    view.send(Message::Action("action-invert".to_string()));
    assert_eq!(newest(&surface, |pixel| pixel == 155), 155);
    view.send(Message::Action(String::new()));
    assert_eq!(newest(&surface, |pixel| pixel == 100), 100);
  }

  #[test]
  fn an_edit_to_the_image_shows_up_on_the_view() {
    let image =
      AbraImage::from_image(AbraPixels::new_from_pixels(4, 4, [100u8, 100, 100, 255].repeat(16), Channels::RGBA));
    let view = ImagePreview::new(Arc::clone(&image));
    view.mount(7002);
    let surface = surface_of(7002);
    assert_eq!(newest(&surface, |_| true), 100);
    image.invert();
    assert_eq!(newest(&surface, |pixel| pixel == 155), 155);
    drop(view);
  }

  #[test]
  fn dropping_the_view_takes_it_off_the_native_view() {
    let surface = surface_of(7003);
    drop(view(7003));
    let drawn = surface.0.lock().unwrap().len();
    let view = view(7003);
    view.send(Message::SliderMove("action-brightness".to_string(), 50.0));
    assert_eq!(newest(&surface, |pixel| pixel == 150), 150);
    assert!(surface.0.lock().unwrap().len() > drawn);
  }
}
