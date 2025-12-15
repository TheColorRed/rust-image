use std::fmt::Display;

use rayon::prelude::*;

use crate::color::to_rgb::hsl_to_rgb;
use crate::color::to_rgb::hsv_to_rgb;

use super::to_hsl::rgb_to_hsl;
use super::to_hsv::rgb_to_hsv;

#[derive(Clone, Debug, Copy)]
/// A color with red, green, blue, and alpha values.
pub struct Color {
  /// The red value of the color.
  pub r: u8,
  /// The green value of the color.
  pub g: u8,
  /// The blue value of the color.
  pub b: u8,
  /// The alpha value of the color.
  pub a: u8,
}

impl Display for Color {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "Color {{ r: {}, g: {}, b: {}, a: {} }}", self.r, self.g, self.b, self.a)
  }
}

impl Color {
  /// Creates a new color with the default values (black).\
  /// `Color::default()` creates a black color.
  pub fn default() -> Self {
    Self {
      r: 0,
      g: 0,
      b: 0,
      a: 255,
    }
  }
  /// Creates a new color with the given RGB values and defaults an alpha value to 255.\
  /// `Color::from_rgb(255, 0, 0)` creates a red color.
  pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
    Self { r, g, b, a: 255 }
  }
  /// Creates a new color with the given RGBA values.\
  /// `Color::from_rgba(255, 0, 0, 255)` creates a red color.\
  /// `Color::from_rgba(255, 0, 0, 128)` creates a red color with 50% opacity.
  pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
    Self { r, g, b, a }
  }
  /// Creates a new color with the given HSV values.\
  /// `Color::from_hsv(0.0, 1.0, 1.0)` creates a red color.\
  /// `Color::from_hsv(0.0, 0.5, 0.5)` creates a dark red color.
  pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
    let (r, g, b) = hsv_to_rgb(h, s, v);
    Self { r, g, b, a: 255 }
  }
  /// Creates a new color with the given hex value.\
  /// `Color::from_hex(0xFF0000)` creates a red color.
  pub fn from_hex(hex: u32) -> Self {
    Self {
      r: ((hex >> 16) & 0xFF) as u8,
      g: ((hex >> 8) & 0xFF) as u8,
      b: (hex & 0xFF) as u8,
      a: 255,
    }
  }
  /// Creates a new color with the given HSL values.\
  /// `Color::from_hsl(0.0, 1.0, 0.5)` creates a red color.\
  /// `Color::from_hsl(0.0, 0.5, 0.25)` creates a dark red color.
  pub fn from_hsl(h: f32, s: f32, l: f32) -> Self {
    let (r, g, b) = hsl_to_rgb(h, s, l);
    Self { r, g, b, a: 255 }
  }
  /// Calculates the contrast ratio between two colors.\
  /// The contrast ratio is a value between 1 and 21.\
  /// A ratio of 1 means the colors are the same.\
  /// A ratio of 21 means the colors are the most different.
  pub fn contrast_ratio(&self, other: Color) -> f32 {
    let l1 = self.luminance();
    let l2 = other.luminance();
    if l1 > l2 {
      (l1 + 0.05) / (l2 + 0.05)
    } else {
      (l2 + 0.05) / (l1 + 0.05)
    }
  }
  /// Returns the RGB values of the color.
  pub fn rgb(&self) -> (u8, u8, u8) {
    (self.r, self.g, self.b)
  }
  /// Returns the RGBA values of the color.
  pub fn rgba(&self) -> (u8, u8, u8, u8) {
    (self.r, self.g, self.b, self.a)
  }
  /// Returns the HSL values of the color.
  pub fn hsl(&self) -> (f32, f32, f32) {
    let hsl = rgb_to_hsl(self.r, self.g, self.b);
    (hsl.0, hsl.1, hsl.2)
  }
  /// Returns the HSLA values of the color.
  pub fn hsla(&self) -> (f32, f32, f32, f32) {
    let hsl = rgb_to_hsl(self.r, self.g, self.b);
    (hsl.0, hsl.1, hsl.2, self.a as f32 / 255.0)
  }
  /// Returns the HSV values of the color.
  pub fn hsv(&self) -> (f32, f32, f32) {
    let hsv = rgb_to_hsv(self.r, self.g, self.b);
    (hsv.0, hsv.1, hsv.2)
  }
  /// Returns the HSVA values of the color.
  pub fn hsva(&self) -> (f32, f32, f32, f32) {
    let hsv = rgb_to_hsv(self.r, self.g, self.b);
    (hsv.0, hsv.1, hsv.2, self.a as f32 / 255.0)
  }
  /// Returns the luminance of the color.\
  /// The luminance is a value between 0 and 1.
  pub fn luminance(&self) -> f32 {
    let (r, g, b) = self.rgb();
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0
  }
  /// Returns the color as a tuple of u8 values (r, g, b, a).
  pub fn as_u8(&self) -> (u8, u8, u8, u8) {
    (self.r, self.g, self.b, self.a)
  }
  /// Creates a color that is the average of the provided colors.
  /// Average is calculated by summing each channel and dividing by the number of colors for that channel.
  /// - `p_colors`: A slice of u8 values representing colors in RGB/RGBA format.
  pub fn average(p_colors: &[u8]) -> Self {
    let len = p_colors.len() as u32;
    let is_rgba = len % 4 == 0;
    let chunks = if is_rgba { 4 } else { 3 };
    let channel_count = if is_rgba { len / 4 } else { len / 3 };
    let (r, g, b) = p_colors
      .par_chunks(chunks)
      .fold(
        || (0u32, 0u32, 0u32),
        |(mut r_acc, mut g_acc, mut b_acc), chunk| {
          r_acc += chunk[0] as u32;
          g_acc += chunk[1] as u32;
          b_acc += chunk[2] as u32;
          (r_acc, g_acc, b_acc)
        },
      )
      .reduce(|| (0u32, 0u32, 0u32), |(r1, g1, b1), (r2, g2, b2)| (r1 + r2, g1 + g2, b1 + b2));
    Self {
      r: (r / channel_count) as u8,
      g: (g / channel_count) as u8,
      b: (b / channel_count) as u8,
      a: 255,
    }
  }
  /// Synonym for `Color::average`.
  /// Mean is calculated by summing each channel and dividing by the number of colors for that channel.
  /// - `p_colors`: A slice of u8 values representing colors in RGB/RGBA format.
  pub fn mean(p_colors: &[u8]) -> Self {
    Color::average(p_colors)
  }
  /// Creates a color that is the median of the provided colors.
  /// Median is calculated by sorting each channel independently and selecting the middle value from each channel.
  /// - `p_colors`: A slice of u8 values representing colors in RGB/RGBA format.
  pub fn median(p_colors: &[u8]) -> Self {
    let len = p_colors.len() as u32;
    let is_rgba = len % 4 == 0;
    let chunks = if is_rgba { 4 } else { 3 };
    let mut r_values: Vec<u8> = Vec::new();
    let mut g_values: Vec<u8> = Vec::new();
    let mut b_values: Vec<u8> = Vec::new();

    for chunk in p_colors.chunks(chunks) {
      r_values.push(chunk[0]);
      g_values.push(chunk[1]);
      b_values.push(chunk[2]);
    }

    r_values.sort_unstable();
    g_values.sort_unstable();
    b_values.sort_unstable();

    let mid = (len / 3) as usize / 2;

    Self {
      r: r_values[mid],
      g: g_values[mid],
      b: b_values[mid],
      a: 255,
    }
  }
  /// Creates a color that is the mode of the provided colors.
  /// Mode is calculated by finding the most frequently occurring value in each channel independently.
  /// - `p_colors`: A slice of u8 values representing colors in RGB/RGBA format.
  pub fn mode(p_colors: &[u8]) -> Self {
    let len = p_colors.len() as u32;
    let is_rgba = len % 4 == 0;
    let chunks = if is_rgba { 4 } else { 3 };
    let mut r_counts = std::collections::HashMap::new();
    let mut g_counts = std::collections::HashMap::new();
    let mut b_counts = std::collections::HashMap::new();

    for chunk in p_colors.chunks(chunks) {
      *r_counts.entry(chunk[0]).or_insert(0) += 1;
      *g_counts.entry(chunk[1]).or_insert(0) += 1;
      *b_counts.entry(chunk[2]).or_insert(0) += 1;
    }

    let r_mode = *r_counts.iter().max_by_key(|&(_, count)| count).unwrap().0;
    let g_mode = *g_counts.iter().max_by_key(|&(_, count)| count).unwrap().0;
    let b_mode = *b_counts.iter().max_by_key(|&(_, count)| count).unwrap().0;

    Self {
      r: r_mode,
      g: g_mode,
      b: b_mode,
      a: 255,
    }
  }
}
