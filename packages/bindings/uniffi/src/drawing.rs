//! `Brush` from `drawing`, and the line it draws through points, `stroke_overlay`.
//!
//! Not exported: a brush's `area`, the `Painter`, which needs a `Path` and an image to change in place, and a brush filled with
//! a gradient or an image. The core draws a brush that is not a solid color as black, so those would silently come out
//! black. Areas take any fill: see the `fill` module. A brush with no color set draws black, as the library's does.

use std::sync::Arc;

use abra::drawing::prelude as drawing;

use crate::{Color, Image};

/// A drawing tool: its size, how soft its edge is, how see-through it is and its color. Each `with_` function gives a changed
/// copy.
#[derive(Clone, Debug, uniffi::Object)]
pub struct Brush(pub drawing::Brush<'static>);

#[uniffi::export]
impl Brush {
  /// A brush of size 5, black, with a soft edge and full opacity.
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self(drawing::Brush::new())
  }

  /// A copy of this brush with this size (its width, in pixels).
  pub fn with_size(&self, size: u32) -> Arc<Brush> {
    Arc::new(Brush(self.0.clone().with_size(size)))
  }

  /// A copy of this brush with this solid color.
  pub fn with_color(&self, color: Arc<Color>) -> Arc<Brush> {
    Arc::new(Brush(self.0.clone().with_color(&color.0)))
  }

  /// A copy of this brush with this hardness, from 0 (a soft, fading edge) to 1 (a sharp edge).
  pub fn with_hardness(&self, hardness: f32) -> Arc<Brush> {
    Arc::new(Brush(self.0.clone().with_hardness(hardness)))
  }

  /// A copy of this brush with this opacity, from 0 (invisible) to 1 (solid).
  pub fn with_opacity(&self, opacity: f32) -> Arc<Brush> {
    Arc::new(Brush(self.0.clone().with_opacity(opacity)))
  }

  /// The size of the brush.
  pub fn size(&self) -> u32 {
    self.0.size()
  }

  /// The hardness of the brush, from 0 to 1.
  pub fn hardness(&self) -> f32 {
    self.0.hardness()
  }

  /// The opacity of the brush, from 0 to 1.
  pub fn opacity(&self) -> f32 {
    self.0.opacity()
  }
}

/// The most points a line is drawn from; a longer one is thinned evenly.
#[uniffi::export]
pub fn max_stroke_points() -> u32 {
  drawing::MAX_STROKE_POINTS as u32
}

fn points_of(points: &[Vec<f32>]) -> Vec<(f32, f32)> {
  points
    .iter()
    .map(|point| {
      let [x, y] = point[..] else { panic!("a point is [x, y], not {} numbers", point.len()) };
      (x, y)
    })
    .collect()
}

/// An image of `width` x `height` pixels with a line drawn through `points` (each `[x, y]`, in pixels) with the brush. Pixels
/// away from the line are transparent, so the image is made to be blended over a photo. The pixels are made on the CPU now.
#[uniffi::export]
pub fn stroke_overlay(width: u32, height: u32, points: Vec<Vec<f32>>, brush: Arc<Brush>) -> Arc<Image> {
  Arc::new(Image::from_core(drawing::stroke_overlay(width, height, &points_of(&points), &brush.0)))
}

/// [`stroke_overlay`] that a GPU makes when its pixels are first read. A line that gains a point is a new image that costs one
/// shader run, so it can follow a finger frame after frame.
#[uniffi::export]
pub fn stroke_overlay_deferred(width: u32, height: u32, points: Vec<Vec<f32>>, brush: Arc<Brush>) -> Arc<Image> {
  Arc::new(Image::from_core(drawing::stroke_overlay_deferred(width, height, &points_of(&points), &brush.0)))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn white() -> Arc<Color> {
    Arc::new(Color::from_rgba(255, 255, 255, 255))
  }

  #[test]
  fn a_brush_keeps_what_it_is_given() {
    let brush = Brush::new().with_size(12).with_hardness(0.25).with_opacity(0.5).with_color(white());
    assert_eq!((brush.size(), brush.hardness(), brush.opacity()), (12, 0.25, 0.5));
    // The first brush is not changed by the copies.
    assert_eq!(Brush::new().size(), 5);
  }

  #[test]
  fn a_line_is_drawn_through_the_points_and_clear_elsewhere() {
    let brush = Brush::new().with_size(9).with_hardness(1.0).with_color(white());
    let image = stroke_overlay(40, 20, vec![vec![5.0, 10.0], vec![35.0, 10.0]], brush.clone());
    assert_eq!(image.get_pixel(20, 10), Some(vec![255, 255, 255, 255]));
    assert_eq!(image.get_pixel(20, 2).unwrap()[3], 0);
    let waiting = stroke_overlay_deferred(40, 20, vec![vec![5.0, 10.0], vec![35.0, 10.0]], brush);
    assert_eq!(waiting.get_pixel(20, 10), Some(vec![255, 255, 255, 255]));
  }

  #[test]
  #[should_panic(expected = "a point is [x, y]")]
  fn a_point_that_is_not_two_numbers_panics() {
    stroke_overlay(8, 8, vec![vec![1.0, 2.0, 3.0]], Arc::new(Brush::new()));
  }

  #[test]
  fn the_longest_line_is_the_library_s() {
    assert_eq!(max_stroke_points() as usize, drawing::MAX_STROKE_POINTS);
  }
}
