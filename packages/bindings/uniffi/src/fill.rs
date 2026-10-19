//! What an area is filled with (a solid color, a gradient or an image) and the fill of an area, from `drawing`. An area takes
//! any of the three; a brush takes only a solid color.

use std::borrow::Cow;
use std::sync::Arc;

use abra::abra_core;
use abra::drawing::prelude as drawing;

use crate::{Area, Color, Gradient, Image};

/// Which directions an image fill repeats in. An image that does not repeat in a direction is drawn once there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, uniffi::Enum)]
pub enum ImageRepeat {
  /// Drawn once, at the top-left corner of the area.
  #[default]
  NoRepeat,
  /// Tiled in both directions.
  Both,
  /// Tiled horizontally only.
  X,
  /// Tiled vertically only.
  Y,
}

impl From<ImageRepeat> for abra_core::ImageRepeat {
  fn from(p_repeat: ImageRepeat) -> Self {
    match p_repeat {
      ImageRepeat::NoRepeat => abra_core::ImageRepeat::NoRepeat,
      ImageRepeat::Both => abra_core::ImageRepeat::Both,
      ImageRepeat::X => abra_core::ImageRepeat::X,
      ImageRepeat::Y => abra_core::ImageRepeat::Y,
    }
  }
}

/// What an area is filled with: a solid color, a gradient or an image.
#[derive(Clone, uniffi::Object)]
pub struct Fill(pub abra_core::Fill<'static>);

#[uniffi::export]
impl Fill {
  /// A solid color.
  #[uniffi::constructor]
  pub fn solid(color: Arc<Color>) -> Self {
    Self(abra_core::Fill::solid(color.0.clone()))
  }

  /// A gradient, laid out across the area it fills.
  #[uniffi::constructor]
  pub fn gradient(gradient: Arc<Gradient>) -> Self {
    Self(abra_core::Fill::gradient(Cow::Owned(gradient.0.clone())))
  }

  /// An image, placed at the top-left corner of the area it fills, drawn once unless changed with [`Fill::with_repeat`].
  #[uniffi::constructor]
  pub fn image(image: Arc<Image>) -> Self {
    Self(abra_core::Fill::image(Arc::new(image.core())))
  }

  /// A copy of this fill that tiles its image in the given directions. A solid or gradient fill is returned unchanged.
  pub fn with_repeat(&self, repeat: ImageRepeat) -> Arc<Fill> {
    Arc::new(Fill(self.0.clone().with_repeat(repeat.into())))
  }
}

/// A fill of an area that is drawn when asked for. Made by [`fill_area`].
#[derive(Clone, uniffi::Object)]
pub struct FillArea(pub drawing::FillArea<'static>);

#[uniffi::export]
impl FillArea {
  /// A copy of this fill that draws its top-left corner at (`x`, `y`), in pixels.
  pub fn with_position(&self, x: f32, y: f32) -> Arc<FillArea> {
    Arc::new(FillArea(self.0.clone().with_position((x, y))))
  }

  /// A copy of this fill whose opacity is multiplied by the red channel of `mask`. The mask must be the same size as the
  /// area's bounds, or drawing panics.
  pub fn with_mask(&self, mask: Arc<Image>) -> Arc<FillArea> {
    Arc::new(FillArea(self.0.clone().with_mask(mask.core())))
  }

  /// The filled area as a new image the size of its bounds.
  pub fn to_image(&self) -> Arc<Image> {
    Arc::new(Image::from_core(self.0.to_image()))
  }

  /// Draws the filled area over `target`, at the position set by [`FillArea::with_position`], or else at the area's bounds. The
  /// GPU does it when there is one and the fill fits, and the target is then not changed until its pixels are read; otherwise the
  /// CPU does, with the same result.
  pub fn apply(&self, target: Arc<Image>) {
    target.edit(|image| self.0.apply(image));
  }
}

/// The fill of `area` with `fill`. Draw it with [`FillArea::apply`], or get it as an image with [`FillArea::to_image`].
#[uniffi::export]
pub fn fill_area(area: Arc<Area>, fill: Arc<Fill>) -> Arc<FillArea> {
  Arc::new(FillArea(drawing::fill(area.core(), fill.0.clone())))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn black() -> Arc<Color> {
    Arc::new(Color::from_rgba(0, 0, 0, 255))
  }

  fn white() -> Arc<Color> {
    Arc::new(Color::from_rgba(255, 255, 255, 255))
  }

  fn red() -> Arc<Color> {
    Arc::new(Color::from_rgba(255, 0, 0, 255))
  }

  fn rect(x: f64, y: f64, width: f64, height: f64) -> Arc<Area> {
    Arc::new(Area::from_core(abra_core::Area::rect((x, y), (width, height))))
  }

  fn canvas(width: u32, height: u32) -> Arc<Image> {
    Arc::new(Image::from_core(abra_core::Image::new_from_color(width, height, abra_core::Color::from_rgba(0, 0, 0, 255))))
  }

  #[test]
  fn a_solid_fill_draws_only_inside_the_area() {
    let image = canvas(8, 8);
    fill_area(rect(4.0, 4.0, 2.0, 2.0), Arc::new(Fill::solid(red()))).apply(image.clone());
    assert_eq!(image.get_pixel(4, 4), Some(vec![255, 0, 0, 255]));
    assert_eq!(image.get_pixel(1, 1), Some(vec![0, 0, 0, 255]));
  }

  #[test]
  fn with_position_moves_the_fill() {
    let image = canvas(8, 8);
    fill_area(rect(4.0, 4.0, 2.0, 2.0), Arc::new(Fill::solid(red())))
      .with_position(0.0, 0.0)
      .apply(image.clone());
    assert_eq!(image.get_pixel(0, 0), Some(vec![255, 0, 0, 255]));
    assert_eq!(image.get_pixel(4, 4), Some(vec![0, 0, 0, 255]));
  }

  #[test]
  fn a_gradient_fill_changes_across_the_area() {
    let gradient = Arc::new(Gradient::from_to(black(), white()));
    let filled = fill_area(rect(0.0, 0.0, 8.0, 8.0), Arc::new(Fill::gradient(gradient))).to_image();
    let left = filled.get_pixel(0, 4).unwrap()[0];
    let right = filled.get_pixel(7, 4).unwrap()[0];
    assert!(right > left, "a gradient from black to white should get lighter, got {left} then {right}");
  }

  #[test]
  fn an_image_fill_is_the_image() {
    let source = Arc::new(Image::from_core(abra_core::Image::new_from_color(4, 4, abra_core::Color::from_rgba(255, 0, 0, 255))));
    let filled = fill_area(rect(0.0, 0.0, 4.0, 4.0), Arc::new(Fill::image(source))).to_image();
    assert_eq!(filled.get_pixel(2, 2), Some(vec![255, 0, 0, 255]));
  }

  #[test]
  fn an_image_fill_repeats_when_asked() {
    let tile = Arc::new(Image::from_core(abra_core::Image::new_from_pixels(
      1,
      2,
      vec![255, 0, 0, 255, 0, 0, 0, 0],
      abra_core::Channels::RGBA,
    )));
    let fill_value = Fill::image(tile).with_repeat(ImageRepeat::Y);
    let filled = fill_area(rect(0.0, 0.0, 1.0, 4.0), fill_value).to_image();
    assert_eq!(filled.get_pixel(0, 0), Some(vec![255, 0, 0, 255]));
    assert_eq!(filled.get_pixel(0, 1), Some(vec![0, 0, 0, 0]));
    assert_eq!(filled.get_pixel(0, 2), Some(vec![255, 0, 0, 255]));
  }

  #[test]
  fn a_mask_of_zero_hides_the_fill() {
    let mask = canvas(4, 4);
    let filled = fill_area(rect(0.0, 0.0, 4.0, 4.0), Arc::new(Fill::solid(red()))).with_mask(mask).to_image();
    assert_eq!(filled.get_pixel(2, 2).unwrap()[3], 0);
  }
}
