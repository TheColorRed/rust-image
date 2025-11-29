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
  pub fn default() -> Self {
    Self {
      r: 0,
      g: 0,
      b: 0,
      a: 255,
    }
  }
  pub fn black() -> Self {
    Self::from_rgba(0, 0, 0, 255)
  }
  pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
    Self { r, g, b, a: 255 }
  }
  pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
    Self { r, g, b, a }
  }
  pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
    let (r, g, b) = hsv_to_rgb(h, s, v);
    Self { r, g, b, a: 255 }
  }
  pub fn from_hex(hex: u32) -> Self {
    Self {
      r: ((hex >> 16) & 0xFF) as u8,
      g: ((hex >> 8) & 0xFF) as u8,
      b: (hex & 0xFF) as u8,
      a: 255,
    }
  }
  pub fn from_hsl(h: f32, s: f32, l: f32) -> Self {
    let (r, g, b) = hsl_to_rgb(h, s, l);
    Self { r, g, b, a: 255 }
  }
  pub fn contrast_ratio(&self, other: Color) -> f32 {
    let l1 = self.luminance();
    let l2 = other.luminance();
    if l1 > l2 {
      (l1 + 0.05) / (l2 + 0.05)
    } else {
      (l2 + 0.05) / (l1 + 0.05)
    }
  }
  pub fn rgb(&self) -> (u8, u8, u8) {
    (self.r, self.g, self.b)
  }
  pub fn rgba(&self) -> (u8, u8, u8, u8) {
    (self.r, self.g, self.b, self.a)
  }
  pub fn hsl(&self) -> (f32, f32, f32) {
    let hsl = rgb_to_hsl(self.r, self.g, self.b);
    (hsl.0, hsl.1, hsl.2)
  }
  pub fn hsla(&self) -> (f32, f32, f32, f32) {
    let hsl = rgb_to_hsl(self.r, self.g, self.b);
    (hsl.0, hsl.1, hsl.2, self.a as f32 / 255.0)
  }
  pub fn hsv(&self) -> (f32, f32, f32) {
    let hsv = rgb_to_hsv(self.r, self.g, self.b);
    (hsv.0, hsv.1, hsv.2)
  }
  pub fn hsva(&self) -> (f32, f32, f32, f32) {
    let hsv = rgb_to_hsv(self.r, self.g, self.b);
    (hsv.0, hsv.1, hsv.2, self.a as f32 / 255.0)
  }
  pub fn luminance(&self) -> f32 {
    let (r, g, b) = self.rgb();
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0
  }
  pub fn as_u8(&self) -> (u8, u8, u8, u8) {
    (self.r, self.g, self.b, self.a)
  }
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
  pub fn mean(p_colors: &[u8]) -> Self {
    Color::average(p_colors)
  }
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
  pub fn transparent() -> Self {
    Self::from_rgba(0, 0, 0, 0)
  }
  pub fn red() -> Self {
    Self::from_rgb(255, 0, 0)
  }
  pub fn crimson() -> Self {
    Self::from_rgb(220, 20, 60)
  }
  pub fn ruby() -> Self {
    Self::from_rgb(224, 17, 95)
  }
  pub fn pink() -> Self {
    Self::from_rgb(255, 192, 203)
  }
  pub fn magenta() -> Self {
    Self::from_rgb(255, 0, 255)
  }
  pub fn hot_pink() -> Self {
    Self::from_rgb(255, 105, 180)
  }
  pub fn green() -> Self {
    Self::from_rgb(0, 128, 0)
  }
  pub fn lime_green() -> Self {
    Self::from_rgb(50, 205, 50)
  }
  pub fn sea_green() -> Self {
    Self::from_rgb(46, 139, 87)
  }
  pub fn forest_green() -> Self {
    Self::from_rgb(34, 139, 34)
  }
  pub fn blue() -> Self {
    Self::from_rgb(0, 0, 255)
  }
  pub fn royal_blue() -> Self {
    Self::from_rgb(65, 105, 225)
  }
  pub fn sky_blue() -> Self {
    Self::from_rgb(135, 206, 235)
  }
  pub fn navy_blue() -> Self {
    Self::from_rgb(0, 0, 128)
  }
  pub fn yellow() -> Self {
    Self::from_rgb(255, 255, 0)
  }
  pub fn orange() -> Self {
    Self::from_rgb(255, 165, 0)
  }
  pub fn indigo() -> Self {
    Self::from_rgb(75, 0, 130)
  }
  pub fn violet() -> Self {
    Self::from_rgb(238, 130, 238)
  }
  pub fn white() -> Self {
    Self::from_rgb(255, 255, 255)
  }
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
