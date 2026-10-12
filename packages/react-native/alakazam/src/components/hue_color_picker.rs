use std::sync::Arc;

use abra::{
  abra_core::Gradient,
  drawing::prelude::fill,
  prelude::{Area, Image, Size},
};
use vessel::prelude::{Component, Frame, Slider};

use crate::components::{frame_of, skin_tan_color_picker::SkinTanColorPicker};

/// A bar of hues with a handle: the user drags it to pick a color. It only picks; read the value when the user lets go.
#[derive(uniffi::Object, Component)]
pub struct HueColorPicker {
  slider: Slider,
}

#[uniffi::export]
impl HueColorPicker {
  /// A picker that starts at `p_value` (0 to 1 along `Gradient::hue()`, where 0 and 1 are both red).
  #[uniffi::constructor]
  pub fn new(p_value: f64) -> Arc<Self> {
    let slider = Slider::new(p_value as f32)
      .with_track_image(Arc::new(HueColorPicker::create_gradient_image()))
      .with_thumb_image(Arc::new(SkinTanColorPicker::create_gradient_handle()));
    Arc::new(Self { slider })
  }

  /// Where the handle is now, 0 to 1 along `Gradient::hue()`.
  pub fn value(&self) -> f64 {
    self.slider.value() as f64
  }

  /// Moves the handle to `p_value` (0 to 1) from the program.
  pub fn set_value(&self, p_value: f64) {
    self.slider.set_value(p_value as f32);
  }
}

impl HueColorPicker {
  /// The color wheel laid out as a bar. Its height is the bar's thickness.
  fn create_gradient_image() -> Frame {
    let area = Area::from_size(Size::new(200, 14));
    let mut image = Image::new(200, 14);
    let wheel = Gradient::hue();
    fill(&area, &wheel).apply(&mut image);
    frame_of(&image)
  }
}
