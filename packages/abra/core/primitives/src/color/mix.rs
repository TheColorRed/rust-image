//! Mixing colors in a color space of your choice, and the color spaces to do it in.
//!
//! A mix of two colors is not one thing: halfway between a blue and a yellow is a dull gray if the red, green and blue values are
//! averaged, and a clean green if the colors are averaged in Oklab, which is built to follow what the eye sees. [`Color::mix`] takes the
//! space to mix in, and follows CSS Color 4 (`color-mix()` and gradients) so that its results are what a designer expects:
//!
//! - alpha is premultiplied, so a color fading in from transparent does not tint what it fades from;
//! - the hue of a gray is "powerless" in a polar space, so a gray mixed with a color takes that color's hue and does not sweep through
//!   the wheel;
//! - which way round the wheel a hue goes is a [`HueDirection`].

use super::Color;
use super::to_lab::{rgb_to_lab, srgb_u8_to_linear_f32};
use super::to_oklab::{from_polar, oklab_to_linear_rgb, rgb_to_oklab, to_polar};
use super::to_rgb::{hsl_to_unit_rgb, hsv_to_unit_rgb, lab_to_linear_rgb, linear_to_srgb};

/// A color space to mix colors in, or to read a color in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorSpace {
  /// The red, green and blue values as they are stored (gamma encoded). The plain average of the channels, and the default.
  #[default]
  Srgb,
  /// Red, green and blue as light, which is how light adds: halfway between black and white is brighter than in [`ColorSpace::Srgb`].
  LinearSrgb,
  /// Hue, saturation and lightness. A polar space: the hue is an angle.
  Hsl,
  /// Hue, saturation and value. A polar space: the hue is an angle.
  Hsv,
  /// CIE Lab (D65), lightness and two color axes. Close to how the eye sees, but blues drift toward purple.
  Lab,
  /// CIE Lab as lightness, chroma and hue. A polar space.
  Lch,
  /// Oklab, lightness and two color axes. Mixing in it is smooth and keeps the brightness of the mix between the two colors.
  Oklab,
  /// Oklab as lightness, chroma and hue. A polar space: turning the hue keeps how bright the color looks.
  Oklch,
}

impl ColorSpace {
  /// Whether one of its three components is a hue, an angle that wraps around the color wheel.
  pub fn is_polar(self) -> bool {
    matches!(self, ColorSpace::Hsl | ColorSpace::Hsv | ColorSpace::Lch | ColorSpace::Oklch)
  }

  /// Where the hue is among the three components, and where the component that is zero for a gray is: `(hue, chroma)`, and how small the
  /// chroma may be for the hue to count as powerless.
  fn hue_and_chroma(self) -> Option<(usize, usize, f32)> {
    match self {
      ColorSpace::Hsl | ColorSpace::Hsv => Some((0, 1, 1e-6)),
      ColorSpace::Lch => Some((2, 1, 0.02)),
      ColorSpace::Oklch => Some((2, 1, 5e-4)),
      _ => None,
    }
  }
}

/// Which way round the color wheel a hue goes when mixing two colors in a polar space. The same as CSS's `<hue-interpolation-method>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HueDirection {
  /// The shorter of the two ways round, which is less than half a turn. The default.
  #[default]
  Shorter,
  /// The longer of the two ways round, which is more than half a turn.
  Longer,
  /// The way the hue grows (red to yellow to green), going once round if it has to.
  Increasing,
  /// The way the hue shrinks (red to magenta to blue), going once round if it has to.
  Decreasing,
}

impl HueDirection {
  /// The direction that goes the same way back: a mix from the second color to the first.
  pub fn reversed(self) -> Self {
    match self {
      HueDirection::Shorter => HueDirection::Shorter,
      HueDirection::Longer => HueDirection::Longer,
      HueDirection::Increasing => HueDirection::Decreasing,
      HueDirection::Decreasing => HueDirection::Increasing,
    }
  }
}

/// How two colors are mixed: the space, and for a polar space which way round the wheel the hue goes. A [`ColorSpace`] converts into one
/// with the shorter way round, so `Color::mix(other, 0.5, ColorSpace::Oklab)` works.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ColorInterpolation {
  /// The space the colors are mixed in.
  pub space: ColorSpace,
  /// Which way round the wheel the hue goes. Not used by a space with no hue.
  pub hue: HueDirection,
}

impl ColorInterpolation {
  /// Mixing in a space, the shorter way round the color wheel.
  pub fn new(p_space: ColorSpace) -> Self {
    Self {
      space: p_space,
      hue: HueDirection::Shorter,
    }
  }

  /// Sets which way round the color wheel the hue goes.
  pub fn with_hue(mut self, p_hue: HueDirection) -> Self {
    self.hue = p_hue;
    self
  }

  /// The same mixing, for going from the second color to the first.
  pub fn reversed(self) -> Self {
    Self {
      space: self.space,
      hue: self.hue.reversed(),
    }
  }
}

impl From<ColorSpace> for ColorInterpolation {
  fn from(p_space: ColorSpace) -> Self {
    ColorInterpolation::new(p_space)
  }
}

/// The three components of a color in a space, as they are stored in a byte color, so there is no rounding on the way in.
fn components(p_color: &Color, p_space: ColorSpace) -> [f32; 3] {
  let (r, g, b) = p_color.rgb();
  match p_space {
    ColorSpace::Srgb => [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0],
    ColorSpace::LinearSrgb => [srgb_u8_to_linear_f32(r), srgb_u8_to_linear_f32(g), srgb_u8_to_linear_f32(b)],
    ColorSpace::Hsl => {
      let (h, s, l) = p_color.hsl();
      [h, s, l]
    }
    ColorSpace::Hsv => {
      let (h, s, v) = p_color.hsv();
      [h, s, v]
    }
    ColorSpace::Lab => {
      let (l, a, b) = rgb_to_lab(r, g, b);
      [l, a, b]
    }
    ColorSpace::Lch => {
      let (l, a, b) = rgb_to_lab(r, g, b);
      let (l, c, h) = to_polar(l, a, b);
      [l, c, h]
    }
    ColorSpace::Oklab => {
      let (l, a, b) = rgb_to_oklab(r, g, b);
      [l, a, b]
    }
    ColorSpace::Oklch => {
      let (l, a, b) = rgb_to_oklab(r, g, b);
      let (l, c, h) = to_polar(l, a, b);
      [l, c, h]
    }
  }
}

/// A color given as three components of a space, as sRGB channels from 0 to 1, with no clamping: a color outside the gamut has a channel
/// outside 0 to 1.
fn to_srgb(p_components: [f32; 3], p_space: ColorSpace) -> [f32; 3] {
  let [x, y, z] = p_components;
  let encode = |(r, g, b): (f32, f32, f32)| [linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b)];
  match p_space {
    ColorSpace::Srgb => [x, y, z],
    ColorSpace::LinearSrgb => encode((x, y, z)),
    ColorSpace::Hsl => {
      let (r, g, b) = hsl_to_unit_rgb(x.rem_euclid(360.0), y.clamp(0.0, 1.0), z.clamp(0.0, 1.0));
      [r, g, b]
    }
    ColorSpace::Hsv => {
      let (r, g, b) = hsv_to_unit_rgb(x.rem_euclid(360.0), y.clamp(0.0, 1.0), z.clamp(0.0, 1.0));
      [r, g, b]
    }
    ColorSpace::Lab => encode(lab_to_linear_rgb(x, y, z)),
    ColorSpace::Lch => {
      let (l, a, b) = from_polar(x, y.max(0.0), z);
      encode(lab_to_linear_rgb(l, a, b))
    }
    ColorSpace::Oklab => encode(oklab_to_linear_rgb(x, y, z)),
    ColorSpace::Oklch => {
      let (l, a, b) = from_polar(x, y.max(0.0), z);
      encode(oklab_to_linear_rgb(l, a, b))
    }
  }
}

/// Moves two hues, in degrees, so that a straight line from the first to the second goes the way `p_direction` says.
fn fix_hues(p_first: f32, p_second: f32, p_direction: HueDirection) -> (f32, f32) {
  let (mut first, mut second) = (p_first.rem_euclid(360.0), p_second.rem_euclid(360.0));
  let difference = second - first;
  match p_direction {
    HueDirection::Shorter => {
      if difference > 180.0 {
        first += 360.0;
      } else if difference < -180.0 {
        second += 360.0;
      }
    }
    HueDirection::Longer => {
      if difference > 0.0 && difference < 180.0 {
        first += 360.0;
      } else if difference > -180.0 && difference <= 0.0 {
        second += 360.0;
      }
    }
    HueDirection::Increasing => {
      if second < first {
        second += 360.0;
      }
    }
    HueDirection::Decreasing => {
      if first < second {
        first += 360.0;
      }
    }
  }
  (first, second)
}

impl Color {
  /// This color mixed with another, `p_amount` of the way from this color to the other.
  ///
  /// The colors are mixed in the space of `p_interpolation`, following CSS Color 4. Alpha is premultiplied, so a color that is mostly
  /// transparent counts for less, and a color mixed with a transparent one does not tint. In a polar space the hue of a gray (or of a
  /// color too pale to have one) is powerless: it takes the other color's hue, so a gray mixed with a red does not sweep through the
  /// wheel on the way. The result is rounded to bytes once, at the end. A mix that is outside the sRGB gamut, which can happen in Oklab,
  /// Lab and their polar forms, is clipped to it channel by channel.
  ///
  /// Note that the amount is how far toward `p_other`, so `0` is this color and `1` is the other, as a gradient's position is. (Sass's
  /// `color.mix` weight is the share of the first color, so it is `1 - p_amount`.)
  ///
  /// - `p_other`: The color to mix with.
  /// - `p_amount`: How far from this color to `p_other`. It is not limited to 0 to 1, but alpha is, so going past either end gives colors
  ///   that are further along the same line.
  /// - `p_interpolation`: The space to mix in, and for a polar space which way round the wheel the hue goes. A [`ColorSpace`] mixes the
  ///   shorter way round.
  ///
  /// ```ignore
  /// let green = Color::blue().mix(Color::yellow(), 0.5, ColorSpace::Oklab);
  /// let rainbow = Color::red().mix(Color::blue(), 0.5, ColorInterpolation::new(ColorSpace::Hsl).with_hue(HueDirection::Longer));
  /// ```
  pub fn mix(&self, p_other: Color, p_amount: f32, p_interpolation: impl Into<ColorInterpolation>) -> Color {
    let interpolation: ColorInterpolation = p_interpolation.into();
    let space = interpolation.space;
    let (mut first, mut second) = (components(self, space), components(&p_other, space));
    let (alpha_first, alpha_second) = (self.a as f32 / 255.0, p_other.a as f32 / 255.0);

    let mut hue = None;
    if let Some((hue_at, chroma_at, powerless_below)) = space.hue_and_chroma() {
      // A hue with no chroma behind it is no hue: it takes the other color's, and two of them are both zero.
      let first_powerless = first[chroma_at] <= powerless_below;
      let second_powerless = second[chroma_at] <= powerless_below;
      if first_powerless && !second_powerless {
        first[hue_at] = second[hue_at];
      } else if second_powerless && !first_powerless {
        second[hue_at] = first[hue_at];
      } else if first_powerless && second_powerless {
        first[hue_at] = 0.0;
        second[hue_at] = 0.0;
      }
      let (from, to) = fix_hues(first[hue_at], second[hue_at], interpolation.hue);
      hue = Some((hue_at, from + (to - from) * p_amount));
    }

    let alpha = alpha_first + (alpha_second - alpha_first) * p_amount;
    let mut mixed = [0.0f32; 3];
    for channel in 0..3 {
      if hue.is_some_and(|(hue_at, _)| hue_at == channel) {
        continue;
      }
      // Premultiplied: the channel counts as much as the color is opaque, then it is divided out of the mix.
      let (from, to) = (first[channel] * alpha_first, second[channel] * alpha_second);
      let premultiplied = from + (to - from) * p_amount;
      mixed[channel] = if alpha > 0.0 {
        premultiplied / alpha
      } else {
        first[channel] + (second[channel] - first[channel]) * p_amount
      };
    }
    if let Some((hue_at, value)) = hue {
      mixed[hue_at] = value;
    }

    let srgb = to_srgb(mixed, space);
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::from_rgba(byte(srgb[0]), byte(srgb[1]), byte(srgb[2]), byte(alpha))
  }

  /// Returns the Oklab values of the color: `(L, a, b)` with `L` from 0 (black) to 1 (white) and `a` and `b` about -0.4 to 0.4.
  pub fn oklab(&self) -> (f32, f32, f32) {
    rgb_to_oklab(self.r, self.g, self.b)
  }

  /// Returns the Oklch values of the color: `(L, chroma, hue)` with `L` from 0 to 1, chroma from 0 (a gray) to about 0.4, and the hue in
  /// degrees from 0 to 360. The hue of a gray is 0.
  pub fn oklch(&self) -> (f32, f32, f32) {
    let (l, a, b) = self.oklab();
    to_polar(l, a, b)
  }

  /// Returns the CIE Lch values of the color: `(L, chroma, hue)` with `L` from 0 to 100, and the hue in degrees from 0 to 360. The hue of
  /// a gray is 0.
  pub fn lch(&self) -> (f32, f32, f32) {
    let (l, a, b) = self.lab();
    to_polar(l, a, b)
  }

  /// Creates a color from Oklab values (`L` 0-1, `a` and `b` about -0.4 to 0.4; alpha set to 255). A color outside the sRGB gamut is
  /// clipped to it.
  pub fn from_oklab(p_l: f32, p_a: f32, p_b: f32) -> Self {
    let (r, g, b) = super::to_oklab::oklab_to_rgb(p_l, p_a, p_b);
    Self::from_rgb(r, g, b)
  }

  /// Creates a color from Oklch values (`L` 0-1, chroma from 0 to about 0.4, hue in degrees; alpha set to 255). A color outside the sRGB
  /// gamut is clipped to it.
  pub fn from_oklch(p_l: f32, p_chroma: f32, p_hue: f32) -> Self {
    let (l, a, b) = from_polar(p_l, p_chroma, p_hue);
    Self::from_oklab(l, a, b)
  }

  /// Creates a color from CIE Lch values (`L` 0-100, chroma from 0 to about 130, hue in degrees; alpha set to 255). A color outside the
  /// sRGB gamut is clipped to it.
  pub fn from_lch(p_l: f32, p_chroma: f32, p_hue: f32) -> Self {
    let (l, a, b) = from_polar(p_l, p_chroma, p_hue);
    Self::from_lab(l, a, b)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const SPACES: [ColorSpace; 8] = [
    ColorSpace::Srgb,
    ColorSpace::LinearSrgb,
    ColorSpace::Hsl,
    ColorSpace::Hsv,
    ColorSpace::Lab,
    ColorSpace::Lch,
    ColorSpace::Oklab,
    ColorSpace::Oklch,
  ];

  fn colors() -> Vec<Color> {
    vec![
      Color::from_rgb(255, 0, 0),
      Color::from_rgb(0, 255, 0),
      Color::from_rgb(0, 0, 255),
      Color::from_rgb(255, 214, 0),
      Color::from_rgb(20, 40, 150),
      Color::from_rgb(120, 120, 120),
      Color::from_rgb(10, 200, 180),
      Color::from_rgb(0, 0, 0),
      Color::from_rgb(255, 255, 255),
      Color::from_rgba(200, 60, 90, 120),
    ]
  }

  #[test]
  fn amount_zero_and_one_give_the_two_colors_in_every_space() {
    for space in SPACES {
      for first in colors() {
        for second in colors() {
          assert_eq!(first.mix(second, 0.0, space), first, "{space:?} start {first} {second}");
          assert_eq!(first.mix(second, 1.0, space), second, "{space:?} end {first} {second}");
        }
      }
    }
  }

  #[test]
  fn mixing_a_color_with_itself_gives_it_back() {
    for space in SPACES {
      for color in colors() {
        assert_eq!(color.mix(color, 0.37, space), color, "{space:?} {color}");
      }
    }
  }

  #[test]
  fn black_and_white_meet_in_the_middle_where_the_space_says() {
    let (black, white) = (Color::black(), Color::white());
    // The channels' average, rounded.
    assert_eq!(black.mix(white, 0.5, ColorSpace::Srgb).rgb(), (128, 128, 128));
    // Light adds, so the middle in linear light is brighter than the average.
    assert_eq!(black.mix(white, 0.5, ColorSpace::LinearSrgb).rgb(), (188, 188, 188));
    // The middle of Oklab's lightness is darker than the average: an Oklab lightness of one half is the gray 99.
    assert_eq!(black.mix(white, 0.5, ColorSpace::Oklab).rgb(), (99, 99, 99));
  }

  #[test]
  fn blue_and_yellow_are_a_dull_gray_in_srgb_and_not_in_oklab() {
    let (blue, yellow) = (Color::from_rgb(0, 0, 255), Color::from_rgb(255, 255, 0));
    assert_eq!(blue.mix(yellow, 0.5, ColorSpace::Srgb).rgb(), (128, 128, 128));
    let (_, chroma_srgb, _) = blue.mix(yellow, 0.5, ColorSpace::Srgb).oklch();
    let (_, chroma_oklab, _) = blue.mix(yellow, 0.5, ColorSpace::Oklab).oklch();
    assert!(chroma_srgb < 0.001, "the sRGB middle is a gray: {chroma_srgb}");
    assert!(chroma_oklab > 0.05, "the Oklab middle has color: {chroma_oklab}");
  }

  #[test]
  fn a_mix_in_oklab_stays_between_the_lightness_of_the_two() {
    let (dark, light) = (Color::from_rgb(20, 40, 150), Color::from_rgb(255, 214, 0));
    let (low, high) = (dark.oklab().0, light.oklab().0);
    for step in 1..10 {
      let (l, _, _) = dark.mix(light, step as f32 / 10.0, ColorSpace::Oklab).oklab();
      assert!(l > low - 0.01 && l < high + 0.01, "{step}: {l}");
    }
  }

  #[test]
  fn hues_go_the_way_round_the_wheel_that_is_asked_for() {
    let (red, blue) = (Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255));
    let hue_of = |color: Color| color.hsl().0;
    // Red is 0 and blue is 240: the shorter way is through magenta (300) and the longer through yellow and green (120).
    let shorter = red.mix(blue, 0.5, ColorInterpolation::new(ColorSpace::Hsl));
    let longer = red.mix(blue, 0.5, ColorInterpolation::new(ColorSpace::Hsl).with_hue(HueDirection::Longer));
    assert!((hue_of(shorter) - 300.0).abs() < 1.0, "{}", hue_of(shorter));
    assert!((hue_of(longer) - 120.0).abs() < 1.0, "{}", hue_of(longer));
    // Increasing from 0 to 240 goes through 120, and decreasing goes through 300.
    let increasing = red.mix(blue, 0.5, ColorInterpolation::new(ColorSpace::Hsl).with_hue(HueDirection::Increasing));
    let decreasing = red.mix(blue, 0.5, ColorInterpolation::new(ColorSpace::Hsl).with_hue(HueDirection::Decreasing));
    assert!((hue_of(increasing) - 120.0).abs() < 1.0);
    assert!((hue_of(decreasing) - 300.0).abs() < 1.0);
    // Going the other way, from blue to red, the shorter way is the same line back.
    let back = blue.mix(red, 0.5, ColorInterpolation::new(ColorSpace::Hsl));
    assert!((hue_of(back) - 300.0).abs() < 1.0);
    // And a reversed interpolation makes the same colors in reverse order.
    let direction = ColorInterpolation::new(ColorSpace::Oklch).with_hue(HueDirection::Increasing);
    assert_eq!(red.mix(blue, 0.3, direction), blue.mix(red, 0.7, direction.reversed()));
  }

  #[test]
  fn a_gray_takes_the_hue_of_the_color_it_is_mixed_with() {
    let (gray, red) = (Color::from_rgb(120, 120, 120), Color::from_rgb(255, 0, 0));
    for space in [ColorSpace::Hsl, ColorSpace::Hsv, ColorSpace::Lch, ColorSpace::Oklch] {
      let middle = gray.mix(red, 0.5, space);
      // It is a paler red: red is at 29 degrees in Oklch, and the middle is at about the same.
      let (_, chroma, hue) = middle.oklch();
      assert!(chroma > 0.03, "{space:?}: {chroma}");
      assert!(hue < 40.0 || hue > 350.0, "{space:?} swept the wheel: {hue}");
      // And the same from the other end.
      let back = red.mix(gray, 0.5, space);
      let (_, _, hue_back) = back.oklch();
      assert!(hue_back < 40.0 || hue_back > 350.0, "{space:?} back: {hue_back}");
    }
  }

  #[test]
  fn a_color_fading_from_transparent_does_not_tint() {
    let clear_red = Color::from_rgba(255, 0, 0, 0);
    let blue = Color::from_rgba(0, 0, 255, 255);
    for space in SPACES {
      assert_eq!(clear_red.mix(blue, 0.5, space).a, 128, "{space:?}");
    }
    // Premultiplied, the transparent red has nothing to give, so the middle is the blue at half opacity. That holds where the colors are
    // channels that add. In a polar space only those other than the hue are premultiplied, so the hue still goes from red's to blue's.
    for space in SPACES.into_iter().filter(|space| !space.is_polar()) {
      assert_eq!(clear_red.mix(blue, 0.5, space).rgb(), (0, 0, 255), "{space:?}");
    }
  }

  #[test]
  fn two_transparent_colors_make_a_transparent_one() {
    let first = Color::from_rgba(255, 0, 0, 0);
    let second = Color::from_rgba(0, 0, 255, 0);
    let middle = first.mix(second, 0.5, ColorSpace::Srgb);
    assert_eq!(middle.a, 0);
  }

  #[test]
  fn going_past_the_ends_goes_further_along_and_is_clipped() {
    let (black, white) = (Color::black(), Color::white());
    assert_eq!(black.mix(white, 2.0, ColorSpace::Srgb), white);
    assert_eq!(black.mix(white, -1.0, ColorSpace::Srgb), black);
    let half = Color::from_rgb(100, 100, 100);
    assert_eq!(half.mix(white, -0.5, ColorSpace::Srgb).rgb(), (100 - 78, 100 - 78, 100 - 78));
  }

  #[test]
  fn the_polar_forms_and_oklab_read_a_color_back_to_itself() {
    for color in colors() {
      let (l, a, b) = color.oklab();
      assert_eq!(Color::from_oklab(l, a, b).rgb(), color.rgb());
      let (l, c, h) = color.oklch();
      assert_eq!(Color::from_oklch(l, c, h).rgb(), color.rgb());
      let (l, c, h) = color.lch();
      assert_eq!(Color::from_lch(l, c, h).rgb(), color.rgb(), "lch {color}");
    }
  }
}
