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

use crate::{effect_spec::EffectSpec, live_image::LiveImage};

struct PreviewSource {
  image: Arc<abra_uniffi::Image>,
  live: BehaviorSubject<Vec<EffectSpec>>,
  /// The mask the skin effects work on, or none to find the skin by its color.
  skin: BehaviorSubject<Option<Arc<abra_uniffi::Mask>>>,
  tool_image: BehaviorSubject<Option<Arc<abra::prelude::Image>>>,
}

impl PreviewSource {
  fn new(image: Arc<abra_uniffi::Image>) -> Self {
    Self { image, live: BehaviorSubject::new(Vec::new()), skin: BehaviorSubject::new(None), tool_image: BehaviorSubject::new(None) }
  }
}

impl Trackable for PreviewSource {
  fn pictures(&self, size: &BehaviorSubject<(u32, u32)>) -> Subject<Picture> {
    let source = Arc::clone(&self.image);
    let changes = size.combine_latest(&self.live).combine_latest(&self.skin);
    let changes = changes.combine_latest(&self.tool_image);
    let mut rendering: Option<((u32, u32), LiveImage)> = None;
    Offscreen::new(&changes, move |(((size, live), skin), tool)| {
      if size.0 == 0 || size.1 == 0 { return None }
      if let Some(image) = tool {
        return Some(RenderedFrame::Pixels(Arc::new(frame_of(image))));
      }
      if rendering.as_ref().is_none_or(|(rendered_for, _)| rendered_for != size) {
        let preview = fitted(&source.core(), size.0, size.1);
        let (width, height) = preview.dimensions::<u32>();
        rendering = LiveImage::new(width, height, preview.into_rgba_vec()).ok().map(|image| (*size, image));
      }
      let skin = skin.as_ref().map(|mask| mask.core());
      let picture = rendering.as_mut()?.1.picture_with_skin_mask(live.iter(), skin.as_ref())?;
      Some(match picture {
        Picture::Pixels(frame) => RenderedFrame::Pixels(frame),
        Picture::Gpu(frame) => RenderedFrame::Gpu(frame),
      })
    }).map(|frame| match frame {
      RenderedFrame::Pixels(frame) => Picture::Pixels(Arc::clone(frame)),
      RenderedFrame::Gpu(frame) => Picture::Gpu(Arc::clone(frame)),
    })
  }
}

/// The image shrunk to fit within `width` x `height`, or as it is when it already fits.
pub(crate) fn fitted(image: &abra::prelude::Image, width: u32, height: u32) -> abra::prelude::Image {
  let mut shrunk = image.clone();
  let (current_width, current_height) = shrunk.dimensions::<u32>();
  if current_width > width || current_height > height {
    abra::transform::prelude::resize(abra::abra_core::ResizeTarget::Fit(abra::abra_core::Size::new(width as f32, height as f32)))
      .apply(&mut shrunk);
  }
  shrunk
}

fn frame_of(image: &abra::prelude::Image) -> Frame {
  let (width, height) = image.dimensions::<u32>();
  Frame { width, height, pixels: image.rgba().to_vec().into() }
}

/// The photo being edited: an image that follows the photo and the controls React touches. A `VesselView` shows it on a
/// native view, and the Vessel engine does the rest.
#[derive(uniffi::Object, Component)]
pub struct ImagePreview {
  image: Image,
  presented: Arc<AtomicBool>,
}

/// Everything React can tell a view.
#[derive(Clone, Debug, uniffi::Enum)]
pub enum Message {
  /// Shows these effects over the photo, in order, in place of any shown before; none shows the photo as it is. The
  /// controls define their own effects, so a new tool needs nothing here.
  Show(Vec<EffectSpec>),
  /// Shows this image, which a tool written in TypeScript made, in place of the photo and any effects. The next `Show`
  /// goes back to the photo.
  ShowImage(Arc<abra_uniffi::Image>),
  /// Makes the skin effects work on this mask of the skin.
  Skin(Arc<abra_uniffi::Mask>),
  /// Makes the skin effects find the skin by its color, as they do until they are given a mask.
  FindSkin,
}

#[uniffi::export]
impl ImagePreview {
  /// A view of `image`.
  #[uniffi::constructor]
  pub fn new(p_image: Arc<abra_uniffi::Image>) -> Arc<Self> {
    let source = Arc::new(PreviewSource::new(p_image));
    let image = Image::new().track(source.as_ref());
    let presented = Arc::new(AtomicBool::new(false));
    image.subject::<Drawn>().subscribe({
      let presented = Arc::clone(&presented);
      move |_| presented.store(true, Ordering::SeqCst)
    });
    let messages = image.subject::<Message>();

    messages.subscribe({
      let source = Arc::clone(&source);
      move |message| match message {
        Message::Show(effects) => {
          source.live.next(effects.clone());
          if source.tool_image.peek().is_some() { source.tool_image.next(None); }
        }
        Message::ShowImage(image) => source.tool_image.next(Some(Arc::new(image.core()))),
        Message::Skin(mask) => source.skin.next(Some(Arc::clone(mask))),
        Message::FindSkin => source.skin.next(None),
      }
    });

    Arc::new(Self {
      image,
      presented,
    })
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
    let image = Arc::new(abra_uniffi::Image::from_core(AbraPixels::new_from_pixels(
      4, 4, [100u8, 100, 100, 255].repeat(16), Channels::RGBA,
    )));
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

    view.send(Message::Show(vec![EffectSpec::Brightness { amount: 50 }]));
    assert_eq!(newest(&surface, |pixel| pixel == 150), 150);

    view.send(Message::Show(vec![EffectSpec::Brightness { amount: 0 }]));
    assert_eq!(newest(&surface, |pixel| pixel == 100), 100);

    view.send(Message::Show(vec![EffectSpec::Invert]));
    assert_eq!(newest(&surface, |pixel| pixel == 155), 155);
    view.send(Message::Show(vec![]));
    assert_eq!(newest(&surface, |pixel| pixel == 100), 100);
  }

  #[test]
  fn an_image_a_tool_made_replaces_the_photo_until_effects_are_shown_again() {
    let view = view(7004);
    let surface = surface_of(7004);
    assert_eq!(newest(&surface, |_| true), 100);
    // A tool answers with a new image of its own.
    let made = AbraPixels::new_from_color(2, 2, abra::abra_core::Color::from_rgba(30, 60, 90, 255));
    view.send(Message::ShowImage(Arc::new(abra_uniffi::Image::from_core(made))));
    assert_eq!(newest(&surface, |pixel| pixel == 30), 30);
    view.send(Message::Show(vec![]));
    assert_eq!(newest(&surface, |pixel| pixel == 100), 100);
  }

  #[test]
  fn a_tool_s_image_that_is_still_a_recipe_is_drawn_by_the_gpu_and_matches_its_pixels() {
    use abra_uniffi::{Color, ColorStop, Gradient, blend, radial_gradient};

    let view = view(7005);
    let surface = surface_of(7005);
    assert_eq!(newest(&surface, |_| true), 100);
    // A solid dark red over the gray photo, made with the same calls a tool makes. On a machine with a GPU the result is
    // a recipe the view draws itself; without one it is plain pixels. Either way the view shows the red.
    let source = Arc::new(abra_uniffi::Image::from_core(AbraPixels::new_from_pixels(
      4, 4, [100u8, 100, 100, 255].repeat(16), Channels::RGBA,
    )));
    let red = Arc::new(Color::from_rgba(50, 0, 0, 255));
    let gradient = Arc::new(Gradient::new(vec![
      Arc::new(ColorStop::new(red.clone(), 0.0)),
      Arc::new(ColorStop::new(red, 1.0)),
    ]));
    let edges = radial_gradient(4, 4, gradient, 2.0, 2.0, 3.0, 3.0);
    // As a tool writes it: blend the edges onto the photo it was given, which is its own to change.
    blend(edges).apply(source.clone());
    view.send(Message::ShowImage(source));
    assert_eq!(newest(&surface, |pixel| pixel == 50), 50);
  }

  #[test]
  fn dropping_the_view_takes_it_off_the_native_view() {
    let surface = surface_of(7003);
    drop(view(7003));
    let drawn = surface.0.lock().unwrap().len();
    let view = view(7003);
    view.send(Message::Show(vec![EffectSpec::Brightness { amount: 50 }]));
    assert_eq!(newest(&surface, |pixel| pixel == 150), 150);
    assert!(surface.0.lock().unwrap().len() > drawn);
  }
}
