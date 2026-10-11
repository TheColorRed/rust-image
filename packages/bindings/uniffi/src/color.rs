//! The color module of `primitives`: `Color`, `Harmony`, `luma`, and the value histograms `Bins` and `Histogram`.
//!
//! What cannot cross to other languages, and what stands in for it:
//! - A tuple becomes an array: `rgb()` is `[r, g, b]`, `hsla()` is `[h, s, l, a]`, and so on.
//! - `set_alpha` takes `impl Into<ColorNumber>`, which is a whole number 0-255 or a fraction 0-1: they are `set_alpha` and
//!   `set_alpha_fraction`.
//! - `Harmony::Shades(usize)` and `Monochromatic(usize)` count with a `u32` here.
//! - `Channel::RGB`/`RGBA` and `Histogram::channel_mut` (a borrow into the histogram) are not exported; the first is a list
//!   a caller writes, and `Histogram::channel` returns a copy.

use std::sync::{Arc, Mutex};

use abra::abra_core::{self, Channel, Channels, ColorStat, LumaStandard};

/// A color with red, green, blue and alpha, each 0 to 255.
#[derive(Debug, PartialEq, uniffi::Object)]
pub struct Color(pub abra_core::Color);

// A statistic used to reduce many pixels to a single color.
#[uniffi::remote(Enum)]
pub enum ColorStat {
  Average,
  Median,
  Mode,
}

// The weighting standard used to compute luma (perceived brightness) from RGB.
#[uniffi::remote(Enum)]
pub enum LumaStandard {
  Rec601,
  Rec709,
}

/// A color harmony (color scheme) that can be generated from a source color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum Harmony {
  /// Five neighboring hues at -30, -15, 0, 15, and 30 degrees, with the source in the middle.
  Analogous,
  /// The source and the hue 180 degrees opposite it.
  Complementary,
  /// The source, the hues on either side of its complement, then darker variants.
  SplitComplementary,
  /// The source and the hues 120 and 240 degrees away.
  Triadic,
  /// The source and the hues 90, 180, and 270 degrees away.
  Square,
  /// The source, a neighboring hue at 30 degrees, and that neighbor's opposite at 210 degrees.
  Compound,
  /// The given number of progressively darker shades, starting at the source and ending at 75% black.
  Shades(u32),
  /// The given number of lightness variants of the source's hue and saturation, with the source in the middle.
  Monochromatic(u32),
}

impl From<Harmony> for abra_core::Harmony {
  fn from(harmony: Harmony) -> Self {
    match harmony {
      Harmony::Analogous => abra_core::Harmony::Analogous,
      Harmony::Complementary => abra_core::Harmony::Complementary,
      Harmony::SplitComplementary => abra_core::Harmony::SplitComplementary,
      Harmony::Triadic => abra_core::Harmony::Triadic,
      Harmony::Square => abra_core::Harmony::Square,
      Harmony::Compound => abra_core::Harmony::Compound,
      Harmony::Shades(count) => abra_core::Harmony::Shades(count as usize),
      Harmony::Monochromatic(count) => abra_core::Harmony::Monochromatic(count as usize),
    }
  }
}

#[uniffi::export]
impl Color {
  /// A color from RGB values (alpha 255).
  #[uniffi::constructor]
  pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
    Self(abra_core::Color::from_rgb(r, g, b))
  }

  /// A color from RGBA values.
  #[uniffi::constructor]
  pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
    Self(abra_core::Color::from_rgba(r, g, b, a))
  }

  /// A color from HSV values (hue 0-360, saturation and value 0-1; alpha 255).
  #[uniffi::constructor]
  pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
    Self(abra_core::Color::from_hsv(h, s, v))
  }

  /// A color from HSL values (hue 0-360, saturation and lightness 0-1; alpha 255).
  #[uniffi::constructor]
  pub fn from_hsl(h: f32, s: f32, l: f32) -> Self {
    Self(abra_core::Color::from_hsl(h, s, l))
  }

  /// A color from CIE Lab values (L 0-100, a and b roughly -128 to 127; alpha 255).
  #[uniffi::constructor]
  pub fn from_lab(l: f32, a: f32, b: f32) -> Self {
    Self(abra_core::Color::from_lab(l, a, b))
  }

  /// A color from a hexadecimal value (alpha 255).
  #[uniffi::constructor]
  pub fn from_hex(hex: u32) -> Self {
    Self(abra_core::Color::from_hex(hex))
  }

  /// A color from a hexadecimal string such as "#RRGGBB" or "#RRGGBBAA". An invalid string gives opaque black.
  #[uniffi::constructor]
  pub fn from_hex_string(hex: String) -> Self {
    Self(abra_core::Color::from_hex_string(&hex))
  }

  /// The one color that stands for many pixels (`pixels` laid out as `channels`): their average, median or mode.
  #[uniffi::constructor]
  pub fn from_pixels(pixels: Vec<u8>, channels: Channels, stat: ColorStat) -> Self {
    Self(abra_core::Color::from_pixels(&pixels, channels, stat))
  }

  /// Black or white, whichever reads better on `color`.
  #[uniffi::constructor]
  pub fn black_white_contrast(color: Arc<Color>) -> Self {
    Self(abra_core::Color::black_white_contrast(color.0))
  }

  /// This color with its alpha set to `a`, 0 to 255.
  pub fn set_alpha(&self, a: u8) -> Arc<Color> {
    Arc::new(Color(self.0.set_alpha(a)))
  }

  /// This color with its alpha set to `a`, a fraction from 0 to 1.
  pub fn set_alpha_fraction(&self, a: f32) -> Arc<Color> {
    Arc::new(Color(self.0.set_alpha(a)))
  }

  /// The color as "#RRGGBB", or "#RRGGBBAA" when it is not opaque.
  pub fn to_hex_string(&self) -> String {
    self.0.to_hex_string()
  }

  /// Red, green and blue: `[r, g, b]`.
  pub fn rgb(&self) -> Vec<u8> {
    let (r, g, b) = self.0.rgb();
    vec![r, g, b]
  }

  /// Red, green, blue and alpha: `[r, g, b, a]`.
  pub fn rgba(&self) -> Vec<u8> {
    let (r, g, b, a) = self.0.rgba();
    vec![r, g, b, a]
  }

  /// Hue, saturation and lightness: `[h, s, l]`.
  pub fn hsl(&self) -> Vec<f32> {
    let (h, s, l) = self.0.hsl();
    vec![h, s, l]
  }

  /// Hue, saturation, lightness and alpha: `[h, s, l, a]`.
  pub fn hsla(&self) -> Vec<f32> {
    let (h, s, l, a) = self.0.hsla();
    vec![h, s, l, a]
  }

  /// Hue, saturation and value: `[h, s, v]`.
  pub fn hsv(&self) -> Vec<f32> {
    let (h, s, v) = self.0.hsv();
    vec![h, s, v]
  }

  /// Hue, saturation, value and alpha: `[h, s, v, a]`.
  pub fn hsva(&self) -> Vec<f32> {
    let (h, s, v, a) = self.0.hsva();
    vec![h, s, v, a]
  }

  /// CIE Lab: `[l, a, b]`.
  pub fn lab(&self) -> Vec<f32> {
    let (l, a, b) = self.0.lab();
    vec![l, a, b]
  }

  /// The relative luminance of the color.
  pub fn luminance(&self) -> f32 {
    self.0.luminance()
  }

  /// The WCAG contrast ratio between this color and `other`.
  pub fn contrast_ratio(&self, other: Arc<Color>) -> f32 {
    self.0.contrast_ratio(other.0)
  }

  /// A color scheme built from this color. Every scheme includes this color.
  pub fn harmony(&self, harmony: Harmony) -> Vec<Arc<Color>> {
    self.0.harmony(harmony.into()).into_iter().map(|color| Arc::new(Color(color))).collect()
  }
}

// The named colors (`Color.red()` and so on), written by build.rs from the library's list of them.
include!(concat!(env!("OUT_DIR"), "/color_names.rs"));

/// The luma (perceived brightness) of a color given as red, green and blue, with the weights of `standard`.
#[uniffi::export]
pub fn luma(r: f32, g: f32, b: f32, standard: LumaStandard) -> f32 {
  abra_core::luma(r, g, b, standard)
}

/// The red, green and blue weights `standard` gives to a color: `[r, g, b]`. They sum to 1.
#[uniffi::export]
pub fn luma_weights(standard: LumaStandard) -> Vec<f32> {
  let (r, g, b) = standard.weights();
  vec![r, g, b]
}

/// How many times each value 0 to 255 occurs in one channel of an image.
#[derive(Debug, uniffi::Object)]
pub struct Bins(Mutex<abra_core::Bins>);

impl Bins {
  fn core(&self) -> abra_core::Bins {
    self.0.lock().unwrap().clone()
  }
}

#[uniffi::export]
impl Bins {
  /// Empty bins.
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self(Mutex::new(abra_core::Bins::new()))
  }

  /// Counts one more of `value`.
  pub fn add(&self, value: u8) {
    self.0.lock().unwrap().add(value);
  }

  /// Counts one less of `value`.
  pub fn remove(&self, value: u8) {
    self.0.lock().unwrap().remove(value);
  }

  /// Adds the counts of `other`.
  pub fn merge(&self, other: Arc<Bins>) {
    // `other` is copied first, so merging bins into themselves does not wait on its own lock.
    let other = other.core();
    self.0.lock().unwrap().merge(&other);
  }

  /// Empties the bins.
  pub fn clear(&self) {
    self.0.lock().unwrap().clear();
  }

  /// The count of every value, 256 of them.
  pub fn counts(&self) -> Vec<u64> {
    self.0.lock().unwrap().counts().to_vec()
  }

  /// How many values are counted.
  pub fn total(&self) -> u64 {
    self.0.lock().unwrap().total()
  }

  /// The value below which `fraction` (0 to 1) of the counted values fall.
  pub fn percentile(&self, fraction: f32) -> u8 {
    self.0.lock().unwrap().percentile(fraction)
  }

  /// The middle value.
  pub fn median(&self) -> u8 {
    self.0.lock().unwrap().median()
  }

  /// The mean value.
  pub fn mean(&self) -> u8 {
    self.0.lock().unwrap().mean()
  }

  /// The most common value.
  pub fn mode(&self) -> u8 {
    self.0.lock().unwrap().mode()
  }

  /// The mean of the values within `threshold` of `center`.
  pub fn weighted_average(&self, center: u8, threshold: u8) -> u8 {
    self.0.lock().unwrap().weighted_average(center, threshold)
  }

  /// The lowest and highest value left after clipping `clip_fraction` of the counts from each end: `[low, high]`.
  pub fn clip_bounds(&self, clip_fraction: f32) -> Vec<u8> {
    let (low, high) = self.0.lock().unwrap().clip_bounds(clip_fraction);
    vec![low, high]
  }

  /// A 256-entry table that stretches the range left after clipping `clip_fraction` from each end to the full range.
  pub fn levels_lut(&self, clip_fraction: f32) -> Vec<u8> {
    self.0.lock().unwrap().levels_lut(clip_fraction).to_vec()
  }
}

/// The value histograms of the red, green, blue and alpha channels of an image.
#[derive(Debug, uniffi::Object)]
pub struct Histogram(Mutex<abra_core::Histogram>);

#[uniffi::export]
impl Histogram {
  /// An empty histogram.
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self(Mutex::new(abra_core::Histogram::new()))
  }

  /// A histogram of the channels of every pixel of an RGBA buffer, leaving out fully transparent pixels if asked.
  #[uniffi::constructor]
  pub fn from_rgba(rgba: Vec<u8>, skip_transparent: bool) -> Self {
    Self(Mutex::new(abra_core::Histogram::from_rgba(&rgba, skip_transparent)))
  }

  /// A copy of the bins of one channel. Changing the copy does not change the histogram.
  pub fn channel(&self, channel: Channel) -> Arc<Bins> {
    Arc::new(Bins(Mutex::new(self.0.lock().unwrap().channel(channel).clone())))
  }

  /// Counts one pixel.
  pub fn add_rgb(&self, r: u8, g: u8, b: u8) {
    self.0.lock().unwrap().add_rgb(r, g, b);
  }

  /// Empties the histogram.
  pub fn clear(&self) {
    self.0.lock().unwrap().clear();
  }

  /// How many pixels are counted.
  pub fn total_pixels(&self) -> u64 {
    self.0.lock().unwrap().total_pixels()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn named_colors_are_the_librarys_colors() {
    assert_eq!(Color::red().0, abra_core::Color::red());
    assert_eq!(Color::transparent().0, abra_core::Color::transparent());
  }

  #[test]
  fn a_hex_string_round_trips() {
    assert_eq!(Color::from_hex_string("#102030".to_string()).to_hex_string(), "#102030");
    assert_eq!(Color::from_rgba(1, 2, 3, 4).to_hex_string(), "#01020304");
  }

  #[test]
  fn color_spaces_come_back_as_arrays() {
    let color = Color::from_rgba(10, 20, 30, 40);
    assert_eq!(color.rgb(), vec![10, 20, 30]);
    assert_eq!(color.rgba(), vec![10, 20, 30, 40]);
    let (h, s, l) = color.0.hsl();
    assert_eq!(color.hsl(), vec![h, s, l]);
    let (h, s, v, a) = color.0.hsva();
    assert_eq!(color.hsva(), vec![h, s, v, a]);
    assert_eq!(color.hsv().len() + color.hsla().len() + color.lab().len(), 3 + 4 + 3);
  }

  #[test]
  fn alpha_is_set_as_a_number_or_as_a_fraction() {
    let color = Color::from_rgb(1, 2, 3);
    assert_eq!(color.set_alpha(100).0.a, 100);
    assert_eq!(color.set_alpha_fraction(0.5).0.a, 127);
  }

  #[test]
  fn contrast_and_pixels_and_harmony_match_the_library() {
    let white = Arc::new(Color::white());
    assert_eq!(
      Color::black().contrast_ratio(white.clone()),
      abra_core::Color::black().contrast_ratio(abra_core::Color::white())
    );
    assert_eq!(
      Color::black_white_contrast(Arc::new(Color::white())).0,
      abra_core::Color::black_white_contrast(abra_core::Color::white())
    );
    let pixels = vec![10, 20, 30, 255, 30, 40, 50, 255];
    assert_eq!(Color::from_pixels(pixels, Channels::RGBA, ColorStat::Average).rgba(), vec![20, 30, 40, 255]);
    assert_eq!(Color::red().harmony(Harmony::Triadic).len(), 3);
    assert_eq!(Color::red().harmony(Harmony::Shades(4)).len(), 4);
  }

  #[test]
  fn luma_uses_the_standards_weights() {
    let weights = luma_weights(LumaStandard::Rec709);
    assert_eq!(weights.len(), 3);
    assert!((weights.iter().sum::<f32>() - 1.0).abs() < 1e-5);
    assert_eq!(luma(1.0, 0.0, 0.0, LumaStandard::Rec709), weights[0]);
  }

  #[test]
  fn bins_count_and_report() {
    let bins = Bins::new();
    for value in [10, 10, 20] {
      bins.add(value);
    }
    assert_eq!(bins.total(), 3);
    assert_eq!(bins.mode(), 10);
    assert_eq!(bins.counts()[10], 2);
    bins.remove(10);
    assert_eq!(bins.counts()[10], 1);
    let other = Arc::new(Bins::new());
    other.add(200);
    bins.merge(other);
    assert_eq!(bins.total(), 3);
    bins.merge(Arc::new(Bins::new()));
    // Clipping nothing leaves the whole range; clipping a third from each end leaves the values counted in the middle.
    assert_eq!(bins.clip_bounds(0.0), vec![0, 255]);
    let mut library_bins = abra_core::Bins::new();
    for value in [10, 20, 200] {
      library_bins.add(value);
    }
    let (low, high) = library_bins.clip_bounds(0.34);
    assert_eq!(bins.clip_bounds(0.34), vec![low, high]);
    assert_eq!(bins.levels_lut(0.0).len(), 256);
    bins.clear();
    assert_eq!(bins.total(), 0);
  }

  #[test]
  fn merging_bins_into_themselves_does_not_lock_up() {
    let bins = Arc::new(Bins::new());
    bins.add(5);
    bins.merge(bins.clone());
    assert_eq!(bins.total(), 2);
  }

  #[test]
  fn a_histogram_counts_channels_and_hands_out_copies() {
    let histogram = Histogram::from_rgba(vec![1, 2, 3, 255, 0, 0, 0, 0], true);
    assert_eq!(histogram.total_pixels(), 1);
    let red = histogram.channel(Channel::R);
    assert_eq!(red.total(), 1);
    red.add(9);
    assert_eq!(histogram.channel(Channel::R).total(), 1, "changing the copy leaves the histogram alone");
    histogram.add_rgb(4, 5, 6);
    assert_eq!(histogram.total_pixels(), 2);
    histogram.clear();
    assert_eq!(histogram.total_pixels(), 0);
  }
}
