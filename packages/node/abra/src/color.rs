use crate::common::*;
use abra::abra_core::Color as AbraColor;

macro_rules! color_factories {
  ($($name:ident),*) => {
    #[napi]
    impl Color {
      $(
        #[napi(factory)]
        /// Creates a new color with the name of the function.
        pub fn $name() -> Self {
          Self { inner: AbraColor::$name() }
        }
      )*
    }
  };
}

#[napi]
#[derive(Clone)]
pub struct Color {
  pub(crate) inner: AbraColor,
}

#[napi]
impl Color {
  #[napi(constructor)]
  pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
    Self {
      inner: AbraColor { r, g, b, a },
    }
  }
  #[napi(factory)]
  /// Creates a new color from RGB values with full opacity.
  pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
    Color::new(r, g, b, 255)
  }
  #[napi(factory)]
  /// Creates a new color from RGBA values.
  pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
    Color::new(r, g, b, a)
  }
  #[napi(factory)]
  /// Creates a new color from a hex value (e.g., 0xRRG, 0xRRGGBBAA).
  pub fn from_hex(hex: u32) -> Self {
    AbraColor::from_hex(hex).into()
  }
  #[napi(factory)]
  /// Creates a new color from a hex string (e.g., "#RRGGBB" or "#RRGGBBAA").
  pub fn from_hex_string(hex: String) -> Self {
    AbraColor::from_hex_string(&hex).into()
  }
  #[napi(factory)]
  /// Creates a new color from HSL values (e.g., hue, saturation, lightness).
  pub fn from_hsl(h: f64, s: f64, l: f64) -> Self {
    AbraColor::from_hsl(h as f32, s as f32, l as f32).into()
  }
  #[napi(factory)]
  /// Creates a new color from HSV values (e.g., hue, saturation, value).
  pub fn from_hsv(h: f64, s: f64, v: f64) -> Self {
    AbraColor::from_hsv(h as f32, s as f32, v as f32).into()
  }

  #[napi(getter)]
  /// Returns the red component of the color.
  /// @return The red component (0-255).
  pub fn r(&self) -> u8 {
    self.inner.r
  }
  #[napi]
  /// Returns the green component of the color.
  /// @return The green component (0-255).
  pub fn g(&self) -> u8 {
    self.inner.g
  }
  #[napi(getter)]
  /// Returns the blue component of the color.
  /// @return The blue component (0-255).
  pub fn b(&self) -> u8 {
    self.inner.b
  }
  #[napi(getter)]
  /// Returns the alpha component of the color.
  /// @return The alpha component (0-255).
  pub fn a(&self) -> u8 {
    self.inner.a
  }
  #[napi]
  /// Calculates the contrast ratio between this color and another color.
  /// @param other The other color to compare with.
  /// @return The contrast ratio as a float.
  pub fn contrast_ratio(&self, other: &Color) -> f64 {
    self.inner.contrast_ratio(other.inner) as f64
  }
  #[napi]
  /// Returns the average of this color and another color.
  /// @param other The other color to average with.
  /// @return The resulting average color.
  pub fn average(colors: &[u8]) -> Color {
    AbraColor::average(colors).into()
  }
}

impl From<AbraColor> for Color {
  fn from(inner: AbraColor) -> Self {
    Self { inner }
  }
}

impl From<(u8, u8, u8, u8)> for Color {
  fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
    Self {
      inner: AbraColor { r, g, b, a },
    }
  }
}

impl From<(u8, u8, u8)> for Color {
  fn from((r, g, b): (u8, u8, u8)) -> Self {
    Self {
      inner: AbraColor { r, g, b, a: 255 },
    }
  }
}

color_factories!(
  transparent,
  red,
  crimson,
  ruby,
  pink,
  magenta,
  hot_pink,
  green,
  lime_green,
  sea_green,
  forest_green,
  blue,
  royal_blue,
  sky_blue,
  navy_blue,
  yellow,
  orange,
  indigo,
  violet,
  white,
  gray,
  random,
  black
);
