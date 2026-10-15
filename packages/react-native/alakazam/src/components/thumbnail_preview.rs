use std::sync::{Arc, Mutex, OnceLock};

use abra::{
  drawing::prelude::fill,
  prelude::{Area, Color, Gradient, Path},
};
use vessel::prelude::*;

use crate::consts::THUMBNAIL_GRADIENT_OPACITY;

#[derive(uniffi::Object, Component)]
pub struct ThumbnailPreview {
  container: Container,
  /// The card's size, which the photo is cropped to fill.
  size: (u32, u32),
}

/// The thumbnail's photo, drawn by the same source the editor preview uses. That source draws the part of the photo the
/// viewport names across the whole card, so the viewport is the centered part of the photo with the card's shape.
fn background(p_image: &Arc<abra_uniffi::Image>, p_width: u32, p_height: u32) -> Image {
  let (photo_width, photo_height) = p_image.core().dimensions::<u32>();
  let photo_shape = photo_width.max(1) as f32 / photo_height.max(1) as f32;
  let card_shape = p_width.max(1) as f32 / p_height.max(1) as f32;
  let shown = if card_shape < photo_shape {
    let width = card_shape / photo_shape;
    Viewport { x: (1.0 - width) / 2.0, y: 0.0, width, height: 1.0 }
  } else {
    let height = photo_shape / card_shape;
    Viewport { x: 0.0, y: (1.0 - height) / 2.0, width: 1.0, height }
  };
  let image = Image::new().with_fit(Fit::Cover).track(&super::image_preview::PreviewSource::new(Arc::clone(p_image)));
  image.jump_viewport(shown);
  image
}

fn gradient_overlay(p_width: u32, p_height: u32) -> Image {
  let width = p_width.max(1);
  let height = p_height.max(1);
  // Keep one size cached, bounding memory even when host density or card dimensions change.
  static LAST: OnceLock<Mutex<Option<Arc<Frame>>>> = OnceLock::new();
  let mut cached = LAST.get_or_init(|| Mutex::new(None)).lock().unwrap();
  if cached.as_ref().is_none_or(|frame| (frame.width, frame.height) != (width, height)) {
    let gradient = Gradient::evenly(vec![
      Color::transparent(),
      Color::black().set_alpha(THUMBNAIL_GRADIENT_OPACITY),
    ])
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
  pub fn new(p_image: Arc<abra_uniffi::Image>, p_label: String, p_width: u32, p_height: u32) -> Arc<Self> {
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

    Arc::new(Self { container, size: (p_width, p_height) })
  }

  /// Replaces the preview image without remounting the thumbnail or changing its label.
  pub fn set_image(&self, p_image: Arc<abra_uniffi::Image>) {
    let image = background(&p_image, self.size.0, self.size.1);
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
    assert!(pixel(&frame, 56, 79)[3].abs_diff(last_alpha) <= 3, "gradient must reach the requested black opacity");
  }

  #[test]
  fn black_gradient_darkens_all_photo_channels_without_adding_a_tint() {
    for color in [[240, 120, 60, 255], [0, 200, 80, 255], [60, 80, 240, 255]] {
      let preview = ThumbnailPreview::new(source(color), "".into(), 112, 80);
      preview.size().next((112, 80));
      let frame = rendered(&preview, color);
      // The gradient starts 20% down the card and darkens linearly to the bottom edge.
      for y in [24, 40, 70] {
        let actual = pixel(&frame, 56, y);
        let alpha = THUMBNAIL_GRADIENT_OPACITY * ((y as f32 - 16.0) / 64.0).clamp(0.0, 1.0);
        for channel in 0..3 {
          let expected = (color[channel] as f32 * (1.0 - alpha)).round() as u8;
          assert!(actual[channel].abs_diff(expected) <= 12, "color={color:?}, row={y}, actual={actual:?}");
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

  fn source(p_color: [u8; 4]) -> Arc<abra_uniffi::Image> {
    Arc::new(abra_uniffi::Image::from_core(AbraPixels::new_from_color(
      16,
      32,
      Color::from_rgba(p_color[0], p_color[1], p_color[2], p_color[3]),
    )))
  }

  fn pixel(p_frame: &Frame, p_x: u32, p_y: u32) -> [u8; 4] {
    let at = ((p_y * p_frame.width + p_x) * 4) as usize;
    p_frame.pixels[at..at + 4].try_into().unwrap()
  }

  fn rendered(p_preview: &ThumbnailPreview, p_color: [u8; 4]) -> Frame {
    // The shared source draws the photo a moment after it is shown, so wait for its first frame.
    let start = std::time::Instant::now();
    let mut frame = p_preview.component().clone().render(Duration::ZERO);
    let ready = |frame: &Frame| {
      pixel(frame, 56, 40)[3] == 255 && (0..3).all(|c| p_color[c] == 0 || pixel(frame, 56, 40)[c] < p_color[c])
    };
    while !ready(&frame) && start.elapsed() < Duration::from_secs(5) {
      std::thread::sleep(Duration::from_millis(10));
      frame = p_preview.component().clone().render(Duration::ZERO);
    }
    // Row 40 is inside the gradient (it starts at row 16), so the photo is darkened but not black.
    let actual = pixel(&frame, 56, 40);
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
  fn a_wide_card_shows_the_middle_of_a_square_photo_and_does_not_squash_it() {
    // Red on the top third, green in the middle, blue on the bottom, so the rows show how much of the photo is in view.
    let side = 30u32;
    let mut pixels = Vec::new();
    for row in 0..side {
      pixels.extend(std::iter::repeat([[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]][(row / 10) as usize]).take(side as usize).flatten());
    }
    let image = Arc::new(abra_uniffi::Image::from_core(AbraPixels::new_from_pixels(side, side, pixels, abra::abra_core::Channels::RGBA)));
    let preview = ThumbnailPreview::new(image, "".into(), 112, 80);
    preview.size().next((112, 80));
    let start = std::time::Instant::now();
    let mut frame = preview.component().clone().render(Duration::ZERO);
    while pixel(&frame, 56, 40)[1] < 50 && start.elapsed() < Duration::from_secs(5) {
      std::thread::sleep(Duration::from_millis(10));
      frame = preview.component().clone().render(Duration::ZERO);
    }
    // A squashed photo would still be red at row 24 (its top third ends at row 27). Cropped to the card's shape, the middle
    // 71% of the photo is in view, so red ends at row 21 and row 24 is green.
    let at = pixel(&frame, 56, 24);
    assert!(at[1] > at[0], "row 24 should be in the green band, got {at:?}");
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
  fn replacing_the_image_again_and_again_lets_go_of_every_earlier_photo() {
    // Every edit makes the editor give each card a new image, so a card that kept the earlier ones would grow with each edit.
    let first = source([200, 0, 0, 255]);
    let mut photos = vec![Arc::downgrade(&first)];
    let preview = ThumbnailPreview::new(first, "Warm".into(), 112, 80);
    preview.size().next((112, 80));
    for shade in 1..=8u8 {
      let next = source([200 - shade * 20, 0, 0, 255]);
      photos.push(Arc::downgrade(&next));
      preview.set_image(next);
      preview.component().clone().render(Duration::ZERO);
    }
    let start = std::time::Instant::now();
    let earlier = &photos[..photos.len() - 1];
    while earlier.iter().any(|photo| photo.upgrade().is_some()) {
      let held = earlier.iter().filter(|photo| photo.upgrade().is_some()).count();
      assert!(start.elapsed() < Duration::from_secs(5), "{held} of {} earlier photos are still held", earlier.len());
      std::thread::sleep(Duration::from_millis(10));
    }
  }

  #[test]
  fn gradients_are_present_on_first_frame_for_every_aspect_ratio_and_survive_source_replacement() {
    let white = [200, 200, 200, 255];
    for (width, height) in [(16, 32), (32, 16), (32, 32), (1, 1)] {
      let image = Arc::new(abra_uniffi::Image::from_core(AbraPixels::new_from_color(
        width, height, Color::from_rgba(200, 200, 200, 255),
      )));
      let preview = ThumbnailPreview::new(image, "".into(), 112, 80);
      preview.size().next((112, 80));
      let frame = rendered(&preview, white);
      let top = pixel(&frame, 56, 30)[0];
      let bottom = pixel(&frame, 56, 70)[0];
      let expected = |y: u32| {
        (200.0 * (1.0 - THUMBNAIL_GRADIENT_OPACITY * ((y as f32 - 16.0) / 64.0).clamp(0.0, 1.0))).round() as u8
      };
      assert!(top.abs_diff(expected(30)) <= 12, "{width}x{height}: top={top}");
      assert!(bottom.abs_diff(expected(70)) <= 12, "{width}x{height}: bottom={bottom}");
      preview.set_image(source([0, 0, 200, 255]));
      let frame = rendered(&preview, [0, 0, 200, 255]);
      assert!(pixel(&frame, 56, 30)[2].abs_diff(expected(30)) <= 12);
      assert!(pixel(&frame, 56, 70)[2].abs_diff(expected(70)) <= 12);
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

}
