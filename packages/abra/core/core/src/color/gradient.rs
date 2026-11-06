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

#[derive(Clone, Debug, Copy, Default, PartialEq)]
/// How solid a gradient is at one place along it, apart from its colors. See [`Gradient::with_alpha_stops`].
pub struct AlphaStop {
  /// How solid the gradient is at this stop, from 0 (clear) to 255 (solid).
  pub alpha: u8,
  /// A value between 0 and 1 representing the position of the stop.
  pub time: f32,
}

impl AlphaStop {
  /// Creates a stop with the given alpha and position.
  pub fn new(p_alpha: u8, p_time: f32) -> AlphaStop {
    AlphaStop { alpha: p_alpha, time: p_time }
  }

  /// How solid the gradient is at `p_time`, mixed in a straight line between the stops around it. Before the first stop and after the last
  /// it is held at that stop's alpha. The stops can be in any order. With no stops it is solid (255).
  pub fn alpha_at(p_stops: &[AlphaStop], p_time: f32) -> u8 {
    let mut sorted = p_stops.to_vec();
    sorted.sort_by(|a, b| a.time.total_cmp(&b.time));
    Self::alpha_in_order(&sorted, p_time)
  }

  /// [`alpha_at`](Self::alpha_at) for stops that are already sorted by position.
  fn alpha_in_order(p_sorted: &[AlphaStop], p_time: f32) -> u8 {
    let Some(last) = p_sorted.last() else { return 255 };
    let Some(after) = p_sorted.iter().position(|stop| stop.time >= p_time) else { return last.alpha };
    if after == 0 {
      return p_sorted[0].alpha;
    }
    let (before, after) = (p_sorted[after - 1], p_sorted[after]);
    let along = if after.time > before.time { (p_time - before.time) / (after.time - before.time) } else { 0.0 };
    (before.alpha as f32 + (after.alpha as f32 - before.alpha as f32) * along).round() as u8
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

  /// A gradient with this one's colors and its own alpha taken from `p_alphas` instead, so the colors and how solid they are can be placed
  /// independently (a color stop at 0 and 1, and a gradient that fades out in the middle). The result has a stop at every place either has
  /// one: this gradient's stops keep their own colors (and hard edges), with the alpha the stops give there, and each alpha stop where the
  /// gradient has none gets the color the gradient has there. Direction and interpolation are kept. With no alpha stops the gradient is
  /// returned as it is.
  /// - `p_alphas`: The alpha stops, in any order.
  pub fn with_alpha_stops(&self, p_alphas: &[AlphaStop]) -> Gradient {
    if p_alphas.is_empty() {
      return self.clone();
    }
    let mut sorted = p_alphas.to_vec();
    sorted.sort_by(|a, b| a.time.total_cmp(&b.time));
    let solid_at = |p_time: f32| AlphaStop::alpha_in_order(&sorted, p_time);
    let mut stops: Vec<ColorStop> =
      self.stops.iter().map(|stop| ColorStop::new(Color::from_rgba(stop.color.r, stop.color.g, stop.color.b, solid_at(stop.time)), stop.time)).collect();
    for alpha in &sorted {
      if !self.stops.iter().any(|stop| stop.time == alpha.time) {
        let color = self.color_at(alpha.time);
        stops.push(ColorStop::new(Color::from_rgba(color.r, color.g, color.b, alpha.alpha), alpha.time));
      }
    }
    stops.sort_by(|a, b| a.time.total_cmp(&b.time));
    Gradient { stops, direction: self.direction.clone(), interpolation: self.interpolation }
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
  fn alpha_is_mixed_in_a_straight_line_between_its_stops_and_held_past_the_ends() {
    // Solid at both ends and clear in the middle, given out of order.
    let stops = [AlphaStop::new(255, 1.0), AlphaStop::new(0, 0.5), AlphaStop::new(255, 0.0)];
    assert_eq!(AlphaStop::alpha_at(&stops, 0.5), 0);
    assert_eq!(AlphaStop::alpha_at(&stops, 0.25), 128, "halfway to nothing");
    assert_eq!(AlphaStop::alpha_at(&stops, -1.0), 255, "held before the first stop");
    assert_eq!(AlphaStop::alpha_at(&stops, 5.0), 255, "and after the last");
    assert_eq!(AlphaStop::alpha_at(&[], 0.5), 255, "with no stops it is solid");
  }

  #[test]
  fn alpha_stops_are_placed_apart_from_the_color_stops() {
    let red_to_blue = Gradient::from_to(Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255));
    let faded = red_to_blue.with_alpha_stops(&[AlphaStop::new(255, 0.0), AlphaStop::new(0, 0.5), AlphaStop::new(255, 1.0)]);
    // The two color stops keep their colors, and a stop appears where the alpha has one, with the color the gradient has there.
    let stops: Vec<_> = faded.stops().iter().map(|stop| (stop.time, stop.color.rgba())).collect();
    assert_eq!(stops, vec![(0.0, (255, 0, 0, 255)), (0.5, (128, 0, 128, 0)), (1.0, (0, 0, 255, 255))]);
    assert_eq!(faded.color_at(0.25).a, 128, "the alpha fades in a straight line");
    // No alpha stops leaves the gradient as it was; the direction and the mixing are kept.
    assert_eq!(red_to_blue.with_alpha_stops(&[]).stops(), red_to_blue.stops());
    let turned = red_to_blue.clone().with_direction(crate::geometry::Path::line((0, 0), (0, 10))).with_interpolation(crate::ColorSpace::Oklab);
    let kept = turned.with_alpha_stops(&[AlphaStop::new(10, 0.5)]);
    assert!(kept.direction().is_some() && kept.interpolation() == turned.interpolation());
  }

  #[test]
  fn a_hard_edge_in_the_colors_survives_the_alpha_stops() {
    let hard = Gradient::new(vec![
      ColorStop::new(Color::from_rgb(255, 0, 0), 0.0),
      ColorStop::new(Color::from_rgb(255, 0, 0), 0.5),
      ColorStop::new(Color::from_rgb(0, 0, 255), 0.5),
      ColorStop::new(Color::from_rgb(0, 0, 255), 1.0),
    ]);
    let stops: Vec<_> = hard.with_alpha_stops(&[AlphaStop::new(100, 0.0), AlphaStop::new(100, 1.0)]).stops().iter().map(|stop| stop.color.rgba()).collect();
    assert_eq!(stops, vec![(255, 0, 0, 100), (255, 0, 0, 100), (0, 0, 255, 100), (0, 0, 255, 100)]);
  }

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
