use std::borrow::Cow;
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
  fn from(p_rgb: (u8, u8, u8)) -> Self {
    Color {
      r: p_rgb.0,
      g: p_rgb.1,
      b: p_rgb.2,
      a: 255,
    }
  }
}

impl From<(u8, u8, u8, u8)> for Color {
  fn from(p_rgba: (u8, u8, u8, u8)) -> Self {
    Color {
      r: p_rgba.0,
      g: p_rgba.1,
      b: p_rgba.2,
      a: p_rgba.3,
    }
  }
}

impl<'a> Into<Cow<'a, Color>> for Color {
  fn into(self) -> Cow<'a, Color> {
    Cow::Owned(self)
  }
}

impl Display for Color {
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(p_f, "Color {{ r: {}, g: {}, b: {}, a: {} }}", self.r, self.g, self.b, self.a)
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

  /// Returns either black or white, whichever has higher contrast with the color.
  pub fn black_white_contrast(p_color: Color) -> Color {
    let black = Color::black();
    let white = Color::white();
    if p_color.contrast_ratio(black) > p_color.contrast_ratio(white) { black } else { white }
  }

  fn from_hsl_with_alpha(p_h: f32, p_s: f32, p_l: f32, p_alpha: u8) -> Self {
    Self {
      a: p_alpha,
      ..Self::from_hsl(p_h, p_s, p_l)
    }
  }

  fn from_hsv_with_alpha(p_h: f32, p_s: f32, p_v: f32, p_alpha: u8) -> Self {
    Self {
      a: p_alpha,
      ..Self::from_hsv(p_h, p_s, p_v)
    }
  }

  /// Creates a five-color analogous scheme around the given color.
  ///
  /// Analogous colors sit next to each other on the color wheel and produce a low-contrast,
  /// cohesive palette. The returned hues use offsets of -30, -15, 0, 15, and 30 degrees,
  /// placing the source color in the middle. Saturation, lightness, and alpha are preserved.
  pub fn analogous(p_color: Color) -> Vec<Color> {
    let (h, s, l) = p_color.hsl();
    [-30.0, -15.0, 0.0, 15.0, 30.0]
      .iter()
      .map(|offset| Self::from_hsl_with_alpha((h + offset).rem_euclid(360.0), s, l, p_color.a))
      .collect()
  }

  /// Returns the complementary color.
  ///
  /// Complementary colors are 180 degrees apart on the color wheel, producing the strongest
  /// hue contrast. Saturation, lightness, and alpha are preserved.
  pub fn complementary(p_color: Color) -> Color {
    let (h, s, l) = p_color.hsl();
    Self::from_hsl_with_alpha((h + 180.0) % 360.0, s, l, p_color.a)
  }

  /// Creates a five-color split-complementary theme around the given color.
  ///
  /// This follows the Adobe-style theme layout: the source color, the two hues on either side
  /// of its complement, then darker variants of the source and the clockwise complement.
  pub fn split_complementary(p_color: Color) -> Vec<Color> {
    let (h, s, v) = p_color.hsv();
    let clockwise = Self::from_hsv_with_alpha((h + 210.0).rem_euclid(360.0), s, v, p_color.a);
    let counterclockwise = Self::from_hsv_with_alpha((h + 150.0).rem_euclid(360.0), s, v, p_color.a);

    vec![
      p_color,
      clockwise,
      counterclockwise,
      Self::from_hsv_with_alpha(h, s * 0.5, v * 0.5, p_color.a),
      Self::from_hsv_with_alpha((h + 210.0).rem_euclid(360.0), s * 0.5, v * 0.5, p_color.a),
    ]
  }

  /// Returns the other two colors in the source color's triadic scheme.
  ///
  /// A triadic scheme places three colors 120 degrees apart on the color wheel. The returned
  /// colors are offset by 120 and 240 degrees, with saturation, lightness, and alpha preserved.
  pub fn triadic(p_color: Color) -> Vec<Color> {
    let (h, s, l) = p_color.hsl();
    let color1 = Self::from_hsl_with_alpha((h + 120.0) % 360.0, s, l, p_color.a);
    let color2 = Self::from_hsl_with_alpha((h + 240.0) % 360.0, s, l, p_color.a);
    vec![color1, color2]
  }

  /// Returns the other three colors in the source color's square scheme.
  ///
  /// A square scheme places four colors 90 degrees apart on the color wheel. The returned colors
  /// are offset by 90, 180, and 270 degrees, producing a broad balance of warm and cool hues.
  pub fn square(p_color: Color) -> Vec<Color> {
    let (h, s, l) = p_color.hsl();
    let color1 = Self::from_hsl_with_alpha((h + 90.0) % 360.0, s, l, p_color.a);
    let color2 = Self::from_hsl_with_alpha((h + 180.0) % 360.0, s, l, p_color.a);
    let color3 = Self::from_hsl_with_alpha((h + 270.0) % 360.0, s, l, p_color.a);
    vec![color1, color2, color3]
  }

  /// Returns the two colors in a compound scheme based on the source color.
  ///
  /// A compound scheme combines a neighboring hue at 30 degrees with its opposite at 210 degrees.
  /// It mixes the cohesion of an analogous scheme with the contrast of a complementary scheme.
  pub fn compound(p_color: Color) -> Vec<Color> {
    let (h, s, l) = p_color.hsl();
    let color1 = Self::from_hsl_with_alpha((h + 30.0) % 360.0, s, l, p_color.a);
    let color2 = Self::from_hsl_with_alpha((h + 210.0) % 360.0, s, l, p_color.a);
    vec![color1, color2]
  }

  /// Creates `p_out` progressively darker shades of the given color.
  ///
  /// A shade is formed by mixing a color with black. The first item is the source color and the
  /// final item contains 75% black, avoiding a pure-black endpoint. Alpha is preserved.
  pub fn shades(p_color: Color, p_out: usize) -> Vec<Color> {
    if p_out == 0 {
      return Vec::new();
    }

    (0..p_out)
      .map(|index| {
        let progress = if p_out == 1 { 0.0 } else { index as f32 / (p_out - 1) as f32 };
        let scale = 1.0 - progress * 0.75;
        Self::from_rgba(
          (p_color.r as f32 * scale).round() as u8,
          (p_color.g as f32 * scale).round() as u8,
          (p_color.b as f32 * scale).round() as u8,
          p_color.a,
        )
      })
      .collect()
  }

  /// Creates `p_out` monochromatic colors around the given color.
  ///
  /// Monochromatic colors share one hue and saturation while varying in lightness. The source
  /// color is placed between evenly distributed darker and lighter variants. Alpha is preserved.
  pub fn monochromatic(p_color: Color, p_out: usize) -> Vec<Color> {
    let (h, s, l) = p_color.hsl();
    if p_out == 0 {
      return Vec::new();
    }

    let darker_count = p_out / 2;
    let lighter_count = p_out - darker_count - 1;
    let darker = (1..=darker_count).map(|index| {
      let new_l = l * index as f32 / (darker_count + 1) as f32;
      Self::from_hsl_with_alpha(h, s, new_l, p_color.a)
    });
    let lighter = (1..=lighter_count).map(|index| {
      let new_l = l + (1.0 - l) * index as f32 / (lighter_count + 1) as f32;
      Self::from_hsl_with_alpha(h, s, new_l, p_color.a)
    });

    darker.chain(std::iter::once(p_color)).chain(lighter).collect()
  }

  /// Creates a black color.
  pub fn black() -> Self {
    Self::from_rgba(0, 0, 0, 255)
  }
  /// Creates a color from RGB values (alpha set to 255).
  pub fn from_rgb(p_r: u8, p_g: u8, p_b: u8) -> Self {
    Self {
      r: p_r,
      g: p_g,
      b: p_b,
      a: 255,
    }
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
  /// Creates a color from HSV values (alpha set to 255).
  pub fn from_hsv(p_h: f32, p_s: f32, p_v: f32) -> Self {
    let (r, g, b) = hsv_to_rgb(p_h, p_s, p_v);
    Self { r, g, b, a: 255 }
  }
  /// Creates a color from a hexadecimal value (alpha set to 255).
  pub fn from_hex(p_hex: u32) -> Self {
    Self {
      r: ((p_hex >> 16) & 0xFF) as u8,
      g: ((p_hex >> 8) & 0xFF) as u8,
      b: (p_hex & 0xFF) as u8,
      a: 255,
    }
  }
  /// Creates a color from a hexadecimal string (e.g., "#RRGGBB" or "#RRGGBBAA").
  pub fn from_hex_string(p_hex: &str) -> Self {
    let p_hex = p_hex.trim_start_matches('#');
    let hex_value = u32::from_str_radix(p_hex, 16).unwrap_or(0);
    match p_hex.len() {
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
  pub fn from_hsl(p_h: f32, p_s: f32, p_l: f32) -> Self {
    let (r, g, b) = hsl_to_rgb(p_h, p_s, p_l);
    Self { r, g, b, a: 255 }
  }
  /// Converts the color to a hexadecimal string (e.g., "#RRGGBB" or "#RRGGBBAA").
  pub fn to_hex_string(&self) -> String {
    if self.a == 255 {
      format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    } else {
      format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }
  }
  /// Calculates the contrast ratio between this color and another color.
  pub fn contrast_ratio(&self, p_other: Color) -> f32 {
    let l1 = self.luminance();
    let l2 = p_other.luminance();
    if l1 > l2 { (l1 + 0.05) / (l2 + 0.05) } else { (l2 + 0.05) / (l1 + 0.05) }
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
  /// Tan color using RGB(210, 180, 140)
  pub fn tan() -> Self {
    Self::from_rgb(210, 180, 140)
  }
  /// Purple color using RGB(128, 0, 128)
  pub fn purple() -> Self {
    Self::from_rgb(128, 0, 128)
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
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;
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

#[cfg(test)]
mod tests {
  use super::Color;

  #[test]
  fn analogous_green_stays_between_chartreuse_and_mint() {
    let colors: Vec<_> = Color::analogous(Color::green()).into_iter().map(|color| color.rgb()).collect();

    assert_eq!(colors, vec![(128, 255, 0), (64, 255, 0), (0, 255, 0), (0, 255, 64), (0, 255, 128)]);
  }

  #[test]
  fn harmony_colors_preserve_alpha() {
    let source = Color::from_rgba(255, 0, 0, 73);
    let mut colors = vec![Color::complementary(source)];
    colors.extend(Color::triadic(source));
    colors.extend(Color::square(source));
    colors.extend(Color::compound(source));

    let split_colors = Color::split_complementary(source);
    assert_eq!(split_colors.len(), 5);
    assert!(split_colors.iter().all(|color| color.a == source.a));
    assert!(colors.iter().all(|color| color.a == source.a));
    assert!(Color::analogous(source).iter().all(|color| color.a == source.a));
  }

  #[test]
  fn shades_mix_the_source_evenly_toward_black() {
    let source = Color::from_rgba(120, 255, 60, 73);
    let shades = Color::shades(source, 5);

    assert_eq!(shades.len(), 5);
    assert_eq!(shades[0].rgba(), source.rgba());
    assert_eq!(shades[4].rgba(), (30, 64, 15, 73));
    assert!(shades.windows(2).all(|pair| pair[0].luminance() > pair[1].luminance()));
  }

  #[test]
  fn monochromatic_colors_surround_and_include_source() {
    let source = Color::from_rgba(0, 255, 0, 73);
    let colors = Color::monochromatic(source, 5);

    assert_eq!(colors.len(), 5);
    assert_eq!(colors[2].rgba(), source.rgba());
    assert!(colors[0].hsl().2 < source.hsl().2);
    assert!(colors[4].hsl().2 > source.hsl().2);
    assert!(colors.iter().all(|color| color.a == source.a));
  }
}
