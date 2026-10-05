use std::sync::Arc;

use abra::{
  drawing::prelude::fill,
  prelude::{Area, Image, Size},
};
use vessel::prelude::{Component, Frame, Observable, Observer, Slider};

use crate::{
  components::image_preview::{ImagePreview, Message},
  effect_spec::skin_tan_gradient,
  image::frame_of,
};

/// The key the picker sends to the preview, and the editor's control for tanning.
const KEY: &str = "action-skin-tan";

/// A bar of tans with a handle: the user drags it to pick how tanned the skin in the photo gets. The preview shows each
/// position as it is picked; the value is read when the user lets go, to apply it to the edit.
#[derive(uniffi::Object, Component)]
pub struct SkinTanColorPicker {
  slider: Slider,
}

#[uniffi::export]
impl SkinTanColorPicker {
  /// A picker that shows its tans on `p_preview` and starts at `p_value` (0 to 1, where 0 is no tan).
  #[uniffi::constructor]
  pub fn new(p_preview: Arc<ImagePreview>, p_value: f64) -> Arc<Self> {
    let slider = Slider::new(p_value as f32)
      .with_track_image(Arc::new(SkinTanColorPicker::create_gradient_image()))
      .with_thumb_image(Arc::new(SkinTanColorPicker::create_gradient_handle()));

    // Send each position to the image_preview component.
    let preview = p_preview.subject::<Message>();
    slider.changes().subscribe(move |changed| preview.next(Message::SliderMove(KEY.into(), changed.0 as f64)));

    Arc::new(Self { slider })
  }

  /// Where the handle is now, 0 to 1 (0 is no tan, 1 is the darkest).
  ///
  /// Call it when the user lets go, to apply the tan to the edit (`sliderCommitRequested` in the editor). You do not need
  /// it while they drag: the picker already sends every position to the preview itself, so the photo follows the handle
  /// with no help from JavaScript. Reading it is cheap and does not change anything.
  pub fn value(&self) -> f64 {
    self.slider.value() as f64
  }

  /// Moves the handle to `p_value` (0 to 1) from the program, as when an edit is undone or reset and the handle has to
  /// show what is applied now.
  ///
  /// It only moves the handle: nothing is sent to the preview, because the photo is already showing that state (the
  /// edit's result). To also preview a value that is not applied yet, send a `SliderMove` to the preview as well. The
  /// user's own drags do not need this, since the handle follows their finger. The editor currently makes a new picker
  /// for a new starting value instead of calling this.
  pub fn set_value(&self, p_value: f64) {
    self.slider.set_value(p_value as f32);
  }
}

impl SkinTanColorPicker {
  /// The tan scale as a picture, for the bar. Its height is the bar's thickness.
  fn create_gradient_image() -> Frame {
    let area = Area::from_size(Size::new(200, 14));
    let mut image = Image::new(200, 14);
    fill(&area, &skin_tan_gradient()).apply(&mut image);
    frame_of(&image)
  }

  /// The handle as a picture: a white ring with a soft shadow, hollow so the tan under it shows through.
  fn create_gradient_handle() -> Frame {
    const SIZE: u32 = 36;
    // The ring runs from `INNER` to `OUTER` pixels from the center, edged on the inside with a thin dark line, and the
    // shadow fades out beyond it.
    const OUTER: f32 = 14.0;
    const INNER: f32 = 9.5;
    let center = SIZE as f32 / 2.0;
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
      for x in 0..SIZE {
        let distance = ((x as f32 + 0.5 - center).powi(2) + (y as f32 + 0.5 - center).powi(2)).sqrt();
        // How much of the pixel is inside a shape, with a one pixel soft edge.
        let within =
          |p_from: f32, p_to: f32| (p_to - distance + 0.5).clamp(0.0, 1.0) * (distance - p_from + 0.5).clamp(0.0, 1.0);
        let shadow =
          (1.0 - (distance - INNER) / (center - INNER)).clamp(0.0, 1.0) * 0.35 * (distance > INNER - 1.0) as u8 as f32;
        let line = within(INNER - 1.5, INNER - 0.5) * 0.25;
        let ring = within(INNER, OUTER);
        // Black shadow, then the dark line, then the white ring on top, composited by alpha.
        let mut alpha = shadow;
        let mut shade = 0.0;
        for (layer_alpha, layer_shade) in [(line, 0.0), (ring, 1.0)] {
          let out = layer_alpha + alpha * (1.0 - layer_alpha);
          if out > 0.0 {
            shade = (layer_shade * layer_alpha + shade * alpha * (1.0 - layer_alpha)) / out;
          }
          alpha = out;
        }
        let level = (shade * 255.0).round() as u8;
        pixels.extend_from_slice(&[level, level, level, (alpha * 255.0).round() as u8]);
      }
    }
    Frame {
      width: SIZE,
      height: SIZE,
      pixels: pixels.into(),
    }
  }
}
