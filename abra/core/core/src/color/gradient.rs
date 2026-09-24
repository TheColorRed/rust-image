use std::{
  borrow::Cow,
  fmt::{self, Display, Formatter},
};

use crate::Color;

#[derive(Clone, Debug, Copy)]
/// The color stops for a gradient.
pub struct ColorStop {
  /// The color of the stop.
  pub color: Color,
  /// A value between 0 and 1 representing the x position of the stop.
  pub time: f32,
}

impl ColorStop {
  /// Creates a new gradient color stop with the default values.
  pub fn default() -> ColorStop {
    ColorStop {
      color: Color::default(),
      time: 0.0,
    }
  }

  /// Creates a new gradient color stop with the given color and time.
  pub fn new(p_color: Color, p_time: f32) -> ColorStop {
    ColorStop {
      color: p_color,
      time: p_time,
    }
  }
}

#[derive(Debug)]
/// Describes how to interpolate between colors in a gradient.
pub struct Gradient {
  /// The color stops in the gradient.
  stops: Vec<ColorStop>,
  /// The path defining the gradient direction (optional).
  direction: Option<crate::geometry::Path>,
}

impl Display for ColorStop {
  /// Displays the color stop as a string.
  fn fmt(&self, p_f: &mut Formatter) -> fmt::Result {
    writeln!(p_f, "{:?} at {}", self.color, self.time)
  }
}

impl Gradient {
  /// Creates a new gradient with the given stops.
  pub fn new(p_stops: Vec<ColorStop>) -> Gradient {
    Gradient {
      stops: p_stops,
      direction: None,
    }
  }

  /// Creates a new gradient that goes from one color to another.
  pub fn from_to(p_from: Color, p_to: Color) -> Gradient {
    Gradient {
      stops: vec![ColorStop::new(p_from, 0.0), ColorStop::new(p_to, 1.0)],
      direction: None,
    }
  }

  /// Creates a new gradient that goes from one color to black.
  pub fn to_black(p_from: Color) -> Gradient {
    Gradient {
      stops: vec![
        ColorStop::new(p_from, 0.0),
        ColorStop::new(Color::from_hex(0x000000), 1.0),
      ],
      direction: None,
    }
  }

  /// Creates a new gradient that goes from one color to white.
  pub fn to_white(p_from: Color) -> Gradient {
    Gradient {
      stops: vec![
        ColorStop::new(p_from, 0.0),
        ColorStop::new(Color::from_hex(0xFFFFFF), 1.0),
      ],
      direction: None,
    }
  }

  /// Creates a new gradient with evenly spaced colors.
  pub fn evenly(p_colors: Vec<Color>) -> Gradient {
    let mut stops = Vec::new();
    let step = 1.0 / (p_colors.len() as f32 - 1.0);
    for (i, color) in p_colors.iter().enumerate() {
      stops.push(ColorStop::new(color.clone(), i as f32 * step));
    }
    Gradient { stops, direction: None }
  }

  /// Creates a five-stop analogous gradient in hue order.
  ///
  /// Analogous colors sit next to each other on the color wheel and produce a low-contrast,
  /// cohesive palette. The source color is centered between colors offset toward each neighboring hue.
  pub fn analogous(p_color: Color) -> Gradient {
    Gradient::evenly(Color::analogous(p_color))
  }

  /// Creates a two-stop complementary gradient in hue order.
  ///
  /// Complementary colors are 180 degrees apart on the color wheel, producing the strongest hue contrast.
  /// The gradient runs from the source color to its opposite.
  pub fn complementary(p_color: Color) -> Gradient {
    let complementary = Color::complementary(p_color);
    Gradient::evenly(vec![p_color, complementary])
  }

  /// Creates a smooth five-stop split-complementary gradient in hue order.
  ///
  /// The palette API returns Adobe-style colors in palette order. The gradient reorders all five
  /// colors into a smooth progression: dark source, source, counterclockwise split, clockwise
  /// split, and dark clockwise split.
  pub fn split_complementary(p_color: Color) -> Gradient {
    let palette = Color::split_complementary(p_color);
    Gradient::evenly(vec![palette[3], palette[0], palette[2], palette[1], palette[4]])
  }

  /// Creates a three-stop triadic gradient in hue order.
  ///
  /// A triadic scheme places three colors 120 degrees apart on the color wheel, giving balanced contrast
  /// while keeping each hue visually distinct.
  pub fn triadic(p_color: Color) -> Gradient {
    let mut colors = vec![p_color];
    colors.extend(Color::triadic(p_color));
    Gradient::evenly(colors)
  }

  /// Creates a four-stop square gradient in hue order.
  ///
  /// A square scheme places four colors 90 degrees apart on the color wheel. It provides a broad,
  /// evenly balanced range of warm and cool hues.
  pub fn square(p_color: Color) -> Gradient {
    let mut colors = vec![p_color];
    colors.extend(Color::square(p_color));
    Gradient::evenly(colors)
  }

  /// Creates a three-stop compound gradient in hue order.
  ///
  /// A compound scheme combines a neighboring hue with the hue opposite that neighbor. It mixes the
  /// cohesion of an analogous scheme with the contrast of a complementary scheme.
  pub fn compound(p_color: Color) -> Gradient {
    let mut colors = vec![p_color];
    colors.extend(Color::compound(p_color));
    Gradient::evenly(colors)
  }

  /// Creates a gradient of progressively darker shades of the given color.
  ///
  /// A shade is formed by mixing a color with black. The first stop is the source color and subsequent
  /// stops become darker without reaching pure black. `p_out` specifies the number of stops.
  pub fn shades(p_color: Color, p_out: usize) -> Gradient {
    let colors = Color::shades(p_color, p_out);
    let stops = colors
      .into_iter()
      .enumerate()
      .map(|(index, p_color)| ColorStop::new(p_color, index as f32 / (p_out - 1) as f32))
      .collect();
    Gradient { stops, direction: None }
  }

  /// Creates a monochromatic gradient around the given color.
  ///
  /// Monochromatic colors share one hue while varying in lightness. The source color is surrounded by
  /// darker and lighter variants, and `p_out` specifies the number of stops.
  pub fn monochromatic(p_color: Color, p_out: usize) -> Gradient {
    let colors = Color::monochromatic(p_color, p_out);
    let stops = colors
      .into_iter()
      .enumerate()
      .map(|(index, p_color)| ColorStop::new(p_color, index as f32 / (p_out - 1) as f32))
      .collect();
    Gradient { stops, direction: None }
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

  /// Gets the color of the gradient at the given time.
  pub fn get_color(&self, p_time: f32) -> (u8, u8, u8, u8) {
    let mut start = ColorStop::default();
    let mut end = ColorStop::default();
    let mut found_start = false;
    let mut found_end = false;

    for stop in self.stops.iter() {
      if stop.time <= p_time {
        start = stop.clone();
        found_start = true;
      } else if found_start && !found_end {
        end = stop.clone();
        found_end = true;
        break;
      }
    }

    if found_start && found_end {
      let t = (p_time - start.time) / (end.time - start.time);
      let r = (start.color.r as f32 + (end.color.r as f32 - start.color.r as f32) * t) as u8;
      let g = (start.color.g as f32 + (end.color.g as f32 - start.color.g as f32) * t) as u8;
      let b = (start.color.b as f32 + (end.color.b as f32 - start.color.b as f32) * t) as u8;
      let a = (start.color.a as f32 + (end.color.a as f32 - start.color.a as f32) * t) as u8;
      (r, g, b, a)
    } else if found_start && !found_end {
      (start.color.r, start.color.g, start.color.b, start.color.a)
    } else if !found_start && found_end {
      (end.color.r, end.color.g, end.color.b, end.color.a)
    } else {
      (0, 0, 0, 0)
    }
  }

  /// Gets the color of the gradient at the given time.
  pub fn get_color_type(&self, p_time: f32) -> Color {
    let (r, g, b, a) = self.get_color(p_time);
    Color { r, g, b, a }
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

impl Clone for Gradient {
  /// Clones the gradient.
  fn clone(&self) -> Gradient {
    let mut stops = Vec::new();
    for stop in self.stops.iter() {
      stops.push(stop.clone());
    }
    Gradient {
      stops,
      direction: self.direction.clone(),
    }
  }
}

impl<'a> Into<Cow<'a, Gradient>> for Gradient {
  fn into(self) -> Cow<'a, Gradient> {
    Cow::Owned(self)
  }
}
