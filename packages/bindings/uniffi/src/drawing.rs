//! `Brush` and `Painter` from `drawing`, and the line a brush draws through points, `stroke_overlay`.
//!
//! The `Painter` holds the image it draws on, which it changes in place, so it is a handle of the image and not a borrow of it.
//!
//! Not exported: a brush's `area`, a `Painter`'s performance settings, and a brush filled with a gradient or an image. The core draws a
//! brush that is not a solid color as black, so those would silently come out black. Areas take any fill: see the `fill` module. A
//! brush with no color set draws black, as the library's does.
//!
//! A brush's width and opacity along a stroke are used by `Painter::stroke_with_brush`, along a path, and by `stroke_overlay`, along the
//! line through its points. A dab and `fill_area_with_brush` have no line to run along, so they use the brush's size and its opacity at the
//! start. The library's `with_width` and `with_opacity` take a number or a `StrokeWidth` or `StrokeOpacity`, which uniffi cannot overload, so a
//! number is `with_width` and `with_opacity` and the other is `with_width_along` and `with_opacity_along`.

use std::sync::Arc;

use abra::drawing::prelude as drawing;

use crate::{Area, Color, Image, Path, StrokeOpacity, StrokeWidth};

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

  /// A copy of this brush with this opacity, from 0 (invisible) to 1 (solid), the same all the way along a stroke.
  pub fn with_opacity(&self, opacity: f32) -> Arc<Brush> {
    Arc::new(Brush(self.0.clone().with_opacity(opacity)))
  }

  /// A copy of this brush as opaque at each place along a stroke as the `StrokeOpacity` says, where `0` is the start of the stroke and `1`
  /// is its end.
  pub fn with_opacity_along(&self, opacity: Arc<StrokeOpacity>) -> Arc<Brush> {
    Arc::new(Brush(self.0.clone().with_opacity(opacity.0.clone())))
  }

  /// A copy of this brush whose stroke is this wide the whole way along a path or the line through points. A dab keeps the brush's size. It
  /// replaces the size for a stroke, and a size set after it replaces it.
  pub fn with_width(&self, width: f32) -> Arc<Brush> {
    Arc::new(Brush(self.0.clone().with_width(width)))
  }

  /// A copy of this brush whose stroke along a path, or along the line through points, is as wide at each place along it as the
  /// `StrokeWidth` says, where `0` is the start and `1` is the end.
  pub fn with_width_along(&self, width: Arc<StrokeWidth>) -> Arc<Brush> {
    Arc::new(Brush(self.0.clone().with_width(width.0.clone())))
  }

  /// The size of the brush.
  pub fn size(&self) -> u32 {
    self.0.size()
  }

  /// How wide a stroke along a path is: what was set with `with_width`, or the size all the way along.
  pub fn width(&self) -> Arc<StrokeWidth> {
    Arc::new(StrokeWidth(self.0.width()))
  }

  /// The hardness of the brush, from 0 to 1.
  pub fn hardness(&self) -> f32 {
    self.0.hardness()
  }

  /// How opaque the brush is along a stroke: what was set with `with_opacity` or `with_opacity_along`, or solid.
  pub fn opacity(&self) -> Arc<StrokeOpacity> {
    Arc::new(StrokeOpacity(self.0.opacity()))
  }
}

/// Draws with brushes on an image, in place.
#[derive(Debug, uniffi::Object)]
pub struct Painter {
  image: Arc<Image>,
}

#[uniffi::export]
impl Painter {
  /// A painter that draws on this image.
  #[uniffi::constructor]
  pub fn new(image: Arc<Image>) -> Self {
    Self { image }
  }

  /// Paints one dab of the brush at a place in the image.
  pub fn dab_brush(&self, x: f32, y: f32, brush: Arc<Brush>) {
    self.image.edit(|image| drawing::Painter::new(image).dab_brush(x, y, &brush.0));
  }

  /// Strokes a path with the brush, as wide along it as the brush's width says.
  pub fn stroke_with_brush(&self, path: Arc<Path>, brush: Arc<Brush>) {
    self.image.edit(|image| drawing::Painter::new(image).stroke_with_brush(&path.core(), &brush.0));
  }

  /// Fills an area with dabs of the brush.
  pub fn fill_area_with_brush(&self, area: Arc<Area>, brush: Arc<Brush>) {
    self.image.edit(|image| drawing::Painter::new(image).fill_area_with_brush(&area.core(), &brush.0));
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

/// An image of `width` x `height` pixels with a line drawn through `points` (each `[x, y]`, in pixels) with the brush, as wide along it as
/// the brush's width says. Pixels away from the line are transparent, so the image is made to be blended over a photo. The pixels are
/// made on the CPU now.
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
    assert_eq!((brush.size(), brush.hardness(), brush.opacity().constant_opacity()), (12, 0.25, Some(0.5)));
    assert_eq!(Brush::new().opacity().constant_opacity(), Some(1.0));
    // The first brush is not changed by the copies.
    assert_eq!(Brush::new().size(), 5);
  }

  #[test]
  fn a_stroke_that_tapers_is_drawn_thinner_toward_its_end() {
    let image = Arc::new(Image::new(100, 40));
    let path = Path::line(crate::point_f_new(10.0, 20.0), crate::point_f_new(90.0, 20.0));
    let brush = Brush::new().with_size(20).with_hardness(1.0).with_color(white()).with_width_along(Arc::new(StrokeWidth::from_to(20.0, 2.0)));
    assert_eq!(brush.width().width_at(1.0), 2.0);
    assert_eq!(Brush::new().with_size(7).width().width_at(0.5), 7.0);
    Painter::new(image.clone()).stroke_with_brush(Arc::new(path), brush);
    let painted = |x: u32| (0..40).filter(|y| image.get_pixel(x, *y).is_some_and(|pixel| pixel[3] > 127)).count();
    assert!(painted(15) > painted(50) && painted(50) > painted(85), "{} {} {}", painted(15), painted(50), painted(85));
  }

  #[test]
  fn a_stroke_that_fades_out_is_drawn_clearer_toward_its_end() {
    let brush = Brush::new().with_size(10).with_hardness(1.0).with_color(white()).with_opacity_along(Arc::new(StrokeOpacity::from_to(1.0, 0.0)));
    assert_eq!(brush.opacity().opacity_at(0.5), 0.5);
    let image = stroke_overlay(100, 30, vec![vec![10.0, 15.0], vec![90.0, 15.0]], brush);
    let alpha = |x: u32| image.get_pixel(x, 15).unwrap()[3];
    assert!(alpha(14) > 220 && alpha(50) > 100 && alpha(50) < 150 && alpha(86) < 40, "{} {} {}", alpha(14), alpha(50), alpha(86));
    // A number is the same opacity all the way, and a width given as a number is the same width.
    let half = stroke_overlay(100, 30, vec![vec![10.0, 15.0], vec![90.0, 15.0]], Brush::new().with_size(3).with_hardness(1.0).with_color(white()).with_width(10.0).with_opacity(0.5));
    assert!((half.get_pixel(50, 15).unwrap()[3] as i32 - 128).abs() <= 2);
    assert!(half.get_pixel(50, 11).unwrap()[3] > 0);
  }

  #[test]
  fn a_painter_dabs_and_fills_in_place() {
    let image = Arc::new(Image::new(30, 30));
    let painter = Painter::new(image.clone());
    let brush = Brush::new().with_size(8).with_hardness(1.0).with_color(white());
    painter.dab_brush(15.0, 15.0, brush.clone());
    assert_eq!(image.get_pixel(15, 15).unwrap()[3], 255);
    assert_eq!(image.get_pixel(2, 2).unwrap()[3], 0);
    let area = Arc::new(Area::rect(crate::point_f_new(0.0, 0.0), crate::size_new(10.0, 10.0)));
    painter.fill_area_with_brush(area, brush);
    assert!(image.get_pixel(5, 5).unwrap()[3] > 0);
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
