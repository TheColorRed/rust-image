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

impl From<(u8, u8, u8)> for Color {
  fn from(rgb: (u8, u8, u8)) -> Self {
    Color {
      r: rgb.0,
      g: rgb.1,
      b: rgb.2,
      a: 255,
    }
  }
}

impl From<(u8, u8, u8, u8)> for Color {
  fn from(rgba: (u8, u8, u8, u8)) -> Self {
    Color {
      r: rgba.0,
      g: rgba.1,
      b: rgba.2,
      a: rgba.3,
    }
  }
}

impl Display for Color {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "Color {{ r: {}, g: {}, b: {}, a: {} }}", self.r, self.g, self.b, self.a)
  }
}

impl Color {
  pub fn default() -> Self {
    Self {
      r: 0,
      g: 0,
      b: 0,
      a: 255,
    }
  }
  /// Creates a black color.
  pub fn black() -> Self {
    Self::from_rgba(0, 0, 0, 255)
  }
  /// Creates a color from RGB values (alpha set to 255).
  pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
    Self { r, g, b, a: 255 }
  }
  /// Creates a color from RGBA values.
  pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
    Self { r, g, b, a }
  }
  /// Creates a color from HSV values (alpha set to 255).
  pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
    let (r, g, b) = hsv_to_rgb(h, s, v);
    Self { r, g, b, a: 255 }
  }
  /// Creates a color from a hexadecimal value (alpha set to 255).
  pub fn from_hex(hex: u32) -> Self {
    Self {
      r: ((hex >> 16) & 0xFF) as u8,
      g: ((hex >> 8) & 0xFF) as u8,
      b: (hex & 0xFF) as u8,
      a: 255,
    }
  }
  /// Creates a color from a hexadecimal string (e.g., "#RRGGBB" or "#RRGGBBAA").
  pub fn from_hex_string(hex: &str) -> Self {
    let hex = hex.trim_start_matches('#');
    let hex_value = u32::from_str_radix(hex, 16).unwrap_or(0);
    match hex.len() {
      6 => Self::from_hex(hex_value),
      8 => Self {
        r: ((hex_value >> 24) & 0xFF) as u8,
        g: ((hex_value >> 16) & 0xFF) as u8,
        b: ((hex_value >> 8) & 0xFF) as u8,
        a: (hex_value & 0xFF) as u8,
      },
      _ => Self::default(),
    }
  }
  /// Creates a color from HSL values (alpha set to 255).
  pub fn from_hsl(h: f32, s: f32, l: f32) -> Self {
    let (r, g, b) = hsl_to_rgb(h, s, l);
    Self { r, g, b, a: 255 }
  }
  /// Calculates the contrast ratio between this color and another color.
  pub fn contrast_ratio(&self, other: Color) -> f32 {
    let l1 = self.luminance();
    let l2 = other.luminance();
    if l1 > l2 {
      (l1 + 0.05) / (l2 + 0.05)
    } else {
      (l2 + 0.05) / (l1 + 0.05)
    }
  }
  /// Returns the RGB values of the color as a tuple.
  pub fn rgb(&self) -> (u8, u8, u8) {
    (self.r, self.g, self.b)
  }
  /// Returns the RGBA values of the color as a tuple.
  pub fn rgba(&self) -> (u8, u8, u8, u8) {
    (self.r, self.g, self.b, self.a)
  }
  /// Returns the HSL values of the color as a tuple.
  pub fn hsl(&self) -> (f32, f32, f32) {
    let hsl = rgb_to_hsl(self.r, self.g, self.b);
    (hsl.0, hsl.1, hsl.2)
  }
  /// Returns the HSLA values of the color as a tuple.
  pub fn hsla(&self) -> (f32, f32, f32, f32) {
    let hsl = rgb_to_hsl(self.r, self.g, self.b);
    (hsl.0, hsl.1, hsl.2, self.a as f32 / 255.0)
  }
  /// Returns the HSV values of the color as a tuple.
  pub fn hsv(&self) -> (f32, f32, f32) {
    let hsv = rgb_to_hsv(self.r, self.g, self.b);
    (hsv.0, hsv.1, hsv.2)
  }
  /// Returns the HSVA values of the color as a tuple.
  pub fn hsva(&self) -> (f32, f32, f32, f32) {
    let hsv = rgb_to_hsv(self.r, self.g, self.b);
    (hsv.0, hsv.1, hsv.2, self.a as f32 / 255.0)
  }
  /// Calculates the luminance of the color.
  pub fn luminance(&self) -> f32 {
    let (r, g, b) = self.rgb();
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0
  }
  /// Calculates the average color from a slice of colors represented as u8 values.
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
  /// Calculates the mean color from a slice of colors represented as u8 values.
  pub fn mean(p_colors: &[u8]) -> Self {
    Color::average(p_colors)
  }
  /// Calculates the median color from a slice of colors represented as u8 values.
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
  /// Calculates the mode color from a slice of colors represented as u8 values.
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
  // Common colors for convenience (matching previous abra_core::Color API)
  /// A transparent color using RGBA(0, 0, 0, 0)
  pub fn transparent() -> Self {
    Self::from_rgba(0, 0, 0, 0)
  }
  /// Red color using RGB(255, 0, 0)
  pub fn red() -> Self {
    Self::from_rgb(255, 0, 0)
  }
  /// Crimson color using RGB(220, 20, 60)
  pub fn crimson() -> Self {
    Self::from_rgb(220, 20, 60)
  }
  /// Coral color using RGB(255, 127, 80)
  pub fn ruby() -> Self {
    Self::from_rgb(224, 17, 95)
  }
  /// Pink color using RGB(255, 192, 203)
  pub fn pink() -> Self {
    Self::from_rgb(255, 192, 203)
  }
  /// Magenta color using RGB(255, 0, 255)
  pub fn magenta() -> Self {
    Self::from_rgb(255, 0, 255)
  }
  /// Hot pink color using RGB(255, 105, 180)
  pub fn hot_pink() -> Self {
    Self::from_rgb(255, 105, 180)
  }
  /// Green color using RGB(0, 255, 0)
  pub fn green() -> Self {
    Self::from_rgb(0, 255, 0)
  }
  /// Lime green color using RGB(50, 205, 50)
  pub fn lime_green() -> Self {
    Self::from_rgb(50, 205, 50)
  }
  /// Sea green color using RGB(46, 139, 87)
  pub fn sea_green() -> Self {
    Self::from_rgb(46, 139, 87)
  }
  /// Forest green color using RGB(34, 139, 34)
  pub fn forest_green() -> Self {
    Self::from_rgb(34, 139, 34)
  }
  /// Blue color using RGB(0, 0, 255)
  pub fn blue() -> Self {
    Self::from_rgb(0, 0, 255)
  }
  /// Royal blue color using RGB(65, 105, 225)
  pub fn royal_blue() -> Self {
    Self::from_rgb(65, 105, 225)
  }
  /// Sky blue color using RGB(135, 206, 235)
  pub fn sky_blue() -> Self {
    Self::from_rgb(135, 206, 235)
  }
  /// Navy blue color using RGB(0, 0, 128)
  pub fn navy_blue() -> Self {
    Self::from_rgb(0, 0, 128)
  }
  /// Yellow color using RGB(255, 255, 0)
  pub fn yellow() -> Self {
    Self::from_rgb(255, 255, 0)
  }
  /// Orange color using RGB(255, 165, 0)
  pub fn orange() -> Self {
    Self::from_rgb(255, 165, 0)
  }
  /// Indigo color using RGB(75, 0, 130)
  pub fn indigo() -> Self {
    Self::from_rgb(75, 0, 130)
  }
  /// Violet color using RGB(238, 130, 238)
  pub fn violet() -> Self {
    Self::from_rgb(238, 130, 238)
  }
  /// White color using RGB(255, 255, 255)
  pub fn white() -> Self {
    Self::from_rgb(255, 255, 255)
  }
  /// Gray color using RGB(128, 128, 128)
  pub fn gray() -> Self {
    Self::from_rgb(128, 128, 128)
  }
  /// Random color generator (alpha=255)
  pub fn random() -> Self {
    // Lightweight LCG seeded from current system time to avoid adding rand dependency.
    let nanos = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos() as u64;
    let mut x: u64 = nanos.wrapping_mul(6364136223846793005).wrapping_add(1);
    x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
    let r = (x >> 24) as u8;
    x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
    let g = (x >> 32) as u8;
    x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
    let b = (x >> 16) as u8;
    Self::from_rgba(r, g, b, 255)
  }
}
