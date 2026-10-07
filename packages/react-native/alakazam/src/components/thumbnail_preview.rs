use std::sync::{Arc, Mutex, OnceLock};

use abra::{
  drawing::prelude::fill,
  prelude::{Area, Color, Gradient, Path},
};
use vessel::prelude::*;

use crate::{AbraImage, consts::THUMBNAIL_GRADIENT_OPACITY};

#[derive(uniffi::Object, Component)]
pub struct ThumbnailPreview {
  container: Container,
  preview_size: (u32, u32),
}

struct ThumbnailSource(Arc<AbraImage>);

impl Trackable for ThumbnailSource {
  fn pictures(&self, p_size: &BehaviorSubject<(u32, u32)>) -> Subject<Picture> {
    self.0.thumbnail_pictures(p_size)
  }
}

fn background(p_image: &Arc<AbraImage>, p_width: u32, p_height: u32) -> Image {
  // Seed a frame before attaching the reactive source.
  let pixels = p_image.preview(p_width, p_height);
  Image::new()
    .with_fit(Fit::Cover)
    .with_pixels(pixels.width, pixels.height, pixels.data)
    .track(&ThumbnailSource(Arc::clone(p_image)))
}

fn gradient_overlay(p_width: u32, p_height: u32) -> Image {
  let width = p_width.max(1);
  let height = p_height.max(1);
  // Keep one size cached, bounding memory even when host density or card dimensions change.
  static LAST: OnceLock<Mutex<Option<Arc<Frame>>>> = OnceLock::new();
  let mut cached = LAST.get_or_init(|| Mutex::new(None)).lock().unwrap();
  if cached.as_ref().is_none_or(|frame| (frame.width, frame.height) != (width, height)) {
    let gradient = Gradient::evenly(vec![Color::transparent(), Color::black().set_alpha(THUMBNAIL_GRADIENT_OPACITY)])
      .with_direction(Path::line((0, height as f32 * 0.20), (0, height)));
    let pixels = fill(Area::rect((0, 0), (width, height)), &gradient).to_image();
    *cached = Some(Arc::new(Frame {
      width,
      height,
      pixels: pixels.rgba().to_vec().into(),
    }));
  }
  let image = Image::new().with_fit(Fit::Stretch);
  image.show(&Picture::Pixels(Arc::clone(cached.as_ref().unwrap())));
  image
}

#[uniffi::export]
impl ThumbnailPreview {
  /// A view of `image`.
  #[uniffi::constructor]
  pub fn new(p_image: Arc<AbraImage>, p_label: String, p_width: u32, p_height: u32) -> Arc<Self> {
    let image = background(&p_image, p_width, p_height);
    let gradient = gradient_overlay(p_width, p_height);

    let text = Label::new(p_label)
      .with_color([255, 255, 255, 255])
      .with_vertical_align(VerticalAlign::Bottom)
      .with_text_align(TextAlign::Left)
      .with_padding(Edges::new(0, 0, 0, 10))
      .with_radius(17)
      .with_background(&gradient);

    let container = Container::new()
      .with_width(p_width)
      .with_height(p_height)
      .with_background(&image)
      .with_radius(20)
      .with_border_color([255, 255, 255, 255])
      .with_border_width(3);

    container.add(&text);

    Arc::new(Self {
      container,
      preview_size: (p_width, p_height),
    })
  }

  /// Replaces the preview image without remounting the thumbnail or changing its label.
  pub fn set_image(&self, p_image: Arc<AbraImage>) {
    let image = background(&p_image, self.preview_size.0, self.preview_size.1);
    self.container.set_background(image);
  }
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;
  use abra::prelude::{Color, Image as AbraPixels};

  #[test]
  fn gradient_pixels_are_black_with_increasing_alpha_and_no_color_tint() {
    let overlay = gradient_overlay(112, 80);
    overlay.size().next((112, 80));
    let mut component = overlay.component().clone();
    let frame = component.render(Duration::ZERO);
    let last_alpha = (THUMBNAIL_GRADIENT_OPACITY * 255.0) as u8;
    let mut previous_alpha = 0;
    for y in 0..80 {
      let sample = pixel(&frame, 56, y);
      assert_eq!(&sample[..3], &[0, 0, 0], "gradient acquired a color at row {y}");
      assert!(sample[3] >= previous_alpha, "gradient alpha decreased at row {y}");
      previous_alpha = sample[3];
    }
    assert_eq!(pixel(&frame, 56, 0)[3], 0, "gradient must start fully transparent");
    assert_eq!(pixel(&frame, 56, 79)[3], last_alpha, "gradient must reach the requested black opacity");
  }

  #[test]
  fn black_gradient_darkens_all_photo_channels_without_adding_a_tint() {
    for color in [[240, 120, 60, 255], [0, 200, 80, 255], [60, 80, 240, 255]] {
      let preview = ThumbnailPreview::new(source(color), "".into(), 112, 80);
      preview.size().next((112, 80));
      let frame = rendered(&preview, color);
      for y in [8, 40, 70] {
        let actual = pixel(&frame, 56, y);
        let alpha = ((THUMBNAIL_GRADIENT_OPACITY * 255.0) as u8 as f32 * (y - 3) as f32 / 73.0) as u8;
        for channel in 0..3 {
          let expected = (color[channel] as f32 * (1.0 - alpha as f32 / 255.0)).round() as u8;
          assert!(actual[channel].abs_diff(expected) <= 3, "color={color:?}, row={y}, actual={actual:?}");
        }
        assert_eq!(actual[3], 255);
      }
    }
  }

  #[test]
  fn host_fonts_reach_labels_without_overriding_explicit_component_sizes() {
    let preview = ThumbnailPreview::new(source([200, 0, 0, 255]), "Warm".into(), 112, 80);
    let inherited = Label::new("Inherited");
    preview.add(&inherited);
    assert!(preview.set_host_font(None, 35));
    assert_eq!(inherited.font().size, 35);
    inherited.set_font_size(Units::Pixels(12));
    assert!(preview.set_host_font(None, 42));
    assert_eq!(inherited.font().size, 12);
    assert!(!preview.set_host_font(Some("missing-vessel-font.ttf".into()), 20));
    inherited.inherit_font();
    assert_eq!(inherited.font().size, 42, "a failed font update leaves the previous default intact");
  }

  fn source(p_color: [u8; 4]) -> Arc<AbraImage> {
    AbraImage::from_image(AbraPixels::new_from_color(
      16,
      32,
      Color::from_rgba(p_color[0], p_color[1], p_color[2], p_color[3]),
    ))
  }

  fn pixel(p_frame: &Frame, p_x: u32, p_y: u32) -> [u8; 4] {
    let at = ((p_y * p_frame.width + p_x) * 4) as usize;
    p_frame.pixels[at..at + 4].try_into().unwrap()
  }

  fn rendered(p_preview: &ThumbnailPreview, p_color: [u8; 4]) -> Frame {
    let frame = p_preview.component().clone().render(Duration::ZERO);
    let actual = pixel(&frame, 56, 12);
    for channel in 0..3 {
      if p_color[channel] == 0 {
        assert_eq!(actual[channel], 0);
      } else {
        assert!(actual[channel] > p_color[channel] / 2 && actual[channel] < p_color[channel]);
      }
    }
    assert_eq!(actual[3], 255);
    frame
  }

  #[test]
  fn landscape_thumbnail_covers_its_box_and_retains_a_dropped_source() {
    let red = [200, 0, 0, 255];
    let image = source(red);
    let preview = ThumbnailPreview::new(Arc::clone(&image), "Warm".into(), 112, 80);
    drop(image);
    preview.size().next((112, 80));
    let frame = rendered(&preview, red);
    assert_eq!((frame.width, frame.height), (112, 80));
    assert_eq!(pixel(&frame, 100, 12), pixel(&frame, 56, 12), "cover must not leave transparent side bands");
    assert_eq!(pixel(&frame, 56, 0), [255; 4], "the native border remains visible");
    assert_eq!(pixel(&frame, 0, 0), [0; 4], "the outside of the rounded corner stays transparent");
    for (x, y) in [(6, 6), (105, 6), (6, 73), (105, 73)] {
      assert_eq!(pixel(&frame, x, y), [255; 4], "the border must follow every rounded corner");
    }
    assert!(
      frame
        .pixels
        .chunks_exact(4 * 112)
        .skip(40)
        .take(30)
        .any(|row| row[20 * 4..90 * 4].chunks_exact(4).any(|pixel| pixel == [255; 4])),
      "the native label is overlaid on the background"
    );
  }

  #[test]
  fn replacing_the_source_updates_the_background_and_keeps_the_component() {
    let red = [200, 0, 0, 255];
    let blue = [0, 0, 200, 255];
    let preview = ThumbnailPreview::new(source(red), "Midnight".into(), 112, 80);
    preview.size().next((112, 80));
    rendered(&preview, red);
    preview.set_image(source(blue));
    let frame = rendered(&preview, blue);
    assert_eq!(pixel(&frame, 100, 12), pixel(&frame, 56, 12));
    assert_eq!(pixel(&frame, 56, 0), [255; 4]);
    preview.size().next((224, 160));
    let frame = rendered(&preview, blue);
    assert_eq!((frame.width, frame.height), (224, 160));
  }

  #[test]
  fn the_background_still_tracks_edits_after_its_initial_frame() {
    let red = [200, 0, 0, 255];
    let blue = [0, 0, 200, 255];
    let image = source(red);
    let preview = ThumbnailPreview::new(Arc::clone(&image), "Warm".into(), 112, 80);
    preview.size().next((112, 80));
    rendered(&preview, red);
    image.with_image_mut(|pixels| {
      *pixels = AbraPixels::new_from_color(16, 32, Color::from_rgba(0, 0, 200, 255));
    });
    drop(image);
    rendered(&preview, blue);
  }

  #[test]
  fn gradients_are_present_on_first_frame_for_every_aspect_ratio_and_survive_source_replacement() {
    let white = [200, 200, 200, 255];
    for (width, height) in [(16, 32), (32, 16), (32, 32), (1, 1)] {
      let image =
        AbraImage::from_image(AbraPixels::new_from_color(width, height, Color::from_rgba(200, 200, 200, 255)));
      let preview = ThumbnailPreview::new(image, "".into(), 112, 80);
      preview.size().next((112, 80));
      let frame = rendered(&preview, white);
      let top = pixel(&frame, 56, 8)[0];
      let bottom = pixel(&frame, 56, 70)[0];
      let expected = |y: u32| (200.0 * (1.0 - THUMBNAIL_GRADIENT_OPACITY * y as f32 / 79.0)).round() as u8;
      assert!(top.abs_diff(expected(8)) <= 4, "{width}x{height}: top={top}");
      assert!(bottom.abs_diff(expected(70)) <= 4, "{width}x{height}: bottom={bottom}");
      preview.set_image(source([0, 0, 200, 255]));
      let frame = rendered(&preview, [0, 0, 200, 255]);
      assert!(pixel(&frame, 56, 8)[2].abs_diff(expected(8)) <= 4);
      assert!(pixel(&frame, 56, 70)[2].abs_diff(expected(70)) <= 4);
      preview.next(PointerEvent::Moved { x: 20.0, y: 20.0 });
      preview.next(PointerEvent::Button {
        button: PointerButton::Left,
        pressed: true,
      });
      preview.next(PointerEvent::Button {
        button: PointerButton::Left,
        pressed: false,
      });
    }
  }

  #[test]
  fn thumbnail_sources_publish_cpu_pixels_even_when_the_gpu_is_enabled() {
    let image = source([200, 0, 0, 255]);
    let size = BehaviorSubject::new((112, 80));
    let pictures = image.thumbnail_pictures(&size);
    let seen = Arc::new(std::sync::Mutex::new(0));
    let count = Arc::clone(&seen);
    let _subscription = pictures.subscribe(move |picture| {
      let Picture::Pixels(frame) = picture else { panic!("CPU-composed thumbnails must not allocate GPU frames") };
      assert!(frame.width > 0 && frame.height > 0);
      *count.lock().unwrap() += 1;
    });
    size.next((224, 160));
    image.with_image_mut(|pixels| {
      *pixels = AbraPixels::new_from_color(16, 32, Color::black());
    });
    assert!(*seen.lock().unwrap() >= 2);
  }
}
