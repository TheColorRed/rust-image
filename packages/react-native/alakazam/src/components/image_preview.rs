//! The photo being edited: a component that React creates and the Vessel engine draws.
//!
//! `LiveView` keeps a Vessel `Image` and gives it something to follow: the pictures made from the messages React sends.
//! The chain of effects is the state those messages add up to, and a picture is that chain rendered over the photo. On a
//! GPU the picture is a texture that goes to the screen without being copied to the CPU. Everything else (when to draw, the
//! native view, the GPU target) is the engine's.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, Ordering},
};

use abra::prelude::ImageExt;
use vessel::prelude::*;


pub(crate) struct PreviewSource {
  /// The image that is shown. It is shared: whoever holds the other end edits it, and the picture follows.
  image: Arc<abra_uniffi::Image>,
  /// The renderer that makes the pictures, once there is one, so it can be woken when the image is edited.
  renderer: Mutex<Option<Offscreen>>,
}

impl PreviewSource {
  pub(crate) fn new(image: Arc<abra_uniffi::Image>) -> Self {
    Self { image, renderer: Mutex::new(None) }
  }

  /// Draws again at once if what is shown has changed since it was drawn, such as a photo that was edited in place.
  fn refresh(&self) {
    if let Some(renderer) = self.renderer.lock().unwrap().as_ref() {
      renderer.wake();
    }
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
struct OnTheGpu {
  /// Names the pixels: which version of the image and how big.
  key: (u64, (u32, u32)),
  _root: abra::prelude::Image,
  photo: gpu::TiledRenderer,
}

impl Trackable for PreviewSource {
  fn pictures(&self, size: &BehaviorSubject<(u32, u32)>, viewport: &BehaviorSubject<Viewport>) -> Subject<Picture> {
    let source = Arc::clone(&self.image);
    let changes = size.combine_latest(viewport);
    // The renderer draws again whenever the image is a different one (see `ImagePreview::refresh`), and keeps drawing while tiles
    // are still on their way; between them it sleeps.
    let version = {
      let source = Arc::clone(&self.image);
      move || source.core().version()
    };
    let busy = Arc::new(AtomicBool::new(false));
    // The photos on the GPU, the one drawn last at the end. An edit that is a smaller picture (a mood, say) and then undone brings
    // the full-size photo back, which must not be copied up and built again.
    let mut held: Vec<OnTheGpu> = Vec::new();
    let renderer = Offscreen::tracking(&changes, version, { let busy = Arc::clone(&busy); move || busy.load(Ordering::SeqCst) }, move |(size, viewport)| {
      busy.store(false, Ordering::SeqCst);
      if size.0 == 0 || size.1 == 0 { return None }
      // An image that is still a recipe is the pixels it started from and the passes over them, so the GPU keeps those pixels
      // and a drag only changes the passes.
      let image = source.core();
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
          held.push(OnTheGpu { key, _root: root.clone(), photo });
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
      // No GPU, or it could not draw this: the pixels the tool made (or the photo), with the part in view cut out and set on
      // black, which is what the GPU draws where the view reaches past the photo.
      let whole = image;
      let frame = frame_of(&whole);
      let whole = abra::prelude::Image::new_from_pixels(frame.width, frame.height, frame.pixels.to_vec(), abra::abra_core::Channels::RGBA);
      let mut pixels = [0u8, 0, 0, 255].repeat(size.0 as usize * size.1 as usize);
      let (left, top) = (viewport.x.max(0.0), viewport.y.max(0.0));
      let (right, bottom) = ((viewport.x + viewport.width).min(1.0), (viewport.y + viewport.height).min(1.0));
      if right > left && bottom > top {
        let to_box = |at: f32, from: f32, extent: f32, room: u32| ((at - from) / extent * room as f32).round().clamp(0.0, room as f32) as u32;
        let x = to_box(left, viewport.x, viewport.width, size.0).min(size.0 - 1);
        let y = to_box(top, viewport.y, viewport.height, size.1).min(size.1 - 1);
        let width = to_box(right, viewport.x, viewport.width, size.0).saturating_sub(x).max(1);
        let height = to_box(bottom, viewport.y, viewport.height, size.1).saturating_sub(y).max(1);
        let mut part = fitted(&whole, &Viewport { x: left, y: top, width: right - left, height: bottom - top }, u32::MAX, u32::MAX);
        if part.dimensions::<u32>() != (width, height) {
          abra::transform::prelude::resize(abra::abra_core::ResizeTarget::Exact(abra::abra_core::Size::new(width as f32, height as f32)))
            .apply(&mut part);
        }
        let (part_width, part_height) = part.dimensions::<u32>();
        let part_rgba = part.rgba();
        let (columns, rows) = (part_width.min(size.0 - x) as usize, part_height.min(size.1 - y));
        // Only the colors are copied: the view is opaque, and resizing fades the alpha at the edges of a small crop.
        for row in 0..rows {
          let source = row as usize * part_width as usize * 4;
          let target = ((y + row) as usize * size.0 as usize + x as usize) * 4;
          for column in 0..columns {
            pixels[target + column * 4..target + column * 4 + 3].copy_from_slice(&part_rgba[source + column * 4..source + column * 4 + 3]);
          }
        }
      }
      Some(RenderedFrame::Pixels(Arc::new(Frame { width: size.0, height: size.1, pixels: pixels.into() })))
    });
    *self.renderer.lock().unwrap() = Some(renderer.clone());
    renderer.map(|frame| match frame {
      RenderedFrame::Pixels(frame) => Picture::Pixels(Arc::clone(frame)),
      RenderedFrame::Gpu(frame) => Picture::Gpu(Arc::clone(frame)),
    })
  }
}

/// The part of the image inside `region` shrunk to fit within `width` x `height`, or as it is when it already fits. The
/// default region is the whole image. The region is cut out of the image as it is, so a small region of a large photo
/// keeps the photo's own pixels.
pub(crate) fn fitted(image: &abra::prelude::Image, region: &Viewport, width: u32, height: u32) -> abra::prelude::Image {
  let (full_width, full_height) = image.dimensions::<u32>();
  let (x, y, cut_width, cut_height) = region.in_pixels(full_width, full_height);
  let mut shrunk = image.clone();
  if (x, y, cut_width, cut_height) != (0, 0, full_width, full_height) {
    abra::transform::prelude::crop(x, y, cut_width, cut_height).apply(&mut shrunk);
  }
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
  source: Arc<PreviewSource>,
}

#[uniffi::export]
impl ImagePreview {
  /// A view of `image`, which it shares: edit the image and call [`refresh`](Self::refresh), and the view shows the edit. Give it an
  /// image of its own (a copy), not one that other work needs to keep as it is.
  #[uniffi::constructor]
  pub fn new(p_image: Arc<abra_uniffi::Image>) -> Arc<Self> {
    let source = Arc::new(PreviewSource::new(p_image));
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

  /// A 40x20 photo whose red is the column and green is the row, so every pixel tells where it came from.
  fn gradient_photo() -> AbraPixels {
    let pixels = (0..20u32).flat_map(|row| (0..40u32).flat_map(move |column| [column as u8 * 6, row as u8 * 12, 0, 255]));
    AbraPixels::new_from_pixels(40, 20, pixels.collect::<Vec<u8>>(), Channels::RGBA)
  }

  #[test]
  fn the_default_region_gives_the_whole_photo_as_before() {
    let photo = gradient_photo();
    let whole = fitted(&photo, &Viewport::default(), 40, 20);
    assert_eq!(whole.rgba(), photo.rgba());
    let smaller = fitted(&photo, &Viewport::default(), 20, 10);
    let mut expected = photo.clone();
    abra::transform::prelude::resize(abra::abra_core::ResizeTarget::Fit(abra::abra_core::Size::new(20.0, 10.0))).apply(&mut expected);
    assert_eq!(smaller.rgba(), expected.rgba());
  }

  #[test]
  fn a_region_is_cut_from_the_photo_before_it_is_shrunk() {
    let photo = gradient_photo();
    // The right half, touching the edge: columns 20..40, all rows. Not square and not the photo's shape.
    let right = fitted(&photo, &Viewport { x: 0.5, y: 0.0, width: 0.5, height: 1.0 }, 100, 100);
    assert_eq!(right.dimensions::<u32>(), (20, 20));
    assert_eq!(&right.rgba()[..4], &[20 * 6, 0, 0, 255]);
    // A quarter in the middle, shown at its own size, is a plain crop with no resampling.
    let middle = fitted(&photo, &Viewport { x: 0.25, y: 0.25, width: 0.5, height: 0.5 }, 100, 100);
    assert_eq!(middle.dimensions::<u32>(), (20, 10));
    let mut expected = photo.clone();
    abra::transform::prelude::crop(10, 5, 20, 10).apply(&mut expected);
    assert_eq!(middle.rgba(), expected.rgba());
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
}
