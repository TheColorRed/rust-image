//! `Mask` and the functions that apply one, from the `mask` crate.
//!
//! `Mask::plane` returns the library's `GrayPlane`, which `values`, `width`, `height` and `get` already read, and `Mask::dimensions` is `width` and
//! `height`. `IntoMaskCow` and `IntoOptionalPointF` are conversions for Rust callers.
//!
//! `apply_mask_to_pixels_rgba` changes a slice in place, which cannot cross to other languages, so it returns the changed
//! pixels. A mask of the wrong size panics, as the library does.

use std::sync::{Arc, RwLock};

use abra::abra_core::{ColorStat, PointF, Rect, Size, TransformAlgorithm};
use abra::mask::prelude as library;
use abra::mask::prelude::{Blob, Falloff};

use crate::{Area, Color, Fill, Image};

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

/// How a `Blob` fades from its middle out to its edge.
#[uniffi::remote(Enum)]
pub enum Falloff {
  Gaussian { fade: f32, reach: f32 },
  Smooth { fade: f32 },
}

/// A soft oval to put in a mask: a place, a size, a turn and how it fades.
#[uniffi::remote(Record)]
pub struct Blob {
  pub center: PointF,
  pub half_size: Size,
  pub angle: f32,
  pub falloff: Falloff,
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

  /// A mask that lets nothing through (all `0`), to draw into or to add to.
  #[uniffi::constructor]
  pub fn empty(width: u32, height: u32) -> Self {
    Self::from_core(library::Mask::empty(width, height))
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

  /// A mask of soft ovals, for shading a face or any region with a place and a shape. Where ovals overlap the strongest one wins.
  #[uniffi::constructor]
  pub fn from_blobs(width: u32, height: u32, blobs: Vec<Blob>) -> Self {
    Self::from_core(library::Mask::from_blobs(width, height, &blobs))
  }

  /// A mask of the pixels of an image that are close to a color: `255` where a pixel has the color's hue (within `hue_range`
  /// degrees either way) and is at least `min_saturation` and `min_value` (each `0` to `1`), `0` where it does not, and
  /// values between along the edge when `feather` (`0` to `1`) is more than `0`.
  #[uniffi::constructor]
  pub fn from_color_range(
    image: Arc<Image>, color: Arc<Color>, hue_range: f32, min_saturation: f32, min_value: f32, feather: f32,
  ) -> Self {
    Self::from_core(library::Mask::from_color_range(
      &image.core(),
      color.0,
      hue_range,
      min_saturation,
      min_value,
      feather,
    ))
  }

  /// A mask of the pixels of an image that are between two brightnesses: `255` where a pixel is no darker than `darkest` and no
  /// lighter than `lightest` (each `0` to `255`), `0` where it is not, and values between along the edge when `feather` (a number of
  /// levels of brightness) is more than `0`. Use it to pick out the shadows, the highlights or the midtones of a picture.
  #[uniffi::constructor]
  pub fn from_brightness_range(image: Arc<Image>, darkest: u8, lightest: u8, feather: u8) -> Self {
    Self::from_core(library::Mask::from_brightness_range(&image.core(), darkest, lightest, feather))
  }

  /// A mask of how much of each pixel of `image` is a foreground color rather than a background color, taking each pixel's color as a
  /// mix of the two: `255` for exactly the foreground color, `0` for exactly the background color and half for halfway. The two colors are
  /// pictures, one color for each pixel, the same size as `image`; a picture of another size panics. Where they are closer together than
  /// `least_difference` (a distance with each channel from 0 to 1) the pixel's color cannot decide, and `fallback` counts for more, down to
  /// deciding nothing at no distance. Use it to pull out thin parts such as hair against a background of another color.
  #[uniffi::constructor]
  pub fn from_color_mix(
    image: Arc<Image>, foreground: Arc<Image>, background: Arc<Image>, fallback: Arc<Mask>, least_difference: f32,
  ) -> Self {
    Self::from_core(library::Mask::from_color_mix(
      &image.core(),
      &foreground.core(),
      &background.core(),
      &fallback.core(),
      least_difference,
    ))
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

  /// The smallest rectangle holding every pixel whose value is above `threshold`, or nothing when there is none.
  pub fn bounds(&self, threshold: u8) -> Option<Rect> {
    self.0.read().unwrap().bounds(threshold)
  }

  /// The outlines of the shapes in the mask: each is a closed loop of points in the mask's pixels (the last joins the first). A pixel is inside a
  /// shape when its value is at or above `level`, and an outline crosses between two neighbors in proportion to their values. An outer outline
  /// runs counterclockwise on the screen and a hole runs clockwise.
  pub fn contours(&self, level: u8) -> Vec<Vec<PointF>> {
    self.0.read().unwrap().contours(level)
  }

  /// The shapes in the mask as smooth areas: each outline with the points it does not need dropped to within `tolerance` pixels, and curves
  /// through the rest that keep a corner where the outline turns by more than `corner_degrees`.
  pub fn traced(&self, level: u8, tolerance: f32, corner_degrees: f32) -> Vec<Arc<Area>> {
    self.0.read().unwrap().traced(level, tolerance, corner_degrees).into_iter().map(|area| Arc::new(Area::from_core(area))).collect()
  }

  /// A mask of lines `width` pixels thick along the edges of the shapes in this mask, the size of this mask: `255` on a line and `0`
  /// elsewhere. The lines follow `traced`. It is made in one go, so the points of the outlines never cross over one by one.
  pub fn outlined(&self, level: u8, tolerance: f32, corner_degrees: f32, width: f32) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().outlined(level, tolerance, corner_degrees, width)))
  }

  /// The fraction, from `0` to `1`, of the mask's pixels with a value above `threshold`.
  pub fn coverage(&self, threshold: u8) -> f32 {
    self.0.read().unwrap().coverage(threshold)
  }

  /// A copy of the part of the mask inside a rectangle, clipped to the mask. When none of it is on the mask the result has
  /// no pixels.
  pub fn cropped(&self, rect: Rect) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().cropped(rect)))
  }

  /// This mask placed onto a larger, empty mask of the given size, with its top left corner at `at`. The part outside the
  /// new mask is cut off. It puts a mask found in a crop of a picture back at the crop's place in the whole picture.
  pub fn placed(&self, width: u32, height: u32, at: PointF) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().placed(width, height, at)))
  }

  /// A copy of the mask with every value flipped: `255` becomes `0` and `0` becomes `255`.
  pub fn inverted(&self) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().inverted()))
  }

  /// A copy of the mask grown: each pixel takes the strongest value within `radius` pixels of it, so the edge of a region
  /// moves out that far. The window is a square and the edges of the mask are not eaten.
  pub fn dilated(&self, radius: u32) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().dilated(radius)))
  }

  /// A copy of the mask shrunk: each pixel takes the weakest value within `radius` pixels of it, so the edge of a region
  /// moves in that far. The window is a square and the edges of the mask are not eaten.
  pub fn eroded(&self, radius: u32) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().eroded(radius)))
  }

  /// A copy of the mask opened: shrunk and then grown by `radius`, so that specks and thin spurs smaller than the window are taken
  /// away while larger regions keep their size.
  pub fn opened(&self, radius: u32) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().opened(radius)))
  }

  /// A copy of the mask closed: grown and then shrunk by `radius`, so that holes and gaps smaller than the window are filled in while
  /// the outside of a region stays where it was.
  pub fn closed(&self, radius: u32) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().closed(radius)))
  }

  /// The brightness (`0` to `255`) below which the share `fraction` (`0` to `1`) of the pixels this mask covers fall in `image`, or `0`
  /// when the mask covers nothing. Pixels of the mask above `threshold` are covered. The image must be the size of the mask.
  pub fn brightness_percentile(&self, image: Arc<Image>, fraction: f32, threshold: u8) -> u8 {
    self.0.read().unwrap().brightness_percentile(&image.core(), fraction, threshold)
  }

  /// The one color that stands for the part of `image` this mask covers: the average, median or mode of those pixels, or nothing when the
  /// mask covers nothing. Pixels of the mask above `threshold` are covered. The image must be the size of the mask.
  pub fn color_in(&self, image: Arc<Image>, stat: ColorStat, threshold: u8) -> Option<Arc<Color>> {
    self.0.read().unwrap().color_in(&image.core(), stat, threshold).map(|color| Arc::new(Color(color)))
  }

  /// The two masks added, up to `255`. The result is the size of this mask; a mask of another size is scaled to it.
  pub fn added(&self, other: Arc<Mask>) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().added(&other.core())))
  }

  /// This mask with the other taken away, down to `0`: cuts one region out of another.
  pub fn subtracted(&self, other: Arc<Mask>) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().subtracted(&other.core())))
  }

  /// The two masks multiplied softly (`128` × `128` is `64`): keeps what both reach, such as the skin of one person.
  pub fn multiplied(&self, other: Arc<Mask>) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().multiplied(&other.core())))
  }

  /// The strongest of the two masks at each pixel: everything either one reaches.
  pub fn maximum(&self, other: Arc<Mask>) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().maximum(&other.core())))
  }

  /// The weakest of the two masks at each pixel: only what both reach.
  pub fn minimum(&self, other: Arc<Mask>) -> Arc<Mask> {
    Arc::new(Self::from_core(self.0.read().unwrap().minimum(&other.core())))
  }

  /// Sets the alpha of every pixel of the image to the mask value there: `255` is opaque and `0` is transparent. The mask
  /// must be the size of the image.
  pub fn apply_to_image(&self, image: Arc<Image>) {
    image.edit(|image| self.0.read().unwrap().apply_to_image(image));
  }

  /// Paints a fill onto the image only where this mask is on: a color, a gradient or a picture, as strong as the mask is. Where the mask
  /// is `255` the image gets the fill, where it is `0` it is as it was, and in between they are mixed. The fill starts at the top left
  /// corner of the image. The mask must be the size of the image.
  pub fn fill_image(&self, image: Arc<Image>, fill: Arc<Fill>) {
    let mask = self.0.read().unwrap().clone();
    image.edit(|image| mask.fill_image(image, fill.0.clone()));
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
  fn a_mask_gives_the_color_of_the_part_of_an_image_it_covers() {
    let image = Arc::new(Image::new_from_color(2, 1, Arc::new(Color::from_rgba(10, 20, 30, 255))));
    let covers_first = Mask::from_values(2, 1, vec![255, 0]);
    assert_eq!(covers_first.color_in(image.clone(), ColorStat::Average, 0).map(|color| color.rgba()), Some(vec![10, 20, 30, 255]));
    assert!(Mask::empty(2, 1).color_in(image, ColorStat::Median, 0).is_none());
  }

  #[test]
  fn a_mask_is_traced_and_outlined_in_one_call() {
    let values: Vec<u8> = (0..40 * 40).map(|at| if (10..30).contains(&(at % 40)) && (10..30).contains(&(at / 40)) { 255 } else { 0 }).collect();
    let mask = Mask::from_values(40, 40, values);
    let areas = mask.traced(128, 1.5, 50.0);
    assert_eq!(areas.len(), 1);
    assert!(areas[0].contains(crate::point_f_new(20.0, 20.0)));
    let lines = mask.outlined(128, 1.5, 50.0, 3.0);
    assert_eq!((lines.width(), lines.height()), (40, 40));
    assert_eq!(lines.get(10, 20), Some(255));
    assert_eq!(lines.get(20, 20), Some(0));
  }

  #[test]
  fn a_mask_gives_the_outlines_of_its_shapes() {
    let mask = Mask::from_values(4, 4, vec![0, 0, 0, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 0, 0, 0]);
    let outlines = mask.contours(128);
    assert_eq!(outlines.len(), 1);
    assert_eq!(outlines[0].len(), 8);
    assert!(Mask::empty(4, 4).contours(128).is_empty());
  }

  #[test]
  fn a_fill_is_painted_through_the_mask_onto_the_image() {
    let image = Arc::new(Image::new_from_pixels(2, 1, vec![10, 10, 10, 255, 10, 10, 10, 255], abra::abra_core::Channels::RGBA));
    Mask::from_values(2, 1, vec![0, 255]).fill_image(image.clone(), Arc::new(Fill::solid(Arc::new(Color::from_rgb(255, 0, 0)))));
    assert_eq!(image.rgba(), vec![10, 10, 10, 255, 255, 0, 0, 255]);
  }

  #[test]
  fn a_mask_is_made_from_a_brightness_range_cleaned_and_asked_for_a_percentile() {
    let grays: Vec<u8> = [0u8, 60, 120, 180, 240].iter().flat_map(|gray| [*gray, *gray, *gray, 255]).collect();
    let image = Arc::new(Image::new_from_pixels(5, 1, grays, abra::abra_core::Channels::RGBA));
    let midtones = Mask::from_brightness_range(image.clone(), 100, 200, 0);
    assert_eq!(midtones.values(), vec![0, 0, 255, 255, 0]);
    // Through the five grays from the darkest to the lightest: the start is the darkest, the middle the middle gray, the end the lightest.
    let everything = Mask::from_values(5, 1, vec![255; 5]);
    assert_eq!(everything.brightness_percentile(image.clone(), 0.0, 0), 0);
    assert_eq!(everything.brightness_percentile(image.clone(), 0.5, 0), 120);
    assert_eq!(everything.brightness_percentile(image, 1.0, 0), 240);
    // A speck is taken away by opening and a hole is filled by closing.
    let speck = Mask::from_values(9, 1, vec![0, 255, 0, 0, 255, 255, 255, 255, 255]);
    assert_eq!(speck.opened(1).values()[1], 0);
    assert_eq!(speck.closed(1).values()[2], 255);
  }

  #[test]
  fn a_mask_grows_and_shrinks_as_the_librarys_does() {
    let mask = Mask::from_values(5, 1, vec![0, 0, 255, 0, 0]);
    assert_eq!(mask.dilated(1).values(), vec![0, 255, 255, 255, 0]);
    assert_eq!(mask.dilated(1).eroded(1).values(), vec![0, 0, 255, 0, 0]);
  }

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
  fn masks_combine_through_the_bridge() {
    let a = Arc::new(Mask::from_values(2, 1, vec![100, 255]));
    let b = Arc::new(Mask::from_values(2, 1, vec![200, 128]));
    assert_eq!(a.inverted().values(), vec![155, 0]);
    assert_eq!(a.added(b.clone()).values(), vec![255, 255]);
    assert_eq!(a.subtracted(b.clone()).values(), vec![0, 127]);
    assert_eq!(a.multiplied(b.clone()).values(), vec![78, 128]);
    assert_eq!(a.maximum(b.clone()).values(), vec![200, 255]);
    assert_eq!(a.minimum(b).values(), vec![100, 128]);
  }

  #[test]
  fn a_mask_is_cropped_placed_and_measured_through_the_bridge() {
    let whole = Arc::new(Mask::from_values(4, 3, (0..12).collect()));
    let part = whole.cropped(Rect::new((1, 1), (2, 2)));
    assert_eq!(part.values(), vec![5, 6, 9, 10]);
    assert_eq!(part.placed(4, 3, PointF::new(1, 1)).values(), vec![0, 0, 0, 0, 0, 5, 6, 0, 0, 9, 10, 0]);
    let bounds = part.placed(4, 3, PointF::new(1, 1)).bounds(0).unwrap();
    assert_eq!((bounds.x, bounds.y, bounds.width, bounds.height), (1.0, 1.0, 2.0, 2.0));
    assert!(Mask::from_values(2, 2, vec![0; 4]).bounds(0).is_none());
  }

  #[test]
  fn a_circle_drawn_on_a_mask_sits_where_it_is_put() {
    let mask = Arc::new(Mask::from_values(40, 40, vec![0; 1600]));
    let white = Arc::new(Color(abra::abra_core::Color::from_rgb(255, 255, 255)));
    // A circle of radius 5 whose own box starts at its top left, put with that corner at (10, 20): centered on (15, 25).
    mask.draw_area(Arc::new(Area::circle(PointF::new(5, 5), 5.0)), white, Some(PointF::new(10, 20)));
    assert_eq!(mask.get(15, 25), Some(255));
    assert_eq!(mask.get(2, 2), Some(0));
    let bounds = mask.bounds(0).unwrap();
    assert!((bounds.x - 10.0).abs() <= 1.0 && (bounds.y - 20.0).abs() <= 1.0, "{bounds:?}");
  }

  #[test]
  fn a_color_range_mask_is_made_through_the_bridge() {
    let pixels = vec![220, 30, 30, 255, 30, 200, 30, 255];
    let image = Arc::new(Image::new_from_pixels(2, 1, pixels, abra::abra_core::Channels::RGBA));
    let red = Arc::new(Color(abra::abra_core::Color::red()));
    assert_eq!(Mask::from_color_range(image, red, 20.0, 0.4, 0.3, 0.0).values(), vec![255, 0]);
  }

  #[test]
  fn a_color_mix_mask_is_made_through_the_bridge() {
    let picture = |colors: &[u8]| Arc::new(Image::new_from_pixels(colors.len() as u32 / 4, 1, colors.to_vec(), abra::abra_core::Channels::RGBA));
    let image = picture(&[0, 0, 0, 255, 100, 100, 100, 255, 200, 200, 200, 255]);
    let foreground = picture(&[200, 200, 200, 255, 200, 200, 200, 255, 200, 200, 200, 255]);
    let background = picture(&[0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]);
    let mask = Mask::from_color_mix(image, foreground, background, Arc::new(Mask::from_values(3, 1, vec![0; 3])), 0.0);
    let values = mask.values();
    assert_eq!((values[0], values[2]), (0, 255));
    assert!((values[1] as i32 - 128).abs() <= 1);
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
