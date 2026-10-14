use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use vessel_api::prelude::*;
use vessel_macros::Component;

use crate::camera::Camera;

/// How a picture fills the room an [`Image`] has, like CSS `object-fit`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fit {
  /// The whole picture, as large as fits, keeping its proportions, centered. There may be empty bands at the sides.
  #[default]
  Contain,
  /// The room is filled, keeping the picture's proportions, so some of the picture may be cropped.
  Cover,
  /// The whole picture is stretched to fill the room, whatever its proportions.
  Stretch,
}

/// The part of a source a view shows, as a rectangle in the source's own space: `0.0` is the left or top edge and `1.0`
/// the right or bottom. The default is the whole source. Zoom, crop previews, loupes and split views all just set it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
  /// The left edge.
  pub x: f32,
  /// The top edge.
  pub y: f32,
  /// How much of the source's width is shown.
  pub width: f32,
  /// How much of the source's height is shown.
  pub height: f32,
}

impl Default for Viewport {
  fn default() -> Self {
    Self { x: 0.0, y: 0.0, width: 1.0, height: 1.0 }
  }
}

impl Viewport {
  /// This rectangle in the pixels of a source `p_width` x `p_height`, as `(x, y, width, height)`. It is kept inside the
  /// source and at least one pixel in each direction, so it is always something that can be cut out.
  pub fn in_pixels(&self, p_width: u32, p_height: u32) -> (u32, u32, u32, u32) {
    let left = (self.x.clamp(0.0, 1.0) * p_width as f32).floor() as u32;
    let top = (self.y.clamp(0.0, 1.0) * p_height as f32).floor() as u32;
    let right = ((self.x + self.width).clamp(0.0, 1.0) * p_width as f32).ceil() as u32;
    let bottom = ((self.y + self.height).clamp(0.0, 1.0) * p_height as f32).ceil() as u32;
    let left = left.min(p_width.saturating_sub(1));
    let top = top.min(p_height.saturating_sub(1));
    (left, top, right.saturating_sub(left).max(1), bottom.saturating_sub(top).max(1))
  }
}

/// A picture an [`Image`] can show: pixels, or a picture that was made on the GPU.
#[derive(Clone)]
pub enum Picture {
  /// Pixels in CPU memory. Shared, so passing the picture around copies nothing.
  Pixels(Arc<Frame>),
  /// A picture in GPU memory, drawn on the screen without being copied to the CPU.
  Gpu(Arc<dyn GpuFrame>),
}

/// Something an [`Image`] can follow: it makes the pictures to show, at the size the image has, and makes new ones whenever
/// it changes. A photo, a video, a camera or a game can all be tracked; the image knows nothing about which.
pub trait Trackable {
  /// The pictures of this, as a stream. `p_size` is the size in pixels the image is drawn at, so the pictures can be made
  /// to fit: it is `(0, 0)` until the image is shown somewhere, and changes when the view does.
  ///
  /// `p_viewport` is the part of the source to show. A new viewport means a new picture. A source that cannot show part
  /// of itself ignores it and always makes the whole picture.
  fn pictures(&self, p_size: &BehaviorSubject<(u32, u32)>, p_viewport: &BehaviorSubject<Viewport>) -> Subject<Picture>;
}

struct FittedCacheEntry {
  // Keep the source alive: allocator address reuse must never turn a new picture into a cache hit.
  source: Arc<Frame>,
  room: (u32, u32),
  fit: Fit,
  fitted: (i32, i32, u32, u32),
  pixels: Arc<Vec<u8>>,
}

#[derive(Default)]
struct PictureInner {
  picture: Option<Picture>,
  /// CPU composition reads a published GPU picture at most once, even when fit or host styling changes.
  readback: Option<Arc<Frame>>,
}

impl<T: Trackable> Trackable for Arc<T> {
  fn pictures(&self, p_size: &BehaviorSubject<(u32, u32)>, p_viewport: &BehaviorSubject<Viewport>) -> Subject<Picture> {
    self.as_ref().pictures(p_size, p_viewport)
  }
}

/// A picture, drawn inside the image's box (inside its border and padding).
///
/// Give it a [`Picture`] with [`show`](Self::show): pixels, or a picture made on the GPU, which reaches the screen without
/// ever being copied to the CPU. [`set_pixels`](Self::set_pixels) and [`show_gpu`](Self::show_gpu) are the short forms. Like any component it has
/// the box settings (`set_padding`, `set_radius`, `set_background`, ...), and it sizes and places itself in a parent's
/// layout. It is generic: it knows nothing about where the picture comes from, so a photo editor, a video player or a
/// camera can all feed one.
///
/// ```ignore
/// let photo = Image::new().with_fit(Fit::Cover);
/// photo.set_pixels(width, height, rgba);   // redraws
/// page.add(photo.fill());
/// ```
#[derive(Clone, Component)]
#[component(plain)]
pub struct Image {
  component: Component,
  picture: Arc<Mutex<PictureInner>>,
  /// Counts the pictures set, so the drawing listener redraws for each one without comparing pixels.
  revision: BehaviorSubject<u64>,
  fit: BehaviorSubject<Fit>,
  /// The part of the source shown, given to every source this tracks. Drawing does not read it: the sources make a new
  /// picture when it changes.
  viewport: BehaviorSubject<Viewport>,
  /// Moves `viewport` toward where the app last pointed it, a little each frame.
  camera: Arc<Mutex<Camera>>,
  /// Whether the component currently shows a GPU picture instead of pixels.
  on_gpu: Arc<AtomicBool>,
  /// The streams of pictures this follows, and the listening to them, kept for as long as the image lives.
  sources: Arc<Mutex<Vec<(Subject<Picture>, Subscription)>>>,
  fitted_cache: Arc<Mutex<Option<FittedCacheEntry>>>,
}

impl Image {
  component_builders!();
  color_background_builder!();

  /// An image with nothing to show yet, fitted with [`Fit::Contain`].
  pub fn new() -> Self {
    let picture = Arc::new(Mutex::new(PictureInner::default()));
    let revision = BehaviorSubject::new(0u64);
    let fit = BehaviorSubject::new(Fit::default());
    let fitted_cache = Arc::new(Mutex::new(None::<FittedCacheEntry>));

    let component = Component::new("image");
    component.subject::<Canvas>().subscribe({
      let (picture, revision, fit, fitted_cache) =
        (Arc::clone(&picture), revision.clone(), fit.clone(), Arc::clone(&fitted_cache));
      move |canvas| {
        // Reading these is what makes the image depend on them.
        revision.value();
        let fit = fit.value();
        let frame = {
          let picture = picture.lock().unwrap();
          let Some(Picture::Pixels(frame)) = picture.picture.as_ref() else { return };
          Arc::clone(frame)
        };
        let (left, top, width, height) = canvas.content_rect();
        if (frame.width, frame.height) == (width, height) {
          canvas.draw_pixels(left, top, width, height, frame.pixels.as_slice());
        } else if let Some((x, y, width, height, pixels)) = fitted_cached(&fitted_cache, &frame, width, height, fit) {
          canvas.draw_pixels(left + x, top + y, width, height, pixels.as_slice());
        }
      }
    });

    let viewport = BehaviorSubject::new(Viewport::default());
    let camera = Arc::new(Mutex::new(Camera::new(Viewport::default())));
    component.subject::<Tick>().subscribe({
      let (weak, viewport, camera) = (component.downgrade(), viewport.clone(), Arc::clone(&camera));
      move |tick| {
        let (next, moving) = {
          let mut camera = camera.lock().unwrap();
          (camera.step(tick.0), camera.is_moving())
        };
        viewport.next(next);
        if !moving && let Some(component) = weak.upgrade() {
          component.set_animating(false);
        }
      }
    });

    Self {
      component,
      picture,
      revision,
      fit,
      viewport,
      camera,
      on_gpu: Arc::new(AtomicBool::new(false)),
      sources: Arc::default(),
      fitted_cache,
    }
  }

  /// Follows `p_source`: every picture it sends is shown, until the image is dropped. The image keeps the stream alive, so
  /// the caller doesn't have to hold anything.
  pub fn set_source(&self, p_source: &Subject<Picture>) {
    // The listener gets a copy that does not hold the list of sources, so the image and its sources don't hold each other.
    let shown = Image {
      sources: Arc::default(),
      ..self.clone()
    };
    let following = p_source.subscribe(move |picture| shown.show(picture));
    self.sources.lock().unwrap().push((p_source.clone(), following));
  }

  /// Follows `p_source`: it is shown at the image's size, and the image follows every picture it makes. Returns the image so
  /// calls can be chained.
  pub fn track(self, p_source: &impl Trackable) -> Self {
    self.set_source(&p_source.pictures(self.size(), &self.viewport));
    self
  }

  /// Follows `p_source`, and returns the image so calls can be chained.
  pub fn with_source(self, p_source: &Subject<Picture>) -> Self {
    self.set_source(p_source);
    self
  }

  /// Shows `p_rgba` (`p_width * p_height` RGBA pixels). Pixels of the wrong length are ignored.
  pub fn set_pixels(&self, p_width: u32, p_height: u32, p_rgba: Vec<u8>) {
    if p_rgba.len() != p_width as usize * p_height as usize * 4 {
      return;
    }
    self.show(&Picture::Pixels(Arc::new(Frame {
      width: p_width,
      height: p_height,
      pixels: Arc::new(p_rgba),
    })));
  }

  /// Shows `p_picture`: pixels are drawn in the image's box, and a GPU picture replaces everything (see
  /// [`show_gpu`](Self::show_gpu)).
  pub fn show(&self, p_picture: &Picture) {
    match p_picture {
      Picture::Pixels(frame) => {
        *self.picture.lock().unwrap() = PictureInner {
          picture: Some(Picture::Pixels(Arc::clone(frame))),
          readback: None,
        };
        *self.fitted_cache.lock().unwrap() = None;
        self.back_to_pixels();
        self.revision.next(self.revision.value() + 1);
      }
      Picture::Gpu(frame) => self.show_gpu(Arc::clone(frame)),
    }
  }

  /// Shows `p_rgba`, and returns the image so calls can be chained.
  pub fn with_pixels(self, p_width: u32, p_height: u32, p_rgba: Vec<u8>) -> Self {
    self.set_pixels(p_width, p_height, p_rgba);
    self
  }

  /// Shows a picture that lives on the GPU, drawn on the screen without being copied to the CPU. While it is shown it is
  /// the whole picture, stretched to the image's room (the fit does not apply). A surface that cannot draw from the GPU
  /// reads the picture back, so it is still shown, only slower.
  /// When used as an image background, CPU readback is cached until another picture is published, including with the
  /// same GPU handle. Changing the background's fit or host styling reuses that readback.
  pub fn show_gpu(&self, p_frame: Arc<dyn GpuFrame>) {
    *self.picture.lock().unwrap() = PictureInner {
      picture: Some(Picture::Gpu(Arc::clone(&p_frame))),
      readback: None,
    };
    *self.fitted_cache.lock().unwrap() = None;
    self.on_gpu.store(true, Ordering::SeqCst);
    self.component.subject::<GpuPicture>().next(GpuPicture(Some(p_frame)));
    self.revision.next(self.revision.value() + 1);
  }

  pub(crate) fn draw_background(&self, p_canvas: &Canvas) {
    let size = (p_canvas.width(), p_canvas.height());
    if self.size().peek() != size {
      self.size().next(size);
    }
    self.revision.value();
    let fit = self.fit.value();
    let frame = {
      let mut picture = self.picture.lock().unwrap();
      match picture.picture.as_ref() {
        None => return,
        Some(Picture::Pixels(frame)) => Arc::clone(frame),
        Some(Picture::Gpu(frame)) => {
          if picture.readback.is_none() {
            picture.readback = Some(Arc::new(frame.read_back().expect("background GPU image could not be read back")));
          }
          Arc::clone(picture.readback.as_ref().unwrap())
        }
      }
    };
    if (frame.width, frame.height) == size {
      p_canvas.draw_background_pixels(0, 0, frame.width, frame.height, frame.pixels.as_slice());
    } else if let Some((x, y, width, height, pixels)) = fitted_cached(&self.fitted_cache, &frame, size.0, size.1, fit) {
      p_canvas.draw_background_pixels(x, y, width, height, pixels.as_slice());
    }
  }

  /// Shows nothing.
  pub fn clear(&self) {
    *self.picture.lock().unwrap() = PictureInner::default();
    *self.fitted_cache.lock().unwrap() = None;
    self.back_to_pixels();
    self.revision.next(self.revision.value() + 1);
  }

  /// Stops showing a GPU picture, if one was shown, so the pixels are drawn again.
  fn back_to_pixels(&self) {
    if self.on_gpu.swap(false, Ordering::SeqCst) {
      self.component.subject::<GpuPicture>().next(GpuPicture(None));
    }
  }

  /// Sends the view toward `p_viewport`. It glides there over the next frames, at the camera's speed, instead of jumping.
  pub fn set_viewport(&self, p_viewport: Viewport) {
    self.camera.lock().unwrap().set_target(p_viewport);
    self.component.set_animating(true);
  }

  /// Sends the view toward `p_viewport`, and returns the image so calls can be chained.
  pub fn with_viewport(self, p_viewport: Viewport) -> Self {
    self.set_viewport(p_viewport);
    self
  }

  /// Shows `p_viewport` at once, with no gliding.
  pub fn jump_viewport(&self, p_viewport: Viewport) {
    self.camera.lock().unwrap().jump_to(p_viewport);
    self.viewport.next(p_viewport);
  }

  /// The viewport being shown right now, which is on its way to the one last set.
  pub fn viewport(&self) -> Viewport {
    self.viewport.value()
  }

  properties! {
    fit: Fit => set_fit, with_fit;
  }
}

impl Default for Image {
  fn default() -> Self {
    Self::new()
  }
}

/// The picture scaled to `p_width` x `p_height` of room as `p_fit` says, with where it goes inside that room:
/// `(x, y, width, height, pixels)`. `None` when there is nothing to draw.
pub(crate) fn fitted(
  p_frame: &Frame, p_width: u32, p_height: u32, p_fit: Fit,
) -> Option<(i32, i32, u32, u32, Vec<u8>)> {
  if p_frame.width == 0 || p_frame.height == 0 || p_width == 0 || p_height == 0 {
    return None;
  }
  let (source_width, source_height) = (p_frame.width as f32, p_frame.height as f32);
  let (room_width, room_height) = (p_width as f32, p_height as f32);
  // The part of the picture that is shown, and the size it is drawn at.
  let (source, width, height) = match p_fit {
    Fit::Stretch => ((0.0, 0.0, source_width, source_height), p_width, p_height),
    Fit::Contain => {
      let scale = (room_width / source_width).min(room_height / source_height);
      (
        (0.0, 0.0, source_width, source_height),
        ((source_width * scale).round() as u32).clamp(1, p_width),
        ((source_height * scale).round() as u32).clamp(1, p_height),
      )
    }
    Fit::Cover => {
      let scale = (room_width / source_width).max(room_height / source_height);
      let (shown_width, shown_height) = (room_width / scale, room_height / scale);
      (
        ((source_width - shown_width) / 2.0, (source_height - shown_height) / 2.0, shown_width, shown_height),
        p_width,
        p_height,
      )
    }
  };
  let mut pixels = vec![0u8; width as usize * height as usize * 4];
  for y in 0..height {
    let from_y =
      (source.1 + (y as f32 + 0.5) * source.3 / height as f32).floor().clamp(0.0, source_height - 1.0) as usize;
    for x in 0..width {
      let from_x =
        (source.0 + (x as f32 + 0.5) * source.2 / width as f32).floor().clamp(0.0, source_width - 1.0) as usize;
      let from = (from_y * p_frame.width as usize + from_x) * 4;
      let to = ((y * width + x) * 4) as usize;
      pixels[to..to + 4].copy_from_slice(&p_frame.pixels[from..from + 4]);
    }
  }
  Some((((p_width - width) / 2) as i32, ((p_height - height) / 2) as i32, width, height, pixels))
}

fn fitted_cached(
  p_cache: &Arc<Mutex<Option<FittedCacheEntry>>>, p_frame: &Arc<Frame>, p_width: u32, p_height: u32, p_fit: Fit,
) -> Option<(i32, i32, u32, u32, Arc<Vec<u8>>)> {
  let room = (p_width, p_height);
  let mut cache = p_cache.lock().unwrap();
  if let Some(entry) =
    cache.as_ref().filter(|entry| Arc::ptr_eq(&entry.source, p_frame) && entry.room == room && entry.fit == p_fit)
  {
    let (x, y, width, height) = entry.fitted;
    return Some((x, y, width, height, Arc::clone(&entry.pixels)));
  }

  let (x, y, width, height, pixels) = fitted(p_frame, p_width, p_height, p_fit)?;
  let pixels = Arc::new(pixels);
  *cache = Some(FittedCacheEntry {
    source: Arc::clone(p_frame),
    room,
    fit: p_fit,
    fitted: (x, y, width, height),
    pixels: Arc::clone(&pixels),
  });
  Some((x, y, width, height, pixels))
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;

  /// Makes a 1x1 picture whose red is the viewport's left edge in percent, so a test can see which viewport it was given.
  struct ViewportEcho;

  impl Trackable for ViewportEcho {
    fn pictures(&self, p_size: &BehaviorSubject<(u32, u32)>, p_viewport: &BehaviorSubject<Viewport>) -> Subject<Picture> {
      p_size.combine_latest(p_viewport).map(|(_, viewport)| {
        Picture::Pixels(Arc::new(Frame { width: 1, height: 1, pixels: Arc::new(vec![(viewport.x * 100.0) as u8, 0, 0, 255]) }))
      })
    }
  }

  #[test]
  fn a_tracked_source_gets_the_viewport_and_a_new_one_makes_a_new_picture() {
    let image = Image::new().track(&ViewportEcho);
    image.size().next((4, 4));
    let red = |image: &Image| match image.picture.lock().unwrap().picture.as_ref() {
      Some(Picture::Pixels(frame)) => frame.pixels[0],
      _ => panic!("no pixels"),
    };
    assert_eq!(red(&image), 0, "the whole source until told otherwise");
    image.jump_viewport(Viewport { x: 0.5, y: 0.0, width: 0.5, height: 1.0 });
    assert_eq!(red(&image), 50);
    image.jump_viewport(Viewport { x: 0.25, y: 0.0, width: 0.5, height: 1.0 });
    assert_eq!(red(&image), 25);
  }

  #[test]
  fn set_viewport_glides_one_tick_at_a_time_and_then_lets_the_engine_sleep() {
    let image = Image::new().track(&ViewportEcho);
    image.size().next((4, 4));
    let target = Viewport { x: 0.5, y: 0.0, width: 0.5, height: 1.0 };
    assert_eq!(image.component.clone().pacing(), Pacing::OnDemand);
    image.set_viewport(target);
    assert_eq!(image.viewport(), Viewport::default(), "nothing moves until a frame is made");
    assert_eq!(image.component.clone().pacing(), Pacing::Continuous);

    image.component.clone().render(Duration::from_millis(16));
    let first = image.viewport();
    assert!(first.x > 0.0 && first.x < 0.5, "one frame moves part of the way: {first:?}");
    for _ in 0..200 {
      image.component.clone().render(Duration::from_millis(16));
    }
    assert_eq!(image.viewport(), target);
    assert_eq!(image.component.clone().pacing(), Pacing::OnDemand, "it stops asking for frames once it arrives");
  }

  #[test]
  fn the_default_viewport_is_the_whole_source_and_others_stay_inside_it() {
    assert_eq!(Viewport::default().in_pixels(400, 300), (0, 0, 400, 300));
    // A half-size region in the middle.
    assert_eq!(Viewport { x: 0.25, y: 0.25, width: 0.5, height: 0.5 }.in_pixels(400, 300), (100, 75, 200, 150));
    // A region that touches the right edge and hangs past it is cut at the edge.
    assert_eq!(Viewport { x: 0.75, y: 0.0, width: 0.5, height: 1.0 }.in_pixels(400, 300), (300, 0, 100, 300));
    // A tiny or empty region is still one pixel, even outside the source.
    assert_eq!(Viewport { x: 2.0, y: -1.0, width: 0.0, height: 0.0 }.in_pixels(400, 300), (399, 0, 1, 1));
  }

  #[test]
  fn fitted_background_pixels_are_reused_and_retired_sources_are_released() {
    let source = Arc::new(Frame {
      width: 2,
      height: 1,
      pixels: Arc::new(vec![200, 0, 0, 255, 0, 0, 200, 255]),
    });
    let retired = Arc::downgrade(&source);
    let image = Image::new().with_fit(Fit::Stretch);
    image.show(&super::Picture::Pixels(source));
    let canvas = Canvas::new(12, 8);
    image.draw_background(&canvas);
    let first = Arc::clone(&image.fitted_cache.lock().unwrap().as_ref().unwrap().pixels);
    image.draw_background(&canvas);
    assert!(
      Arc::ptr_eq(&first, &image.fitted_cache.lock().unwrap().as_ref().unwrap().pixels),
      "unchanged pictures must not be rescaled"
    );

    image.set_fit(Fit::Contain);
    image.draw_background(&canvas);
    assert!(!Arc::ptr_eq(&first, &image.fitted_cache.lock().unwrap().as_ref().unwrap().pixels));
    image.set_pixels(1, 1, vec![0, 200, 0, 255]);
    assert!(retired.upgrade().is_none(), "publishing a new picture releases the retained cache source");
    image.draw_background(&canvas);
    image.clear();
    assert!(image.fitted_cache.lock().unwrap().is_none());
  }

  fn render(p_image: &Image) -> Frame {
    let mut component = p_image.component.clone();
    component.render(Duration::ZERO)
  }

  fn pixel(p_frame: &Frame, p_x: u32, p_y: u32) -> [u8; 4] {
    let at = ((p_y * p_frame.width + p_x) * 4) as usize;
    p_frame.pixels[at..at + 4].try_into().unwrap()
  }

  /// A 2x1 picture: one red pixel, then one blue.
  fn red_then_blue() -> Vec<u8> {
    vec![255, 0, 0, 255, 0, 0, 255, 255]
  }

  fn sized(p_image: Image, p_width: u32, p_height: u32) -> Image {
    p_image.size().next((p_width, p_height));
    p_image
  }

  #[test]
  fn it_draws_nothing_until_it_has_a_picture_and_the_picture_one_to_one_when_the_size_matches() {
    let image = sized(Image::new(), 2, 1);
    assert!(render(&image).pixels.iter().all(|byte| *byte == 0));
    image.set_pixels(2, 1, red_then_blue());
    let frame = render(&image);
    assert_eq!((pixel(&frame, 0, 0), pixel(&frame, 1, 0)), ([255, 0, 0, 255], [0, 0, 255, 255]));
  }

  #[test]
  fn contain_keeps_the_proportions_and_centers_the_picture() {
    let image = sized(Image::new().with_pixels(2, 1, red_then_blue()), 4, 4);
    let frame = render(&image);
    assert_eq!(pixel(&frame, 0, 0)[3], 0, "the band above the picture is empty");
    assert_eq!(pixel(&frame, 0, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 3, 2), [0, 0, 255, 255]);
    assert_eq!(pixel(&frame, 0, 3)[3], 0, "and so is the band below it");
  }

  #[test]
  fn cover_fills_the_room_and_crops_and_stretch_fills_it_with_the_whole_picture() {
    let cover = sized(Image::new().with_fit(Fit::Cover).with_pixels(2, 1, red_then_blue()), 2, 2);
    let frame = render(&cover);
    assert!(frame.pixels.chunks_exact(4).all(|pixel| pixel[3] == 255), "no band is left empty");

    let stretch = sized(Image::new().with_fit(Fit::Stretch).with_pixels(2, 1, red_then_blue()), 4, 2);
    let frame = render(&stretch);
    assert_eq!((pixel(&frame, 0, 0), pixel(&frame, 1, 1)), ([255, 0, 0, 255], [255, 0, 0, 255]));
    assert_eq!((pixel(&frame, 2, 0), pixel(&frame, 3, 1)), ([0, 0, 255, 255], [0, 0, 255, 255]));
  }

  #[test]
  fn it_has_the_box_settings_of_any_component_so_padding_keeps_the_picture_inside() {
    let image = sized(Image::new().with_pixels(2, 1, red_then_blue()), 6, 3);
    image.set_padding([0, 2]); // up and down 0, left and right 2: the picture gets a 2x3 room
    let frame = render(&image);
    assert_eq!(pixel(&frame, 0, 1)[3], 0, "the padding stays clear");
    assert_eq!(pixel(&frame, 2, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 3, 1), [0, 0, 255, 255]);
    assert_eq!(pixel(&frame, 5, 1)[3], 0);
  }

  #[test]
  fn a_new_picture_or_fit_redraws_it_without_anything_declared() {
    let image = sized(Image::new().with_pixels(2, 1, red_then_blue()), 2, 1);
    render(&image);
    assert!(!image.component.clone().has_changed());
    image.set_pixels(2, 1, vec![9, 9, 9, 255, 8, 8, 8, 255]);
    assert!(image.component.clone().has_changed());
    assert_eq!(pixel(&render(&image), 0, 0), [9, 9, 9, 255]);
    image.set_fit(Fit::Cover);
    assert!(image.component.clone().has_changed());
    image.clear();
    assert!(render(&image).pixels.iter().all(|byte| *byte == 0), "cleared");
  }

  #[test]
  fn pixels_of_the_wrong_length_are_ignored() {
    let image = sized(Image::new().with_pixels(2, 1, red_then_blue()), 2, 1);
    image.set_pixels(2, 2, vec![1; 8]);
    assert_eq!(pixel(&render(&image), 0, 0), [255, 0, 0, 255], "the earlier picture stays");
  }

  struct FakeGpuFrame;

  impl GpuFrame for FakeGpuFrame {
    fn size(&self) -> (u32, u32) {
      (2, 1)
    }

    fn as_any(&self) -> &dyn std::any::Any {
      self
    }
  }

  #[test]
  fn a_gpu_picture_is_shown_without_pixels_until_pixels_are_set_again() {
    let image = sized(Image::new().with_pixels(2, 1, red_then_blue()), 2, 1);
    let mut source = image.component.clone();
    assert!(source.render_gpu(Duration::ZERO).is_none());

    image.show_gpu(Arc::new(FakeGpuFrame));
    assert!(source.render_gpu(Duration::ZERO).is_some(), "the GPU picture is what is shown");

    image.set_pixels(2, 1, red_then_blue());
    assert!(source.render_gpu(Duration::ZERO).is_none(), "pixels replace it");
  }

  #[test]
  fn it_can_be_added_to_a_parent_and_laid_out_like_any_component() {
    let parent = Component::new("parent").with_display(Display::Flex).with_size(100, 40);
    let image = Image::new();
    parent.add(image.clone().width(30));
    parent.add(Component::new("rest"));
    parent.clone().render(Duration::ZERO);
    assert_eq!(image.size().value(), (30, 40));
  }

  #[test]
  fn it_follows_a_stream_of_pictures_without_the_caller_holding_anything() {
    let image = sized(Image::new(), 2, 1);
    let pictures = Subject::<super::Picture>::new();
    image.set_source(&pictures);
    drop(pictures);
    // The image kept the stream alive, so it still hears it through the list it holds.
    let source = image.sources.lock().unwrap()[0].0.clone();
    source.next(super::Picture::Pixels(Arc::new(Frame {
      width: 2,
      height: 1,
      pixels: Arc::new(red_then_blue()),
    })));
    assert_eq!(pixel(&render(&image), 1, 0), [0, 0, 255, 255]);
  }
}
