//! The photo being edited: a component that React creates and the Vessel engine draws.
//!
//! `LiveView` keeps a Vessel `Image` and gives it something to follow: the pictures made from the messages React sends.
//! The chain of effects is the state those messages add up to, and a picture is that chain rendered over the photo. On a
//! GPU the picture is a texture that goes to the screen without being copied to the CPU. Everything else (when to draw, the
//! native view, the GPU target) is the engine's.

use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicU64, Ordering},
};

use abra::prelude::ImageExt;
use vessel::prelude::*;

use crate::components::{camera::Camera, frame_of};


pub(crate) struct PreviewSource {
  /// The picture that is shown, as it is now. For an image it is the image, which is shared: whoever holds the other end edits it, and
  /// the picture follows. For a canvas it is the canvas composed, which is done here, on the engine's thread, each time the canvas
  /// has changed.
  image: Arc<dyn Fn() -> abra::prelude::Image + Send + Sync>,
  /// Names what is shown: it is a different number each time the picture has to be made again.
  version: Arc<dyn Fn() -> u64 + Send + Sync>,
  /// Wakes the renderer that makes the pictures, once there is one, when the image is edited.
  revisions: Arc<Revisions>,
  /// The size in pixels of the picture that is shown, which is the size the photo's points are in. It is `(0, 0)` until the first picture is made.
  pub(crate) photo: BehaviorSubject<(u32, u32)>,
}

impl PreviewSource {
  pub(crate) fn new(p_image: Arc<abra_uniffi::Image>) -> Self {
    let version = Arc::clone(&p_image);
    Self {
      image: Arc::new({
        let image = Arc::clone(&p_image);
        move || image.core()
      }),
      version: Arc::new(move || version.core().version()),
      revisions: Revisions::new(),
      photo: BehaviorSubject::new((0, 0)),
    }
  }

  /// A source that shows a canvas, composed whenever it has changed since it was last composed.
  pub(crate) fn of_canvas(p_canvas: Arc<abra_uniffi::Canvas>) -> Self {
    let changes = Arc::new(AtomicU64::new(0));
    let version = Arc::clone(&p_canvas);
    Self {
      image: Arc::new({
        let canvas = Arc::clone(&p_canvas);
        move || canvas.as_image().core()
      }),
      // A canvas that has changed is a new number until it has been composed, which is what the renderer is waiting for.
      version: Arc::new(move || {
        if version.is_dirty() {
          changes.fetch_add(1, Ordering::SeqCst);
        }
        changes.load(Ordering::SeqCst)
      }),
      revisions: Revisions::new(),
      photo: BehaviorSubject::new((0, 0)),
    }
  }

  /// Draws again at once if what is shown has changed since it was drawn, such as a photo that was edited in place.
  fn refresh(&self) {
    self.revisions.wake();
  }
}

/// How many different photos a preview keeps on the GPU: the one drawn now and the one before it.
const PHOTOS_ON_THE_GPU: usize = 2;

/// How many tiles past the edge of the view are put on the GPU ahead of time, so a pan finds them there.
const PREFETCHED_TILES: u32 = 1;

/// A frame that takes longer than this is written to the log, with how many tiles are on the GPU.
const SLOW_FRAME: std::time::Duration = std::time::Duration::from_millis(20);

/// A photo on the GPU, kept for as long as the same pixels are drawn, so a slider that only changes the effects over it moves
/// nothing but a few uniforms.
struct PhotoOnTheGpu {
  /// Names the pixels: which version of the image and how big.
  key: (u64, (u32, u32)),
  _root: abra::prelude::Image,
  photo: gpu::TiledRenderer,
}

impl Trackable for PreviewSource {
  fn pictures(&self, size: &BehaviorSubject<(u32, u32)>, viewport: &BehaviorSubject<Viewport>, rotation: &BehaviorSubject<f32>) -> Subject<Picture> {
    let source = Arc::clone(&self.image);
    let photo = self.photo.clone();
    let changes = size.combine_latest(viewport).combine_latest(rotation);
    // The renderer draws again whenever the image is a different one (see `ImagePreview::refresh`), and keeps drawing while tiles
    // are still on their way; between them it sleeps.
    let version = {
      let source = Arc::clone(&self.version);
      move || source()
    };
    let busy = Arc::new(AtomicBool::new(false));
    // The photos on the GPU, the one drawn last at the end. An edit that is a smaller picture (a mood, say) and then undone brings
    // the full-size photo back, which must not be copied up and built again.
    let mut held: Vec<PhotoOnTheGpu> = Vec::new();
    let renderer = Offscreen::tracking(&changes, version, { let busy = Arc::clone(&busy); move || busy.load(Ordering::SeqCst) }, move |((size, viewport), angle)| {
      busy.store(false, Ordering::SeqCst);
      if size.0 == 0 || size.1 == 0 { return None }
      // An image that is still a recipe is the pixels it started from and the passes over them, so the GPU keeps those pixels
      // and a drag only changes the passes.
      let image = source();
      let shown = image.dimensions::<u32>();
      if photo.peek() != shown {
        photo.next(shown);
      }
      let (root, passes) = abra::abra_core::image::recipe::gpu_run(&image).unwrap_or_else(|| (image.clone(), Vec::new()));
      let (photo_width, photo_height) = root.dimensions::<u32>();
      // The view is the whole box and the viewport says which part of the photo it shows, which may reach past the photo.
      let window = gpu::Window {
        x: viewport.x * photo_width as f32,
        y: viewport.y * photo_height as f32,
        width: viewport.width * photo_width as f32,
        height: viewport.height * photo_height as f32,
        out_width: size.0,
        out_height: size.1,
        pad: 0,
        angle: *angle,
      };
      let on_the_gpu = gpu::context().and_then(|context| {
        let key = (root.version(), (photo_width, photo_height));
        if let Some(at) = held.iter().position(|photo| photo.key == key) {
          let photo = held.remove(at);
          held.push(photo);
        } else {
          // Outside the photo is black, drawn after the effects so they do not tint it.
          let options = gpu::TileOptions::default().with_outside([0.0, 0.0, 0.0, 1.0]).with_prefetch(PREFETCHED_TILES);
          // The photo is not copied up: each tile asks the image for the part of it that it shows, and the image makes only that.
          let regions: gpu::Regions = {
            let root = root.clone();
            Arc::new(move |level, left, top, width, height| root.region(level, left, top, width, height))
          };
          let photo = gpu::TiledRenderer::from_regions(context.clone(), photo_width, photo_height, regions, options)
            .map_err(|error| vessel::surface::log(&format!("[preview] the photo could not be put on the GPU: {error}")))
            .ok()?;
          held.push(PhotoOnTheGpu { key, _root: root.clone(), photo });
          if held.len() > PHOTOS_ON_THE_GPU {
            held.remove(0);
          }
        }
        let photo = &mut held.last_mut()?.photo;
        // An effect that cannot be drawn at this zoom, or a failure, is drawn on the CPU below instead.
        let started = std::time::Instant::now();
        let view = photo.render(&window, &passes).map_err(|error| vessel::surface::log(&format!("[preview] a frame could not be rendered: {error}"))).ok()?;
        let texture = photo.snapshot_texture().map_err(|error| vessel::surface::log(&format!("[preview] a frame could not be kept: {error}"))).ok()?;
        // Tiles that did not fit in this frame's time are drawn from blurrier ones now and put on in the frames that follow.
        busy.store(photo.wants_more(), Ordering::SeqCst);
        let took = started.elapsed();
        if took > SLOW_FRAME {
          vessel::surface::log(&format!("[preview] a frame took {} ms with {} tiles on the GPU", took.as_millis(), photo.tiles_loaded()));
        }
        Some(RenderedFrame::Gpu(Arc::new(gpu_passes::target::TextureFrame::new(context, texture).with_view(view))))
      });
      if on_the_gpu.is_some() {
        return on_the_gpu;
      }
      // No GPU, or it could not draw this: the pixels the tool made (or the photo) on the CPU.
      Some(RenderedFrame::Pixels(Arc::new(view_on_the_cpu(&image, viewport, *angle, *size))))
    });
    self.revisions.attach(&renderer);
    renderer.map(|frame| Picture::from(frame))
  }
}

/// The picture the GPU would draw, made on the CPU: `image` seen through the part of it that `viewport` names (shares of the image, which may
/// reach past its edges) turned `angle` radians clockwise, in a box of `size` pixels, on black where the view reaches past the image.
fn view_on_the_cpu(image: &abra::prelude::Image, viewport: &Viewport, angle: f32, size: (u32, u32)) -> Frame {
  let (width, height) = image.dimensions::<u32>();
  let window = abra::transform::prelude::ViewWindow::new(
    viewport.x * width as f32,
    viewport.y * height as f32,
    viewport.width * width as f32,
    viewport.height * height as f32,
  )
  .with_angle(angle);
  frame_of(&abra::transform::prelude::view(image, window, size, abra::abra_core::Color::black()))
}

/// The photo being edited: an image that follows the photo and the controls React touches. A `VesselView` shows it on a
/// native view, and the Vessel engine does the rest.
#[derive(uniffi::Object, Component)]
pub struct ImagePreview {
  pub(crate) image: Image,
  presented: Arc<AtomicBool>,
  pub(crate) source: Arc<PreviewSource>,
}

#[uniffi::export]
impl ImagePreview {
  /// A view of `image`, which it shares: edit the image and call [`refresh`](Self::refresh), and the view shows the edit. Give it an
  /// image of its own (a copy), not one that other work needs to keep as it is.
  #[uniffi::constructor]
  pub fn new(p_image: Arc<abra_uniffi::Image>) -> Arc<Self> {
    Self::over(PreviewSource::new(p_image))
  }

  /// A view of `canvas`, composed by the engine on its own thread each time the canvas has changed: build the canvas once, change its
  /// layers and canvases (a rotation, a position), and call [`refresh`](Self::refresh), and nothing is composed on the caller's thread.
  /// The canvas is shared, not copied.
  #[uniffi::constructor]
  pub fn from_canvas(p_canvas: Arc<abra_uniffi::Canvas>) -> Arc<Self> {
    Self::over(PreviewSource::of_canvas(p_canvas))
  }

  /// Draws the view again now if the image it shares was edited since it was drawn. The view looks at the version of that image
  /// on every frame while it is busy and sleeps while it is not, so this is how an edit made in place wakes it.
  pub fn refresh(&self) {
    self.source.refresh();
  }

  /// Shows only this part of the photo, as a rectangle of the whole photo from 0 to 1. `0, 0, 1, 1` is all of it. The
  /// rectangle may reach past the photo (zooming out, or panning off an edge); that part is black. The view fills its
  /// whole native view and follows the rectangle at once, with no gliding, so a finger drag keeps the photo under it.
  pub fn set_viewport(&self, p_x: f32, p_y: f32, p_width: f32, p_height: f32) {
    self.image.jump_viewport(Viewport { x: p_x, y: p_y, width: p_width, height: p_height });
  }

  /// Moves the view to this part of the photo in `p_milliseconds`, easing in and out, with a frame for every tick on the way. The
  /// part is a rectangle of the whole photo from 0 to 1, like [`set_viewport`](Self::set_viewport), which cancels the move.
  pub fn tween_viewport(&self, p_x: f32, p_y: f32, p_width: f32, p_height: f32, p_milliseconds: u32) {
    self.image.tween_viewport(
      Viewport { x: p_x, y: p_y, width: p_width, height: p_height },
      TweenOptions::default().with_duration(std::time::Duration::from_millis(p_milliseconds as u64)),
    );
  }

  /// Turns the photo to this many radians clockwise about the middle of what the view shows, at once. 0 is upright. The GPU draws the
  /// turn; without a GPU (or when it cannot draw the photo) the CPU turns it, smoothly, on black where the turn leaves the photo.
  pub fn set_rotation(&self, p_radians: f32) {
    self.image.set_rotation(p_radians);
  }

  /// Turns the photo to this many radians in `p_milliseconds`, easing in and out, with a frame for every tick on the way. It runs
  /// beside a [`tween_viewport`](Self::tween_viewport), and [`set_rotation`](Self::set_rotation) cancels it.
  pub fn tween_rotation(&self, p_radians: f32, p_milliseconds: u32) {
    self.image.tween_rotation(
      p_radians,
      TweenOptions::default().with_duration(std::time::Duration::from_millis(p_milliseconds as u64)),
    );
  }

  /// Whether a completed photo frame has reached the mounted surface. Placeholder frames do not count.
  pub fn has_frame(&self) -> bool {
    self.presented.load(Ordering::SeqCst)
  }
}

impl ImagePreview {
  /// What something laid over this view needs to stay on the same part of the photo: the part shown, how far it is turned, and the size of the
  /// picture, each sent when it changes (a glide or a tween sends every frame of it).
  pub(crate) fn looks(&self) -> Subject<((Viewport, f32), (u32, u32))> {
    self.image.viewports().combine_latest(self.image.rotations()).combine_latest(&self.source.photo)
  }

  /// Calls `p_apply` with the camera of this view now, once it has one, and again each time the view moves, turns or shows a picture of another size
  /// (a glide or a tween sends every frame of it). Something laid over the view uses it to stay on the same part of the photo; it follows until the
  /// subscription is dropped.
  pub(crate) fn follow_camera(&self, p_apply: impl Fn(Camera) + Send + Sync + 'static) -> Subscription {
    let apply = move |viewport: &Viewport, angle: f32, photo: (u32, u32)| {
      if let Some(camera) = Camera::of(*viewport, angle, photo) {
        p_apply(camera);
      }
    };
    // Something that is already showing the photo has a camera to start from; the stream only sends what changes after this.
    apply(&self.image.viewport(), self.image.rotation(), self.source.photo.peek());
    self.looks().subscribe(move |((viewport, angle), photo)| apply(viewport, *angle, *photo))
  }

  /// The view over a source, made to follow it and to say when a frame has been drawn.
  fn over(p_source: PreviewSource) -> Arc<Self> {
    let source = Arc::new(p_source);
    let image = Image::new().track(source.as_ref());
    let presented = Arc::new(AtomicBool::new(false));
    image.subject::<Drawn>().subscribe({
      let presented = Arc::clone(&presented);
      move |_| presented.store(true, Ordering::SeqCst)
    });
    Arc::new(Self {
      image,
      presented,
      source,
    })
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
      assert!(start.elapsed() < Duration::from_secs(5), "no matching frame arrived; drawn so far: {:?}", p_surface.0.lock().unwrap());
      std::thread::sleep(Duration::from_millis(5));
    }
  }

  /// A view of a gray 4 x 4 photo, and the photo it shares, which an edit changes.
  fn view_of(p_id: i32) -> (Arc<ImagePreview>, Arc<abra_uniffi::Image>) {
    let image = Arc::new(abra_uniffi::Image::from_core(AbraPixels::new_from_pixels(
      4, 4, [100u8, 100, 100, 255].repeat(16), Channels::RGBA,
    )));
    let view = ImagePreview::new(Arc::clone(&image));
    view.mount(p_id);
    (view, image)
  }

  fn view(p_id: i32) -> Arc<ImagePreview> {
    view_of(p_id).0
  }

  /// Edits the photo the view shares so it is `p_with`, and tells the view.
  fn replace(p_view: &ImagePreview, p_image: &abra_uniffi::Image, p_with: AbraPixels) {
    p_image.edit(|photo| *photo = p_with);
    p_view.refresh();
  }

  #[test]
  fn the_photo_is_shown_as_it_is_and_an_edit_of_it_changes_what_is_shown() {
    // The view appears after the component is mounted on it, which is how a React Native view mounts.
    let (view, photo) = view_of(7001);
    let original = photo.core();
    assert!(!view.has_frame());
    let surface = surface_of(7001);
    assert_eq!(newest(&surface, |_| true), 100, "the photo as it is");
    let start = Instant::now();
    while !view.has_frame() {
      assert!(start.elapsed() < Duration::from_secs(5), "presentation was not acknowledged");
      std::thread::sleep(Duration::from_millis(5));
    }

    let made = AbraPixels::new_from_color(2, 2, abra::abra_core::Color::from_rgba(150, 150, 150, 255));
    replace(&view, &photo, made);
    assert_eq!(newest(&surface, |pixel| pixel == 150), 150);

    replace(&view, &photo, original);
    assert_eq!(newest(&surface, |pixel| pixel == 100), 100);
  }

  #[test]
  fn a_photo_edited_in_place_is_drawn_again_when_the_view_is_told_to_refresh_and_not_before() {
    let image = Arc::new(abra_uniffi::Image::from_core(AbraPixels::new_from_pixels(4, 4, [100u8, 100, 100, 255].repeat(16), Channels::RGBA)));
    let view = ImagePreview::new(Arc::clone(&image));
    view.mount(7010);
    let surface = surface_of(7010);
    assert_eq!(newest(&surface, |_| true), 100);
    // Nothing sent a message, and the view is asleep, so it does not know yet.
    image.edit(|photo| photo.clear_color(abra::abra_core::Color::from_rgba(180, 180, 180, 255)));
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(newest(&surface, |_| true), 100, "an idle view draws nothing by itself");
    view.refresh();
    assert_eq!(newest(&surface, |pixel| pixel == 180), 180, "told to look again, it sees the edit");
  }

  #[test]
  fn dropping_the_view_lets_go_of_the_photo_it_shared() {
    let (view, photo) = view_of(7020);
    let surface = surface_of(7020);
    assert_eq!(newest(&surface, |_| true), 100);
    let weak = Arc::downgrade(&photo);
    drop(view);
    drop(photo);
    let start = Instant::now();
    while weak.upgrade().is_some() {
      assert!(start.elapsed() < Duration::from_secs(5), "the photo is still held after the view was dropped");
      std::thread::sleep(Duration::from_millis(5));
    }
  }

  #[test]
  fn a_new_viewport_shows_that_part_of_the_photo() {
    let pixels = (0..16).flat_map(|at| if at % 4 < 2 { [100u8, 100, 100, 255] } else { [200, 100, 100, 255] }).collect::<Vec<_>>();
    let image = Arc::new(abra_uniffi::Image::from_core(AbraPixels::new_from_pixels(4, 4, pixels, Channels::RGBA)));
    let view = ImagePreview::new(image);
    view.mount(7006);
    let surface = surface_of(7006);
    assert_eq!(newest(&surface, |_| true), 100, "the whole photo starts with its left edge");
    view.set_viewport(0.5, 0.0, 0.5, 1.0);
    // The left half is 100 and the right half 200, and a view magnified between the two pixels is smooth, so the first pixel is
    // most of the way to 200 and no longer 100.
    assert!(newest(&surface, |pixel| pixel >= 170) >= 170, "the right half fills the view");
    view.set_viewport(0.0, 0.0, 1.0, 1.0);
    assert_eq!(newest(&surface, |pixel| pixel == 100), 100, "the whole photo comes back");
  }

  #[test]
  fn a_viewport_past_the_photo_shows_black_there() {
    let view = view(7007);
    let surface = surface_of(7007);
    assert_eq!(newest(&surface, |_| true), 100);
    // Zoomed out to twice the photo's width, with the photo on the right: the left half of the view is past the photo.
    view.set_viewport(-1.0, 0.0, 2.0, 1.0);
    assert_eq!(newest(&surface, |pixel| pixel == 0), 0, "the left edge is black");
    view.set_viewport(0.0, 0.0, 1.0, 1.0);
    assert_eq!(newest(&surface, |pixel| pixel == 100), 100, "the photo fills the view again");
  }

  #[test]
  fn an_image_put_in_place_of_the_photo_is_shown_until_the_photo_is_put_back() {
    let (view, photo) = view_of(7004);
    let original = photo.core();
    let surface = surface_of(7004);
    assert_eq!(newest(&surface, |_| true), 100);
    // A tool answers with a new image of its own.
    let made = AbraPixels::new_from_color(2, 2, abra::abra_core::Color::from_rgba(30, 60, 90, 255));
    replace(&view, &photo, made);
    assert_eq!(newest(&surface, |pixel| pixel == 30), 30);
    replace(&view, &photo, original);
    assert_eq!(newest(&surface, |pixel| pixel == 100), 100);
  }

  #[test]
  fn a_tool_s_image_that_is_still_a_recipe_is_drawn_by_the_gpu_and_matches_its_pixels() {
    use abra_uniffi::{Color, ColorStop, Gradient, blend, radial_gradient};

    let (view, photo) = view_of(7005);
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
    replace(&view, &photo, source.core());
    assert_eq!(newest(&surface, |pixel| pixel == 50), 50);
  }

  #[test]
  fn the_photo_comes_back_after_an_edit_that_is_still_a_recipe_is_undone() {
    use abra_uniffi::{Color, ColorStop, Gradient, blend, radial_gradient};

    let (view, photo) = view_of(7011);
    let original = photo.core();
    let surface = surface_of(7011);
    assert_eq!(newest(&surface, |_| true), 100);
    let source = Arc::new(abra_uniffi::Image::from_core(original.clone()));
    let red = Arc::new(Color::from_rgba(50, 0, 0, 255));
    let gradient = Arc::new(Gradient::new(vec![Arc::new(ColorStop::new(red.clone(), 0.0)), Arc::new(ColorStop::new(red, 1.0))]));
    blend(radial_gradient(4, 4, gradient, 2.0, 2.0, 3.0, 3.0)).apply(source.clone());
    replace(&view, &photo, source.core());
    assert_eq!(newest(&surface, |pixel| pixel == 50), 50);
    // Undo: the photo is put back as a copy of the original, which has the original's version.
    replace(&view, &photo, original.clone());
    assert_eq!(newest(&surface, |pixel| pixel == 100), 100, "the undone edit is still on screen");
  }

  #[test]
  fn without_a_gpu_the_photo_is_seen_through_the_viewport_on_black() {
    // Red is the column and green the row, times 40.
    let pixels = (0..4u32).flat_map(|row| (0..4u32).flat_map(move |column| [(column * 40) as u8, (row * 40) as u8, 0, 255])).collect::<Vec<u8>>();
    let photo = AbraPixels::new_from_pixels(4, 4, pixels, Channels::RGBA);
    let pixel = |frame: &Frame, x: u32, y: u32| -> [u8; 3] {
      let at = ((y * frame.width + x) * 4) as usize;
      frame.pixels[at..at + 3].try_into().unwrap()
    };
    let whole = view_on_the_cpu(&photo, &Viewport::default(), 0.0, (4, 4));
    assert_eq!(&whole.pixels[..], photo.rgba(), "the whole photo at its own size is the photo");
    let right = view_on_the_cpu(&photo, &Viewport { x: 0.5, y: 0.0, width: 0.5, height: 1.0 }, 0.0, (2, 4));
    assert_eq!(pixel(&right, 0, 0), [80, 0, 0], "the right half fills the view");
    let zoomed_out = view_on_the_cpu(&photo, &Viewport { x: -1.0, y: 0.0, width: 2.0, height: 1.0 }, 0.0, (8, 4));
    assert_eq!(pixel(&zoomed_out, 0, 0), [0, 0, 0], "past the photo is black");
    assert_eq!(pixel(&zoomed_out, 7, 3), [120, 120, 0], "and the photo is on the right");
    let turned = view_on_the_cpu(&photo, &Viewport::default(), std::f32::consts::FRAC_PI_2, (4, 4));
    assert_eq!(pixel(&turned, 3, 0), pixel(&whole, 0, 0), "turned a quarter clockwise, the top left corner is at the top right");
  }

  #[test]
  fn dropping_the_view_takes_it_off_the_native_view() {
    let surface = surface_of(7003);
    drop(view(7003));
    let drawn = surface.0.lock().unwrap().len();
    let (view, photo) = view_of(7003);
    let made = AbraPixels::new_from_color(2, 2, abra::abra_core::Color::from_rgba(150, 150, 150, 255));
    replace(&view, &photo, made);
    assert_eq!(newest(&surface, |pixel| pixel == 150), 150);
    assert!(surface.0.lock().unwrap().len() > drawn);
  }

  #[test]
  fn a_canvas_is_composed_by_the_view_and_drawn_again_when_it_changes_and_the_view_is_told_to_refresh() {
    let canvas = Arc::new(abra_uniffi::Canvas::new_blank("Test".to_string(), 4, 4));
    let gray = |value: u8| {
      Arc::new(abra_uniffi::Image::from_core(AbraPixels::new_from_color(4, 4, abra::abra_core::Color::from_rgba(value, value, value, 255))))
    };
    canvas.add_layer_from_image("gray".to_string(), gray(100), None);
    let view = ImagePreview::from_canvas(Arc::clone(&canvas));
    view.mount(7030);
    let surface = surface_of(7030);
    assert_eq!(newest(&surface, |_| true), 100, "the canvas as it is composed");

    canvas.add_layer_from_image("lighter".to_string(), gray(180), None);
    view.refresh();
    assert_eq!(newest(&surface, |pixel| pixel == 180), 180, "a layer added to the canvas is composed and drawn");
  }
}
