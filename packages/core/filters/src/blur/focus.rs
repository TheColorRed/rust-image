use abra_core::Image;
use options::Options;

use crate::blur::{LensBlurOptions, gaussian_blur, lens_blur};

pub enum FocusShape {
  /// Circular focus area.
  Circle,
  /// Square focus area.
  Square,
  /// Diamond-shaped focus area.
  Diamond,
  /// Horizontal focus area.
  Horizontal,
  /// Vertical focus area.
  Vertical,
}

pub enum BlurType {
  Gaussian(u32),
  Lens(LensBlurOptions),
}

macro_rules! clamp_with_fn {
  ($name:ident, $field:ident, $min:expr, $max:expr) => {
    pub fn $name(mut self, value: f32) -> Self {
      self.$field = value.clamp($min, $max);
      self
    }
  };
}

macro_rules! positive_with_fn {
  ($name:ident, $field:ident) => {
    pub fn $name(mut self, value: f32) -> Self {
      self.$field = value.max(0.0);
      self
    }
  };
}

#[allow(unused)]
pub struct FocusGeometry {
  center_x: f32,
  center_y: f32,
  radius: f32,
  sharpness: f32,
  midpoint: f32,
  aspect_ratio: f32,
  rotation: f32,
}

#[allow(unused)]
impl FocusGeometry {
  pub fn new() -> Self {
    Self {
      center_x: 0.5,
      center_y: 0.5,
      radius: 0.75,
      sharpness: 0.25,
      midpoint: 0.5,
      aspect_ratio: 0.0,
      rotation: 0.0,
    }
  }

  clamp_with_fn!(with_center_x, center_x, 0.0, 1.0);
  clamp_with_fn!(with_center_y, center_y, 0.0, 1.0);
  positive_with_fn!(with_radius, radius);
  clamp_with_fn!(with_sharpness, sharpness, 0.0, 1.0);
  clamp_with_fn!(with_midpoint, midpoint, 0.0, 1.0);
  clamp_with_fn!(with_aspect_ratio, aspect_ratio, 0.0, 1.0);
  clamp_with_fn!(with_rotation, rotation, -180.0, 180.0);
}

#[allow(unused)]
pub struct FocusBlurOptions {
  /// Focus geometry configuration.
  geometry: Option<FocusGeometry>,
  /// Shape of the focus area. Defaults to Circle.
  focus_shape: Option<FocusShape>,
  /// Type of blur to apply outside the focus area. Defaults to Gaussian with radius 5.0.
  blur_type: Option<BlurType>,
}

impl FocusBlurOptions {
  pub fn new() -> Self {
    Self {
      geometry: None,
      focus_shape: None,
      blur_type: None,
    }
  }
}

/// Applies a focus blur to an image.
/// - `p_image`: The image to be blurred.
/// - `p_settings`: Settings for the focus blur.
/// - `p_options`: Additional options for applying the blur.
pub fn focus_blur(
  p_image: &mut Image, p_settings: impl Into<Option<FocusBlurOptions>>, p_apply_options: impl Into<Options>,
) {
  let options = p_settings.into().unwrap_or_else(FocusBlurOptions::new);
  match options.blur_type.unwrap_or(BlurType::Gaussian(25)) {
    BlurType::Gaussian(radius) => {
      println!("Applying Gaussian Blur with radius: {}", radius);
      gaussian_blur(p_image, radius, None);
    }
    BlurType::Lens(options) => {
      println!("Applying Lens Blur with options.");
      lens_blur(p_image, options, p_apply_options);
    }
  };
}
