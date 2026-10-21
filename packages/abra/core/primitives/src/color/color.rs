use std::borrow::Cow;
use std::fmt::Display;

use super::luma::{LumaStandard, luma};
use super::to_hsl::rgb_to_hsl;
use super::to_hsv::rgb_to_hsv;
use super::to_lab::{rgb_to_lab, srgb_u8_to_linear_f32};
use super::to_rgb::{hsl_to_rgb, hsv_to_rgb, lab_to_rgb};

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
/// A color with red, green, blue, and alpha values.
pub struct Color {
  /// The red value of the color.
  /// A value between 0 and 255 representing the intensity of the red channel.
  pub r: u8,
  /// The green value of the color.
  /// A value between 0 and 255 representing the intensity of the green channel.
  pub g: u8,
  /// The blue value of the color.
  /// A value between 0 and 255 representing the intensity of the blue channel.
  pub b: u8,
  /// The alpha value of the color.
  /// A value between 0 and 255 representing the opacity of the color.
  pub a: u8,
}

impl From<(u8, u8, u8)> for Color {
  fn from(p_rgb: (u8, u8, u8)) -> Self {
    Color::from_rgb(p_rgb.0, p_rgb.1, p_rgb.2)
  }
}

impl From<(u8, u8, u8, u8)> for Color {
  fn from(p_rgba: (u8, u8, u8, u8)) -> Self {
    Color::from_rgba(p_rgba.0, p_rgba.1, p_rgba.2, p_rgba.3)
  }
}

impl<'a> From<Color> for Cow<'a, Color> {
  fn from(p_color: Color) -> Self {
    Cow::Owned(p_color)
  }
}

impl Display for Color {
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(p_f, "Color {{ r: {}, g: {}, b: {}, a: {} }}", self.r, self.g, self.b, self.a)
  }
}

impl Default for Color {
  /// Opaque black.
  fn default() -> Self {
    Color::black()
  }
}

/// A wrapper for color channel values, allowing conversion from `f32` and `u8`.
pub struct ColorNumber(u8);

impl From<f32> for ColorNumber {
  fn from(p_value: f32) -> Self {
    ColorNumber((p_value * 255.0) as u8)
  }
}

impl From<u8> for ColorNumber {
  fn from(p_value: u8) -> Self {
    ColorNumber(p_value)
  }
}

impl Color {
  /// Creates a color from RGB values (alpha set to 255).
  pub fn from_rgb(p_r: u8, p_g: u8, p_b: u8) -> Self {
    Self::from_rgba(p_r, p_g, p_b, 255)
  }
  /// Creates a color from RGBA values.
  pub fn from_rgba(p_r: u8, p_g: u8, p_b: u8, p_a: u8) -> Self {
    Self {
      r: p_r,
      g: p_g,
      b: p_b,
      a: p_a,
    }
  }
  /// Creates a color from HSV values (hue 0-360, saturation/value 0-1; alpha set to 255).
  pub fn from_hsv(p_h: f32, p_s: f32, p_v: f32) -> Self {
    let (r, g, b) = hsv_to_rgb(p_h, p_s, p_v);
    Self::from_rgb(r, g, b)
  }
  /// Creates a color from HSL values (hue 0-360, saturation/lightness 0-1; alpha set to 255).
  pub fn from_hsl(p_h: f32, p_s: f32, p_l: f32) -> Self {
    let (r, g, b) = hsl_to_rgb(p_h, p_s, p_l);
    Self::from_rgb(r, g, b)
  }
  /// Creates a color from CIE Lab values (L 0-100, a/b roughly -128 to 127; alpha set to 255).
  pub fn from_lab(p_l: f32, p_a: f32, p_b: f32) -> Self {
    let (r, g, b) = lab_to_rgb(p_l, p_a, p_b);
    Self::from_rgb(r, g, b)
  }
  /// Creates a color from a hexadecimal value (alpha set to 255).
  pub fn from_hex(p_hex: u32) -> Self {
    Self::from_rgb(((p_hex >> 16) & 0xFF) as u8, ((p_hex >> 8) & 0xFF) as u8, (p_hex & 0xFF) as u8)
  }
  /// Creates a color from a hexadecimal string (e.g., "#RRGGBB" or "#RRGGBBAA").
  ///
  /// Invalid strings return opaque black.
  pub fn from_hex_string(p_hex: &str) -> Self {
    let p_hex = p_hex.trim_start_matches('#');
    let hex_value = u32::from_str_radix(p_hex, 16).unwrap_or(0);
    match p_hex.len() {
      6 => Self::from_hex(hex_value),
      8 => Self::from_rgba(
        ((hex_value >> 24) & 0xFF) as u8,
        ((hex_value >> 16) & 0xFF) as u8,
        ((hex_value >> 8) & 0xFF) as u8,
        (hex_value & 0xFF) as u8,
      ),
      _ => Self::default(),
    }
  }
  /// Converts the color to a hexadecimal string (e.g., "#RRGGBB" or "#RRGGBBAA").
  pub fn to_hex_string(&self) -> String {
    if self.a == 255 {
      format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    } else {
      format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }
  }
  /// Sets the alpha value of the color.
  pub fn set_alpha(mut self, p_a: impl Into<ColorNumber>) -> Self {
    self.a = p_a.into().0;
    self
  }
  /// This color with its hue turned by some degrees, keeping its saturation, value and alpha. The hue wraps around the color wheel, so
  /// any number of degrees works, and a negative one turns the other way. A gray has no hue, so turning it changes nothing.
  /// - `p_degrees`: How far to turn the hue: `120` takes red to green, and `180` to cyan.
  pub fn with_hue_rotated(self, p_degrees: f32) -> Self {
    let (h, s, v) = self.hsv();
    Self::from_hsv((h + p_degrees).rem_euclid(360.0), s, v).set_alpha(self.a)
  }

  /// This color with at least this much saturation, keeping its hue, value and alpha. A color that already has more is unchanged. A gray
  /// has no hue, so raising its saturation tints it as a hue of `0` (red) would.
  /// - `p_saturation`: The least saturation, from 0 (gray) to 1 (the purest color of its hue).
  pub fn with_least_saturation(self, p_saturation: f32) -> Self {
    let (h, s, v) = self.hsv();
    Self::from_hsv(h, s.max(p_saturation), v).set_alpha(self.a)
  }

  /// This color with at least this much value (the brightness of its purest form: 0 is black and 1 is the brightest), keeping its hue,
  /// saturation and alpha. A color that already has more is unchanged.
  /// - `p_value`: The least value, from 0 to 1.
  pub fn with_least_value(self, p_value: f32) -> Self {
    let (h, s, v) = self.hsv();
    Self::from_hsv(h, s, v.max(p_value)).set_alpha(self.a)
  }

  /// Returns the RGB values of the color as a tuple.
  pub fn rgb(&self) -> (u8, u8, u8) {
    (self.r, self.g, self.b)
  }
  /// Returns the RGBA values of the color as a tuple.
  pub fn rgba(&self) -> (u8, u8, u8, u8) {
    (self.r, self.g, self.b, self.a)
  }
  /// Returns the HSL values of the color as a tuple (hue 0-360, saturation/lightness 0-1).
  pub fn hsl(&self) -> (f32, f32, f32) {
    rgb_to_hsl(self.r, self.g, self.b)
  }
  /// Returns the HSLA values of the color as a tuple (alpha 0-1).
  pub fn hsla(&self) -> (f32, f32, f32, f32) {
    let (h, s, l) = self.hsl();
    (h, s, l, self.a as f32 / 255.0)
  }
  /// Returns the HSV values of the color as a tuple (hue 0-360, saturation/value 0-1).
  pub fn hsv(&self) -> (f32, f32, f32) {
    rgb_to_hsv(self.r, self.g, self.b)
  }
  /// Returns the HSVA values of the color as a tuple (alpha 0-1).
  pub fn hsva(&self) -> (f32, f32, f32, f32) {
    let (h, s, v) = self.hsv();
    (h, s, v, self.a as f32 / 255.0)
  }
  /// Returns the CIE Lab values of the color as a tuple (D65 white point).
  pub fn lab(&self) -> (f32, f32, f32) {
    rgb_to_lab(self.r, self.g, self.b)
  }
  /// Calculates the WCAG relative luminance of the color (0 for black, 1 for white).
  ///
  /// This linearizes sRGB before weighting, so it measures physical light rather than
  /// the gamma-encoded [`luma`](super::luma::luma).
  pub fn luminance(&self) -> f32 {
    luma(
      srgb_u8_to_linear_f32(self.r),
      srgb_u8_to_linear_f32(self.g),
      srgb_u8_to_linear_f32(self.b),
      LumaStandard::Rec709,
    )
  }
  /// Calculates the WCAG contrast ratio between this color and another color (1 to 21).
  pub fn contrast_ratio(&self, p_other: Color) -> f32 {
    let l1 = self.luminance();
    let l2 = p_other.luminance();
    if l1 > l2 { (l1 + 0.05) / (l2 + 0.05) } else { (l2 + 0.05) / (l1 + 0.05) }
  }
  /// Returns either black or white, whichever has higher contrast with the color.
  pub fn black_white_contrast(p_color: Color) -> Color {
    let black = Color::black();
    let white = Color::white();
    if p_color.contrast_ratio(black) > p_color.contrast_ratio(white) { black } else { white }
  }
}

#[cfg(test)]
mod tests {
  use super::Color;

  #[test]
  fn the_hue_turns_around_the_wheel_and_the_rest_is_kept() {
    let red = Color::from_rgba(255, 0, 0, 90);
    assert_eq!(red.with_hue_rotated(120.0).rgba(), (0, 255, 0, 90));
    assert_eq!(red.with_hue_rotated(240.0).rgba(), (0, 0, 255, 90));
    // A full turn, and a turn back, come to the same place, and a negative turn goes the other way.
    assert_eq!(red.with_hue_rotated(360.0).rgba(), red.rgba());
    assert_eq!(red.with_hue_rotated(-120.0).rgba(), red.with_hue_rotated(240.0).rgba());
    assert_eq!(red.with_hue_rotated(480.0).rgba(), red.with_hue_rotated(120.0).rgba());
    // A paler, darker color keeps its saturation and value as the hue moves.
    let (_, s, v) = Color::from_rgb(200, 100, 100).hsv();
    let (h2, s2, v2) = Color::from_rgb(200, 100, 100).with_hue_rotated(90.0).hsv();
    assert!((s - s2).abs() < 0.01 && (v - v2).abs() < 0.01 && (h2 - 90.0).abs() < 2.0, "{h2} {s2} {v2}");
    // A gray has no hue to turn.
    assert_eq!(Color::from_rgb(120, 120, 120).with_hue_rotated(90.0).rgb(), (120, 120, 120));
  }

  #[test]
  fn saturation_and_value_are_raised_to_a_least_and_never_lowered() {
    let muted = Color::from_rgba(150, 110, 100, 200);
    let (h, s, v) = muted.hsv();
    let (h2, s2, v2) = muted.with_least_saturation(0.9).hsv();
    assert!((h - h2).abs() < 2.0 && (v - v2).abs() < 0.01 && s2 > 0.88, "{h2} {s2} {v2}");
    assert_eq!(muted.with_least_saturation(0.9).a, 200);
    let (h3, s3, v3) = muted.with_least_value(0.95).hsv();
    assert!((h - h3).abs() < 2.0 && (s - s3).abs() < 0.01 && v3 > 0.94, "{h3} {s3} {v3}");
    assert_eq!(muted.with_least_value(0.95).a, 200);
    // A color that has more already is as it was.
    let vivid = Color::from_rgba(250, 40, 30, 77);
    assert_eq!(vivid.with_least_saturation(0.5).rgba(), vivid.rgba());
    assert_eq!(vivid.with_least_value(0.5).rgba(), vivid.rgba());
  }

  #[test]
  fn contrast_ratio_matches_wcag() {
    assert!((Color::black().contrast_ratio(Color::white()) - 21.0).abs() < 1e-3);
    assert!((Color::red().contrast_ratio(Color::red()) - 1.0).abs() < 1e-6);
  }

  #[test]
  fn color_space_round_trips() {
    let color = Color::from_rgb(200, 90, 30);
    let (h, s, l) = color.hsl();
    assert_eq!(Color::from_hsl(h, s, l), color);
    let (h, s, v) = color.hsv();
    assert_eq!(Color::from_hsv(h, s, v), color);
    let (l, a, b) = color.lab();
    assert_eq!(Color::from_lab(l, a, b), color);
  }

  #[test]
  fn hex_string_round_trips() {
    let color = Color::from_rgba(1, 2, 3, 4);
    assert_eq!(Color::from_hex_string(&color.to_hex_string()), color);
    assert_eq!(Color::from_hex_string("#0A0B0C"), Color::from_rgb(10, 11, 12));
  }
}
