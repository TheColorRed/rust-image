//! `ColorStop` and `Gradient` from `abra-core`, and `radial_gradient` from `adjustments`, which draws one.
//!
//! `Gradient::with_direction` and `Gradient::direction` are not exported: they take and return a `Path`, which is in the
//! geometry module.

use std::sync::Arc;

use abra::abra_core;
use abra::adjustments::prelude::color;

use crate::{Color, Harmony, Image};
use abra_core::ColorInterpolation;

/// A color at a position along a gradient.
#[derive(Debug, uniffi::Object)]
pub struct ColorStop(pub abra_core::ColorStop);

#[uniffi::export]
impl ColorStop {
  /// A stop with this color at `time`, from 0 to 1.
  #[uniffi::constructor]
  pub fn new(color: Arc<Color>, time: f32) -> Self {
    Self(abra_core::ColorStop::new(color.0, time))
  }

  /// The color of the stop.
  pub fn color(&self) -> Arc<Color> {
    Arc::new(Color(self.0.color))
  }

  /// The position of the stop, from 0 to 1.
  pub fn time(&self) -> f32 {
    self.0.time
  }
}

/// Colors that blend into each other.
#[derive(Debug, uniffi::Object)]
pub struct Gradient(pub abra_core::Gradient);

#[uniffi::export]
impl Gradient {
  /// A gradient through these stops, in any order.
  #[uniffi::constructor]
  pub fn new(stops: Vec<Arc<ColorStop>>) -> Self {
    Self(abra_core::Gradient::new(stops.into_iter().map(|stop| stop.0).collect()))
  }

  /// A gradient from one color to another.
  #[uniffi::constructor]
  pub fn from_to(from: Arc<Color>, to: Arc<Color>) -> Self {
    Self(abra_core::Gradient::from_to(from.0, to.0))
  }

  /// A gradient through these colors, evenly spaced.
  #[uniffi::constructor]
  pub fn evenly(colors: Vec<Arc<Color>>) -> Self {
    Self(abra_core::Gradient::evenly(colors.into_iter().map(|color| color.0).collect()))
  }

  /// An evenly spaced gradient through a color harmony of `color`.
  #[uniffi::constructor]
  pub fn harmony(color: Arc<Color>, harmony: Harmony) -> Self {
    Self(abra_core::Gradient::harmony(color.0, harmony.into()))
  }

  /// Red, orange, yellow, green, blue, indigo and violet.
  #[uniffi::constructor]
  pub fn rainbow() -> Self {
    Self(abra_core::Gradient::rainbow())
  }

  /// The hues of the color wheel, going from 360 to 0.
  #[uniffi::constructor]
  pub fn hue() -> Self {
    Self(abra_core::Gradient::hue())
  }

  /// A copy of the gradient whose colors between two stops are mixed in this color space, and for a polar space this way round the wheel.
  /// The default is `{ space: ColorSpace.Srgb, hue: HueDirection.Shorter }`.
  pub fn with_interpolation(&self, interpolation: ColorInterpolation) -> Arc<Gradient> {
    Arc::new(Gradient(self.0.clone().with_interpolation(interpolation)))
  }

  /// How the colors between two stops are mixed.
  pub fn interpolation(&self) -> ColorInterpolation {
    self.0.interpolation()
  }

  /// The color at `time`, from 0 to 1, mixing the two stops around it as the interpolation says.
  pub fn color_at(&self, time: f32) -> Arc<Color> {
    Arc::new(Color(self.0.color_at(time)))
  }

  /// The gradient the other way round.
  pub fn reverse(&self) -> Arc<Gradient> {
    Arc::new(Gradient(self.0.reverse()))
  }
}

/// An image of `width` x `height` pixels with a radial gradient: the first stop at the center and the last on the oval around
/// it, and the last color beyond. The center and the radii are in pixels. On a GPU it is made only when its pixels are read.
#[uniffi::export]
pub fn radial_gradient(
  width: u32, height: u32, gradient: Arc<Gradient>, center_x: f32, center_y: f32, radius_x: f32, radius_y: f32,
) -> Arc<Image> {
  Arc::new(Image::from_core(color::radial_gradient_deferred(
    width,
    height,
    &gradient.0,
    (center_x, center_y),
    (radius_x, radius_y),
  )))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn rgb(r: u8, g: u8, b: u8) -> Arc<Color> {
    Arc::new(Color::from_rgb(r, g, b))
  }

  #[test]
  fn a_gradient_mixes_in_the_space_it_is_given() {
    use abra_core::{ColorSpace, HueDirection};
    let plain = Gradient::from_to(rgb(0, 0, 255), rgb(255, 255, 0));
    assert_eq!(plain.interpolation(), ColorInterpolation::default());
    assert_eq!(plain.color_at(0.5).rgb(), vec![128, 128, 128]);
    let oklab = ColorInterpolation { space: ColorSpace::Oklab, hue: HueDirection::Shorter };
    let clean = plain.with_interpolation(oklab);
    assert_eq!(clean.interpolation(), oklab);
    assert_eq!(clean.color_at(0.5).0, abra_core::Color::from_rgb(0, 0, 255).mix(abra_core::Color::from_rgb(255, 255, 0), 0.5, ColorSpace::Oklab));
    assert_eq!(clean.reverse().interpolation().space, ColorSpace::Oklab);
  }

  #[test]
  fn the_first_color_is_at_the_center_and_the_last_beyond_the_edge() {
    let gradient = Arc::new(Gradient::from_to(rgb(255, 0, 0), rgb(0, 0, 255)));
    let image = radial_gradient(101, 101, gradient, 50.5, 50.5, 50.0, 50.0);
    assert_eq!((image.width(), image.height()), (101, 101));
    assert_eq!(image.get_pixel(50, 50).unwrap()[0], 255);
    assert_eq!(image.get_pixel(0, 0).unwrap()[2], 255);
  }

  #[test]
  fn a_gradient_blends_between_its_stops() {
    let gradient = Gradient::from_to(rgb(0, 0, 0), rgb(200, 100, 0));
    assert_eq!(gradient.color_at(0.5).0.rgb(), (100, 50, 0));
    assert_eq!(gradient.color_at(1.0).0.rgb(), (200, 100, 0));
  }

  #[test]
  fn stops_given_out_of_order_are_sorted() {
    let stop = |time: f32, red: u8| Arc::new(ColorStop::new(rgb(red, 0, 0), time));
    let gradient = Gradient::new(vec![stop(1.0, 200), stop(0.0, 0)]);
    assert_eq!(gradient.color_at(0.0).0.rgb().0, 0);
    assert_eq!(stop(0.25, 7).time(), 0.25);
    assert_eq!(stop(0.25, 7).color().0.rgb().0, 7);
  }

  #[test]
  fn the_ready_made_gradients_match_the_library() {
    assert_eq!(Gradient::rainbow().color_at(0.0).0, abra_core::Gradient::rainbow().color_at(0.0));
    assert_eq!(Gradient::hue().color_at(0.5).0, abra_core::Gradient::hue().color_at(0.5));
    let evenly = Gradient::evenly(vec![rgb(0, 0, 0), rgb(100, 0, 0), rgb(200, 0, 0)]);
    assert_eq!(evenly.color_at(0.5).0.rgb(), (100, 0, 0));
    let harmony = Gradient::harmony(Arc::new(Color::red()), Harmony::Triadic);
    assert_eq!(harmony.color_at(0.0).0, abra_core::Color::red());
  }

  #[test]
  fn reversing_swaps_the_ends() {
    let gradient = Gradient::from_to(rgb(255, 0, 0), rgb(0, 0, 255));
    let back = gradient.reverse();
    assert_eq!(back.color_at(0.0).0.rgb(), (0, 0, 255));
    assert_eq!(back.color_at(1.0).0.rgb(), (255, 0, 0));
  }
}
