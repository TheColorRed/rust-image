use std::sync::Arc;

use abra::{
  abra_core::{Color, Gradient},
  drawing::prelude::fill,
  prelude::{Area, Image, Size},
};
use vessel::prelude::{Component, Frame, Slider};

use crate::components::frame_of;

/// A bar of colors with a handle: the user drags it to pick one. Read the value when the user lets go. The colors are given, so
/// one bar serves a control that picks a hue, a shade or a tan, and any other bar of colors.
#[derive(uniffi::Object, Component)]
pub struct ColorBarPicker {
  slider: Slider,
}

#[uniffi::export]
impl ColorBarPicker {
  /// A picker that starts at `p_value` (0 to 1 along the bar) and shows `p_colors`, each as red, green and blue from 0 to 255,
  /// evenly spaced from the start of the bar to its end. Panics when a color has fewer than three channels.
  #[uniffi::constructor]
  pub fn new(p_value: f64, p_colors: Vec<Vec<u8>>) -> Arc<Self> {
    let colors = p_colors.iter().map(|color| Color::from_rgb(color[0], color[1], color[2])).collect();
    let slider = Slider::new(p_value as f32)
      .with_track_image(Arc::new(ColorBarPicker::create_gradient_image(&Gradient::evenly(colors))))
      .with_thumb_image(Arc::new(ColorBarPicker::create_gradient_handle()));

    Arc::new(Self { slider })
  }

  /// Where the handle is now, 0 to 1 along the bar.
  ///
  /// Call it while the user drags, to show what is picked, and when they let go, to apply it to the edit
  /// (`sliderCommitRequested` in the editor). Reading it is cheap and does not change anything.
  pub fn value(&self) -> f64 {
    self.slider.value() as f64
  }

  /// Moves the handle to `p_value` (0 to 1) from the program, as when an edit is undone or reset and the handle has to show
  /// what is applied now.
  ///
  /// The editor currently makes a new picker for a new starting value instead of calling this.
  pub fn set_value(&self, p_value: f64) {
    self.slider.set_value(p_value as f32);
  }
}

impl ColorBarPicker {
  /// The colors laid out as a bar. Its height is the bar's thickness.
  fn create_gradient_image(p_gradient: &Gradient) -> Frame {
    let area = Area::from_size(Size::new(200, 14));
    let mut image = Image::new(200, 14);
    fill(&area, p_gradient).apply(&mut image);
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
