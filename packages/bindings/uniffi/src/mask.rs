//! `Mask` and the functions that apply one, from the `mask` crate.
//!
//! `Mask::plane` returns the library's `GrayPlane`, which `values`, `width`, `height` and `get` already read, and `Mask::dimensions` is `width` and
//! `height`. `IntoMaskCow` and `IntoOptionalPointF` are conversions for Rust callers.
//!
//! `apply_mask_to_pixels_rgba` changes a slice in place, which cannot cross to other languages, so it returns the changed
//! pixels. A mask of the wrong size panics, as the library does.

use std::sync::{Arc, RwLock};

use abra::abra_core::{PointF, TransformAlgorithm};
use abra::mask::prelude as library;

use crate::{Area, Color, Image};

// Algorithms for transforming images such as resizing or rotating.
#[uniffi::remote(Enum)]
pub enum TransformAlgorithm {
  NearestNeighbor,
  Bilinear,
  Bicubic,
  Lanczos,
  EdgeDirectNEDI,
  EdgeDirectEDI,
  Auto,
}

/// One byte for each pixel of a picture: `255` is fully applied, `0` is untouched and values between apply partially.
#[derive(Debug, uniffi::Object)]
pub struct Mask(RwLock<library::Mask>);

impl Mask {
  /// A handle for a mask of the library.
  pub fn from_core(mask: library::Mask) -> Self {
    Self(RwLock::new(mask))
  }

  /// A copy of the mask of the library that this holds. Cheap: the values are shared until one of them changes.
  pub fn core(&self) -> library::Mask {
    self.0.read().unwrap().clone()
  }
}

#[uniffi::export]
impl Mask {
  /// A mask the size of an image that lets everything through (all `255`).
  #[uniffi::constructor]
  pub fn new_from_image(image: Arc<Image>) -> Self {
    Self::from_core(library::Mask::new_from_image(&image.core()))
  }

  /// A mask from one value per pixel, row by row from the top left. There must be `width * height` of them.
  #[uniffi::constructor]
  pub fn from_values(width: u32, height: u32, values: Vec<u8>) -> Self {
    Self::from_core(library::Mask::from_values(width, height, values))
  }

  /// A mask from the brightness of each pixel of an image: white is `255` and black is `0`. The alpha is ignored.
  #[uniffi::constructor]
  pub fn from_image(image: Arc<Image>) -> Self {
    Self::from_core(library::Mask::from_image(image.core()))
  }

  /// The width of the mask in pixels.
  pub fn width(&self) -> u32 {
    self.0.read().unwrap().width()
  }

  /// The height of the mask in pixels.
  pub fn height(&self) -> u32 {
    self.0.read().unwrap().height()
  }

  /// The mask values, one per pixel, row by row from the top left.
  pub fn values(&self) -> Vec<u8> {
    self.0.read().unwrap().values().to_vec()
  }

  /// The mask value at a pixel, or nothing outside the mask.
  pub fn get(&self, x: u32, y: u32) -> Option<u8> {
    self.0.read().unwrap().get(x, y)
  }

  /// The mask as a gray image, for drawing or saving.
  pub fn to_image(&self) -> Arc<Image> {
    Arc::new(Image::from_core(self.0.read().unwrap().to_image()))
  }

  /// A copy of the mask scaled to a new size.
  pub fn resized(&self, width: u32, height: u32, algorithm: TransformAlgorithm) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().resized(width, height, algorithm)))
  }

  /// Sets the alpha of every pixel of the image to the mask value there: `255` is opaque and `0` is transparent. The mask
  /// must be the size of the image.
  pub fn apply_to_image(&self, image: Arc<Image>) {
    image.edit(|image| self.0.read().unwrap().apply_to_image(image));
  }

  /// Draws a filled area onto the mask in this color, which is averaged to a gray, with the area's origin at `at` (the
  /// top left corner when there is none).
  pub fn draw_area(&self, area: Arc<Area>, color: Arc<Color>, at: Option<PointF>) {
    self.0.write().unwrap().draw_area(&area.core(), color.0, at);
  }
}

/// The alpha a mask value gives: `255` is opaque and `0` is transparent.
#[uniffi::export]
pub fn mask_value_to_alpha(value: u8) -> u8 {
  library::mask_value_to_alpha(value)
}

/// The gray of an `[r, g, b, a]` pixel, from `0.299 R + 0.587 G + 0.114 B`: white is `255` and black is `0`.
#[uniffi::export]
pub fn rgba_to_gray(rgba: Vec<u8>) -> u8 {
  library::rgba_to_gray(&rgba)
}

/// Sets the alpha of the image from mask data: one byte per pixel, or four (RGBA, taken as gray).
#[uniffi::export]
pub fn apply_mask_to_image(image: Arc<Image>, mask: Vec<u8>) {
  image.edit(|image| library::apply_mask_to_image(image, &mask));
}

/// The RGBA `pixels` with their alpha set from mask data: one byte per pixel, or four (RGBA, taken as gray).
#[uniffi::export]
pub fn apply_mask_to_pixels_rgba(mut pixels: Vec<u8>, mask: Vec<u8>) -> Vec<u8> {
  library::apply_mask_to_pixels_rgba(&mut pixels, &mask);
  pixels
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Color;

  #[test]
  fn a_mask_reads_back_what_it_was_made_from() {
    let mask = Mask::from_values(2, 1, vec![0, 255]);
    assert_eq!((mask.width(), mask.height()), (2, 1));
    assert_eq!(mask.values(), vec![0, 255]);
    assert_eq!(mask.get(1, 0), Some(255));
    assert_eq!(mask.get(2, 0), None);
  }

  #[test]
  fn a_mask_sets_the_alpha_of_an_image() {
    let image =
      Arc::new(Image::new_from_color(2, 1, Arc::new(crate::Color(abra::abra_core::Color::from_rgb(10, 20, 30)))));
    Mask::from_values(2, 1, vec![0, 128]).apply_to_image(image.clone());
    assert_eq!(image.rgba(), vec![10, 20, 30, 0, 10, 20, 30, 128]);
    assert_eq!(apply_mask_to_pixels_rgba(vec![1, 2, 3, 255], vec![7]), vec![1, 2, 3, 7]);
  }

  #[test]
  fn a_mask_is_resized_and_made_from_an_image() {
    let mask = Mask::from_values(1, 1, vec![200]);
    let bigger = mask.resized(4, 4, TransformAlgorithm::NearestNeighbor);
    assert_eq!((bigger.width(), bigger.height()), (4, 4));
    assert!(bigger.values().iter().all(|&value| value == 200));
    let from_image = Mask::from_image(mask.to_image());
    assert_eq!(from_image.values(), vec![200]);
    assert_eq!(Mask::new_from_image(mask.to_image()).values(), vec![255]);
  }

  #[test]
  fn an_area_is_drawn_onto_a_mask() {
    let mask = Mask::from_values(4, 4, vec![0; 16]);
    let white = Arc::new(Color(abra::abra_core::Color::from_rgb(255, 255, 255)));
    mask.draw_area(
      Arc::new(Area::rect(PointF::new(0, 0), abra::abra_core::Size::new(2, 2))),
      white,
      Some(PointF::new(1, 1)),
    );
    assert_eq!(mask.get(1, 1), Some(255));
    assert_eq!(mask.get(0, 0), Some(0));
    assert_eq!(mask.get(3, 3), Some(0));
  }
}
