use std::{
  borrow::Cow,
  fmt::{self, Display, Formatter},
};

use crate::{Color, Harmony};

#[derive(Clone, Debug, Copy, Default, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
/// Describes how to interpolate between colors in a gradient.
pub struct Gradient {
  /// The color stops in the gradient.
  stops: Vec<ColorStop>,
  /// The path defining the gradient direction (optional).
  direction: Option<crate::geometry::Path>,
}

// What other languages can call on a color stop. Colors are handles there, so `new` has its own version here.
#[cfg(feature = "uniffi")]
#[uniffi::export]
impl ColorStop {
  /// Creates a new gradient color stop with the given color and time.
  #[uniffi::constructor(name = "new")]
  pub fn uniffi_new(p_color: std::sync::Arc<Color>, p_time: f32) -> ColorStop {
    ColorStop::new(*p_color, p_time)
  }

  /// The color of the stop.
  pub fn color(&self) -> std::sync::Arc<Color> {
    std::sync::Arc::new(self.color)
  }

  /// A value between 0 and 1 representing the x position of the stop.
  pub fn time(&self) -> f32 {
    self.time
  }
}

impl Display for ColorStop {
  /// Displays the color stop as a string.
  fn fmt(&self, p_f: &mut Formatter) -> fmt::Result {
    write!(p_f, "{} at {}", self.color, self.time)
  }
}

// What other languages can call on a gradient. Colors and stops are handles there, so these constructors have their own
// versions here under the same names.
#[cfg(feature = "uniffi")]
#[uniffi::export]
impl Gradient {
  /// Creates a new gradient with the given stops. They can be in any order; they are sorted by position.
  #[uniffi::constructor(name = "new")]
  pub fn uniffi_new(p_stops: Vec<std::sync::Arc<ColorStop>>) -> Gradient {
    Gradient::new(p_stops.into_iter().map(|stop| *stop).collect())
  }

  /// Creates a new gradient that goes from one color to another.
  #[uniffi::constructor(name = "from_to")]
  pub fn uniffi_from_to(p_from: std::sync::Arc<Color>, p_to: std::sync::Arc<Color>) -> Gradient {
    Gradient::from_to(*p_from, *p_to)
  }

  /// Creates a new gradient with evenly spaced colors.
  #[uniffi::constructor(name = "evenly")]
  pub fn uniffi_evenly(p_colors: Vec<std::sync::Arc<Color>>) -> Gradient {
    Gradient::evenly(p_colors.into_iter().map(|color| *color).collect())
  }

  /// The color of the gradient at `p_time`, blending the two stops around it.
  #[uniffi::method(name = "color_at")]
  pub fn uniffi_color_at(&self, p_time: f32) -> std::sync::Arc<Color> {
    std::sync::Arc::new(self.color_at(p_time))
  }
}

impl Gradient {
  /// Creates a new gradient with the given stops. They can be in any order; they are sorted by position, and stops at
  /// the same position keep the order they were given in, so two of them make a hard edge.
  pub fn new(p_stops: Vec<ColorStop>) -> Gradient {
    let mut stops = p_stops;
    stops.sort_by(|a, b| a.time.total_cmp(&b.time));
    Gradient { stops, direction: None }
  }

  /// Creates a new gradient that goes from one color to another.
  pub fn from_to(p_from: Color, p_to: Color) -> Gradient {
    Gradient {
      stops: vec![ColorStop::new(p_from, 0.0), ColorStop::new(p_to, 1.0)],
      direction: None,
    }
  }

  /// Creates a new gradient with evenly spaced colors.
  pub fn evenly(p_colors: Vec<Color>) -> Gradient {
    // A single color sits at 0 instead of dividing by zero.
    let step = if p_colors.len() > 1 { 1.0 / (p_colors.len() - 1) as f32 } else { 0.0 };
    let stops = p_colors.into_iter().enumerate().map(|(i, color)| ColorStop::new(color, i as f32 * step)).collect();
    Gradient { stops, direction: None }
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

  /// Sets the length of the gradient using a path where the first point is the start and the last point is the end.
  pub fn with_direction(mut self, p_path: impl Into<crate::geometry::Path>) -> Self {
    self.direction = Some(p_path.into());
    self
  }
  /// Gets the length of the gradient.
  pub fn direction(&self) -> Option<crate::geometry::Path> {
    self.direction.clone()
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

  /// The color of the gradient at `p_time`, blending the two stops around it. Times before the first stop or after
  /// the last take that stop's color; a gradient with no stops is transparent.
  pub fn color_at(&self, p_time: f32) -> Color {
    let after = self.stops.iter().position(|stop| stop.time > p_time);
    let (start, end) = match after {
      Some(0) => return self.stops[0].color,
      Some(index) => (self.stops[index - 1], self.stops[index]),
      None => return self.stops.last().map_or(Color::transparent(), |stop| stop.color),
    };
    let t = (p_time - start.time) / (end.time - start.time);
    let channel = |from: u8, to: u8| (from as f32 + (to as f32 - from as f32) * t) as u8;
    Color::from_rgba(
      channel(start.color.r, end.color.r),
      channel(start.color.g, end.color.g),
      channel(start.color.b, end.color.b),
      channel(start.color.a, end.color.a),
    )
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

  #[test]
  fn color_at_blends_between_stops_and_holds_past_the_ends() {
    let gradient = Gradient::from_to(Color::black(), Color::white());
    assert_eq!(gradient.color_at(-1.0), Color::black());
    assert_eq!(gradient.color_at(0.5).rgb(), (127, 127, 127));
    assert_eq!(gradient.color_at(2.0), Color::white());
    assert_eq!(Gradient::new(vec![]).color_at(0.5), Color::transparent());
  }

  #[test]
  fn stops_can_be_given_in_any_order() {
    let red = Color::from_rgba(255, 0, 0, 255);
    let green = Color::from_rgba(0, 255, 0, 255);
    let blue = Color::from_rgba(0, 0, 255, 255);
    let sorted = Gradient::new(vec![ColorStop::new(red, 0.0), ColorStop::new(green, 0.5), ColorStop::new(blue, 1.0)]);
    let shuffled = Gradient::new(vec![ColorStop::new(blue, 1.0), ColorStop::new(red, 0.0), ColorStop::new(green, 0.5)]);
    for time in [-0.5, 0.0, 0.25, 0.5, 0.75, 1.0, 1.5] {
      assert_eq!(shuffled.color_at(time), sorted.color_at(time), "at {time}");
    }
  }

  #[test]
  fn two_stops_at_the_same_position_make_a_hard_edge_in_the_order_given() {
    let red = Color::from_rgba(255, 0, 0, 255);
    let blue = Color::from_rgba(0, 0, 255, 255);
    let hard = Gradient::new(vec![ColorStop::new(red, 0.0), ColorStop::new(red, 0.5), ColorStop::new(blue, 0.5), ColorStop::new(blue, 1.0)]);
    assert_eq!(hard.color_at(0.499), red);
    assert_eq!(hard.color_at(0.5), blue);
  }
}
