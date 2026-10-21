use std::{
  borrow::Cow,
  fmt::{self, Display, Formatter},
};

use crate::{Color, Harmony, ColorInterpolation};

#[derive(Clone, Debug, Copy, Default, PartialEq)]
/// The color stops for a gradient.
pub struct ColorStop {
  /// The color of the stop.
  pub color: Color,
  /// A value between 0 and 1 representing the x position of the stop.
  pub time: f32,
}

impl ColorStop {
  /// Creates a new gradient color stop with the given color and time.
  pub fn new(p_color: Color, p_time: f32) -> ColorStop {
    ColorStop {
      color: p_color,
      time: p_time,
    }
  }
}

#[derive(Debug, Clone)]
/// Describes how to interpolate between colors in a gradient.
pub struct Gradient {
  /// The color stops in the gradient.
  stops: Vec<ColorStop>,
  /// The path defining the gradient direction (optional).
  direction: Option<crate::geometry::Path>,
  /// How the colors between two stops are mixed.
  interpolation: ColorInterpolation,
}

impl Display for ColorStop {
  /// Displays the color stop as a string.
  fn fmt(&self, p_f: &mut Formatter) -> fmt::Result {
    write!(p_f, "{} at {}", self.color, self.time)
  }
}

impl Gradient {
  /// Creates a new gradient with the given stops. They can be in any order; they are sorted by position, and stops at
  /// the same position keep the order they were given in, so two of them make a hard edge.
  pub fn new(p_stops: Vec<ColorStop>) -> Gradient {
    let mut stops = p_stops;
    stops.sort_by(|a, b| a.time.total_cmp(&b.time));
    Gradient {
      stops,
      direction: None,
      interpolation: ColorInterpolation::default(),
    }
  }

  /// Creates a new gradient that goes from one color to another.
  pub fn from_to(p_from: Color, p_to: Color) -> Gradient {
    Gradient {
      stops: vec![ColorStop::new(p_from, 0.0), ColorStop::new(p_to, 1.0)],
      direction: None,
      interpolation: ColorInterpolation::default(),
    }
  }

  /// Creates a new gradient with evenly spaced colors.
  pub fn evenly(p_colors: Vec<Color>) -> Gradient {
    // A single color sits at 0 instead of dividing by zero.
    let step = if p_colors.len() > 1 { 1.0 / (p_colors.len() - 1) as f32 } else { 0.0 };
    let stops = p_colors.into_iter().enumerate().map(|(i, color)| ColorStop::new(color, i as f32 * step)).collect();
    Gradient {
      stops,
      direction: None,
      interpolation: ColorInterpolation::default(),
    }
  }

  /// Creates an evenly spaced gradient from a color harmony of the given color.
  ///
  /// The stops follow the palette from [`Color::harmony`], except
  /// [`Harmony::SplitComplementary`], which is reordered into a smooth hue progression:
  /// dark source, source, counterclockwise split, clockwise split, and dark clockwise split.
  ///
  /// - `p_color`: The source color.
  /// - `p_harmony`: The harmony to build the gradient from.
  ///
  /// ```ignore
  /// let fill = Gradient::harmony(Color::ruby(), Harmony::Complementary);
  /// ```
  pub fn harmony(p_color: Color, p_harmony: Harmony) -> Gradient {
    let colors = p_color.harmony(p_harmony);
    match p_harmony {
      Harmony::SplitComplementary => Gradient::evenly(vec![colors[3], colors[0], colors[2], colors[1], colors[4]]),
      _ => Gradient::evenly(colors),
    }
  }

  /// Sets how the colors between two stops are mixed: the color space, and for a polar space which way round the color wheel the hue goes.
  /// The default is sRGB, the red, green and blue values as stored, which is the plain average of the channels and goes dull between
  /// colors far apart (blue to yellow passes through gray). A space built to follow what the eye sees, such as
  /// [`ColorSpace::Oklab`](crate::ColorSpace::Oklab), keeps the colors clean, and a polar space such as `Oklch` goes round the wheel,
  /// which is how a rainbow is made from two colors. See [`Color::mix`] for how the mixing is done.
  /// - `p_interpolation`: The space to mix in. A [`ColorSpace`](crate::ColorSpace) mixes the shorter way round the wheel.
  ///
  /// ```ignore
  /// let ramp = Gradient::from_to(Color::blue(), Color::yellow()).with_interpolation(ColorSpace::Oklab);
  /// ```
  pub fn with_interpolation(mut self, p_interpolation: impl Into<ColorInterpolation>) -> Self {
    self.interpolation = p_interpolation.into();
    self
  }

  /// How the colors between two stops are mixed.
  pub fn interpolation(&self) -> ColorInterpolation {
    self.interpolation
  }

  /// Sets the length of the gradient using a path where the first point is the start and the last point is the end.
  pub fn with_direction(mut self, p_path: impl Into<crate::geometry::Path>) -> Self {
    self.direction = Some(p_path.into());
    self
  }
  /// Gets the length of the gradient.
  pub fn direction(&self) -> Option<crate::geometry::Path> {
    self.direction.clone()
  }
  /// The color stops, sorted by position.
  pub fn stops(&self) -> &[ColorStop] {
    &self.stops
  }
  /// Creates a new rainbow gradient.
  /// This gradient goes from red to orange to yellow to green to blue to indigo to violet.
  pub fn rainbow() -> Gradient {
    Gradient::evenly(vec![
      Color::from_hex(0xFF0000),
      Color::from_hex(0xFF7F00),
      Color::from_hex(0xFFFF00),
      Color::from_hex(0x00FF00),
      Color::from_hex(0x0000FF),
      Color::from_hex(0x4B0082),
      Color::from_hex(0x9400D3),
    ])
  }

  /// Creates a gradient that is based on the hue of colors going from 360 to 0.
  /// This gradient goes from red to orange to yellow to green to blue to indigo to violet.
  pub fn hue() -> Gradient {
    Gradient::evenly(vec![
      Color::from_hsv(0.0, 1.0, 1.0),
      Color::from_hsv(300.0, 1.0, 1.0),
      Color::from_hsv(240.0, 1.0, 1.0),
      Color::from_hsv(180.0, 1.0, 1.0),
      Color::from_hsv(120.0, 1.0, 1.0),
      Color::from_hsv(60.0, 1.0, 1.0),
      Color::from_hsv(0.0, 1.0, 1.0),
    ])
  }

  /// The color of the gradient at `p_time`, mixing the two stops around it as [`Gradient::with_interpolation`] says. Times before the
  /// first stop or after the last take that stop's color; a gradient with no stops is transparent.
  pub fn color_at(&self, p_time: f32) -> Color {
    let after = self.stops.iter().position(|stop| stop.time > p_time);
    let (start, end) = match after {
      Some(0) => return self.stops[0].color,
      Some(index) => (self.stops[index - 1], self.stops[index]),
      None => return self.stops.last().map_or(Color::transparent(), |stop| stop.color),
    };
    let t = (p_time - start.time) / (end.time - start.time);
    start.color.mix(end.color, t, self.interpolation)
  }

  /// Reverses the gradient.
  pub fn reverse(&self) -> Gradient {
    let mut stops = Vec::new();
    let max_time = self.stops.last().map_or(1.0, |stop| stop.time);
    for stop in self.stops.iter().rev() {
      stops.push(ColorStop::new(stop.color.clone(), max_time - stop.time));
    }
    Gradient {
      stops,
      direction: self.direction.clone(),
      // Going from the last stop to the first, a hue that grew now shrinks.
      interpolation: self.interpolation.reversed(),
    }
  }
}

impl Display for Gradient {
  /// Displays the gradient as a string.
  fn fmt(&self, p_f: &mut Formatter) -> std::fmt::Result {
    let mut results = vec![];
    for stop in self.stops.iter() {
      results.push(format!(
        "stop=rgba({}, {}, {}, {}) at {}",
        stop.color.r, stop.color.g, stop.color.b, stop.color.a, stop.time
      ));
    }
    write!(p_f, "{}", results.join("; "))
  }
}

impl Default for Gradient {
  /// Creates a new gradient with the default values.
  /// The default gradient goes from black to white.
  fn default() -> Gradient {
    Gradient::from_to(Color::black(), Color::white())
  }
}

impl<'a> From<Gradient> for Cow<'a, Gradient> {
  fn from(p_gradient: Gradient) -> Self {
    Cow::Owned(p_gradient)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{ColorSpace, HueDirection};

  #[test]
  fn color_at_blends_between_stops_and_holds_past_the_ends() {
    let gradient = Gradient::from_to(Color::black(), Color::white());
    assert_eq!(gradient.color_at(-1.0), Color::black());
    // The channels' average, rounded: a half is 127.5, which is 128.
    assert_eq!(gradient.color_at(0.5).rgb(), (128, 128, 128));
    assert_eq!(gradient.color_at(2.0), Color::white());
    assert_eq!(Gradient::new(vec![]).color_at(0.5), Color::transparent());
  }

  #[test]
  fn the_space_the_colors_are_mixed_in_is_the_gradients_to_set() {
    let (blue, yellow) = (Color::from_rgb(0, 0, 255), Color::from_rgb(255, 255, 0));
    let plain = Gradient::from_to(blue, yellow);
    assert_eq!(plain.interpolation(), ColorInterpolation::default());
    assert_eq!(plain.color_at(0.5).rgb(), (128, 128, 128), "sRGB goes through a gray");
    let clean = Gradient::from_to(blue, yellow).with_interpolation(ColorSpace::Oklab);
    let (_, chroma, _) = clean.color_at(0.5).oklch();
    assert!(chroma > 0.05, "Oklab keeps the color: {chroma}");
    // The ends are the stops in any space, and the middle is the mix of the two colors.
    assert_eq!((clean.color_at(0.0), clean.color_at(1.0)), (blue, yellow));
    assert_eq!(clean.color_at(0.3), blue.mix(yellow, 0.3, ColorSpace::Oklab));
    // Black to white in Oklab has a darker middle than the average.
    assert_eq!(Gradient::default().with_interpolation(ColorSpace::Oklab).color_at(0.5).rgb(), (99, 99, 99));
  }

  #[test]
  fn a_polar_gradient_goes_round_the_wheel_the_way_it_is_told() {
    let (red, blue) = (Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255));
    let short = Gradient::from_to(red, blue).with_interpolation(ColorInterpolation::new(ColorSpace::Hsl));
    let long = Gradient::from_to(red, blue)
      .with_interpolation(ColorInterpolation::new(ColorSpace::Hsl).with_hue(HueDirection::Longer));
    assert!((short.color_at(0.5).hsl().0 - 300.0).abs() < 1.0, "through magenta");
    assert!((long.color_at(0.5).hsl().0 - 120.0).abs() < 1.0, "through green");
  }

  #[test]
  fn a_gradient_fading_in_from_transparent_does_not_tint() {
    let fade = Gradient::from_to(Color::from_rgba(255, 0, 0, 0), Color::from_rgba(0, 0, 255, 255));
    let middle = fade.color_at(0.5);
    assert_eq!((middle.rgb(), middle.a), ((0, 0, 255), 128));
  }

  #[test]
  fn reversing_a_gradient_reverses_the_colors_and_the_way_the_hue_goes() {
    let (red, blue) = (Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255));
    let up = Gradient::from_to(red, blue)
      .with_interpolation(ColorInterpolation::new(ColorSpace::Oklch).with_hue(HueDirection::Increasing));
    let down = up.reverse();
    assert_eq!(down.interpolation().hue, HueDirection::Decreasing);
    for time in [0.0, 0.2, 0.5, 0.8, 1.0] {
      assert_eq!(down.color_at(time), up.color_at(1.0 - time), "at {time}");
    }
  }

  #[test]
  fn stops_can_be_given_in_any_order() {
    let red = Color::from_rgba(255, 0, 0, 255);
    let green = Color::from_rgba(0, 255, 0, 255);
    let blue = Color::from_rgba(0, 0, 255, 255);
    let sorted = Gradient::new(vec![
      ColorStop::new(red, 0.0),
      ColorStop::new(green, 0.5),
      ColorStop::new(blue, 1.0),
    ]);
    let shuffled = Gradient::new(vec![
      ColorStop::new(blue, 1.0),
      ColorStop::new(red, 0.0),
      ColorStop::new(green, 0.5),
    ]);
    for time in [-0.5, 0.0, 0.25, 0.5, 0.75, 1.0, 1.5] {
      assert_eq!(shuffled.color_at(time), sorted.color_at(time), "at {time}");
    }
  }

  #[test]
  fn two_stops_at_the_same_position_make_a_hard_edge_in_the_order_given() {
    let red = Color::from_rgba(255, 0, 0, 255);
    let blue = Color::from_rgba(0, 0, 255, 255);
    let hard = Gradient::new(vec![
      ColorStop::new(red, 0.0),
      ColorStop::new(red, 0.5),
      ColorStop::new(blue, 0.5),
      ColorStop::new(blue, 1.0),
    ]);
    assert_eq!(hard.color_at(0.499), red);
    assert_eq!(hard.color_at(0.5), blue);
  }
}
