use std::fmt::Display;

use crate::color::to_rgb::hsl_to_rgb;
use crate::color::to_rgb::hsv_to_rgb;

use super::to_hsl::rgb_to_hsl;
use super::to_hsv::rgb_to_hsv;

#[derive(Clone, Debug)]
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
    Self { r: 0, g: 0, b: 0, a: 255 }
  }

  /// Creates the color red with an rgb value of (255, 0, 0).
  pub fn red() -> Self {
    Self {
      r: 255,
      g: 0,
      b: 0,
      a: 255,
    }
  }

  /// Creates the color green with an rgb value of (0, 255, 0).
  pub fn green() -> Self {
    Self {
      r: 0,
      g: 255,
      b: 0,
      a: 255,
    }
  }

  /// Creates the color blue with an rgb value of (0, 0, 255).
  pub fn blue() -> Self {
    Self {
      r: 0,
      g: 0,
      b: 255,
      a: 255,
    }
  }

  /// Creates the color yellow with an rgb value of (255, 255, 0).
  pub fn yellow() -> Self {
    Self {
      r: 255,
      g: 255,
      b: 0,
      a: 255,
    }
  }

  /// Creates the color orange with an rgb value of (255, 127, 0).
  pub fn orange() -> Self {
    Self {
      r: 255,
      g: 127,
      b: 0,
      a: 255,
    }
  }

  /// Creates the color indigo with an rgb value of (75, 0, 130).
  pub fn indigo() -> Self {
    Self {
      r: 75,
      g: 0,
      b: 130,
      a: 255,
    }
  }

  /// Creates the color violet with an rgb value of (148, 0, 211).
  pub fn violet() -> Self {
    Self {
      r: 148,
      g: 0,
      b: 211,
      a: 255,
    }
  }

  /// Creates the color white with an rgb value of (255, 255, 255).
  pub fn white() -> Self {
    Self {
      r: 255,
      g: 255,
      b: 255,
      a: 255,
    }
  }

  /// Creates the color black with an rgb value of (0, 0, 0).
  pub fn black() -> Self {
    Self { r: 0, g: 0, b: 0, a: 255 }
  }

  /// Creates the color gray with an rgb value of (128, 128, 128).
  pub fn gray() -> Self {
    Self {
      r: 128,
      g: 128,
      b: 128,
      a: 255,
    }
  }

  /// Creates the color transparent with an rgba value of (0, 0, 0, 0).
  pub fn transparent() -> Self {
    Self { r: 0, g: 0, b: 0, a: 0 }
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
}
