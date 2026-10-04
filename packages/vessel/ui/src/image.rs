use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use vessel_api::prelude::*;
use vessel_macros::Component;

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
  fn pictures(&self, p_size: &BehaviorSubject<(u32, u32)>) -> Subject<Picture>;
}

impl<T: Trackable> Trackable for Arc<T> {
  fn pictures(&self, p_size: &BehaviorSubject<(u32, u32)>) -> Subject<Picture> {
    self.as_ref().pictures(p_size)
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
  picture: Arc<Mutex<Option<Arc<Frame>>>>,
  /// Counts the pictures set, so the drawing listener redraws for each one without comparing pixels.
  revision: BehaviorSubject<u64>,
  fit: BehaviorSubject<Fit>,
  /// Whether the component currently shows a GPU picture instead of pixels.
  on_gpu: Arc<AtomicBool>,
  /// The streams of pictures this follows, and the listening to them, kept for as long as the image lives.
  sources: Arc<Mutex<Vec<(Subject<Picture>, Subscription)>>>,
}

impl Image {
  /// An image with nothing to show yet, fitted with [`Fit::Contain`].
  pub fn new() -> Self {
    let picture = Arc::new(Mutex::new(None::<Arc<Frame>>));
    let revision = BehaviorSubject::new(0u64);
    let fit = BehaviorSubject::new(Fit::default());

    let component = Component::new("image");
    component.subject::<Canvas>().subscribe({
      let (picture, revision, fit) = (Arc::clone(&picture), revision.clone(), fit.clone());
      move |canvas| {
        // Reading these is what makes the image depend on them.
        revision.value();
        let fit = fit.value();
        let picture = picture.lock().unwrap();
        let Some(frame) = picture.as_ref() else { return };
        let (left, top, width, height) = canvas.content_rect();
        if (frame.width, frame.height) == (width, height) {
          canvas.draw_pixels(left, top, width, height, &frame.pixels);
        } else if let Some((x, y, width, height, pixels)) = fitted(frame, width, height, fit) {
          canvas.draw_pixels(left + x, top + y, width, height, &pixels);
        }
      }
    });

    Self {
      component,
      picture,
      revision,
      fit,
      on_gpu: Arc::new(AtomicBool::new(false)),
      sources: Arc::default(),
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
    self.set_source(&p_source.pictures(self.size()));
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
      pixels: p_rgba,
    })));
  }

  /// Shows `p_picture`: pixels are drawn in the image's box, and a GPU picture replaces everything (see
  /// [`show_gpu`](Self::show_gpu)).
  pub fn show(&self, p_picture: &Picture) {
    match p_picture {
      Picture::Pixels(frame) => {
        *self.picture.lock().unwrap() = Some(Arc::clone(frame));
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
  pub fn show_gpu(&self, p_frame: Arc<dyn GpuFrame>) {
    self.on_gpu.store(true, Ordering::SeqCst);
    self.component.subject::<GpuPicture>().next(GpuPicture(Some(p_frame)));
  }

  /// Shows nothing.
  pub fn clear(&self) {
    *self.picture.lock().unwrap() = None;
    self.back_to_pixels();
    self.revision.next(self.revision.value() + 1);
  }

  /// Stops showing a GPU picture, if one was shown, so the pixels are drawn again.
  fn back_to_pixels(&self) {
    if self.on_gpu.swap(false, Ordering::SeqCst) {
      self.component.subject::<GpuPicture>().next(GpuPicture(None));
    }
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
pub(crate) fn fitted(p_frame: &Frame, p_width: u32, p_height: u32, p_fit: Fit) -> Option<(i32, i32, u32, u32, Vec<u8>)> {
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
  let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
  for y in 0..height {
    let from_y =
      (source.1 + (y as f32 + 0.5) * source.3 / height as f32).floor().clamp(0.0, source_height - 1.0) as usize;
    for x in 0..width {
      let from_x =
        (source.0 + (x as f32 + 0.5) * source.2 / width as f32).floor().clamp(0.0, source_width - 1.0) as usize;
      let at = (from_y * p_frame.width as usize + from_x) * 4;
      pixels.extend_from_slice(&p_frame.pixels[at..at + 4]);
    }
  }
  Some((((p_width - width) / 2) as i32, ((p_height - height) / 2) as i32, width, height, pixels))
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;

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

  struct Picture;

  impl GpuFrame for Picture {
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

    image.show_gpu(Arc::new(Picture));
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
      pixels: red_then_blue(),
    })));
    assert_eq!(pixel(&render(&image), 1, 0), [0, 0, 255, 255]);
  }
}
