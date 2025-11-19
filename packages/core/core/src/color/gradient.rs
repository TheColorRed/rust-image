use std::fmt::{self, Display, Formatter};

use crate::Color;

#[derive(Clone, Debug, Copy)]
/// The color stops for a gradient.
pub struct ColorStop {
  /// The color of the stop.
  pub color: Color,
  /// A value between 0 and 1 representing the x position of the stop.
  pub time: f32,
}

impl Display for ColorStop {
  /// Displays the color stop as a string.
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    writeln!(f, "{:?} at {}", self.color, self.time)
  }
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
  pub fn new(color: Color, time: f32) -> ColorStop {
    ColorStop { color, time }
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

impl Gradient {
  /// Creates a new gradient with the given stops.
  pub fn new(stops: Vec<ColorStop>) -> Gradient {
    Gradient { stops, direction: None }
  }

  /// Creates a new gradient that goes from one color to another.
  pub fn from_to(from: Color, to: Color) -> Gradient {
    Gradient {
      stops: vec![ColorStop::new(from, 0.0), ColorStop::new(to, 1.0)],
      direction: None,
    }
  }

  /// Creates a new gradient that goes from one color to black.
  pub fn to_black(from: Color) -> Gradient {
    Gradient {
      stops: vec![
        ColorStop::new(from, 0.0),
        ColorStop::new(Color::from_hex(0x000000), 1.0),
      ],
      direction: None,
    }
  }

  /// Creates a new gradient that goes from one color to white.
  pub fn to_white(from: Color) -> Gradient {
    Gradient {
      stops: vec![
        ColorStop::new(from, 0.0),
        ColorStop::new(Color::from_hex(0xFFFFFF), 1.0),
      ],
      direction: None,
    }
  }

  /// Creates a new gradient with evenly spaced colors.
  pub fn evenly(colors: Vec<Color>) -> Gradient {
    let mut stops = Vec::new();
    let step = 1.0 / (colors.len() as f32 - 1.0);
    for (i, color) in colors.iter().enumerate() {
      stops.push(ColorStop::new(color.clone(), i as f32 * step));
    }
    Gradient { stops, direction: None }
  }
  /// Sets the length of the gradient using a path where the first point is the start and the last point is the end.
  pub fn set_direction(&mut self, path: crate::geometry::Path) -> &mut Self {
    self.direction = Some(path);
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
  pub fn get_color(&self, time: f32) -> (u8, u8, u8, u8) {
    let mut start = ColorStop::default();
    let mut end = ColorStop::default();
    let mut found_start = false;
    let mut found_end = false;

    for stop in self.stops.iter() {
      if stop.time <= time {
        start = stop.clone();
        found_start = true;
      } else if found_start && !found_end {
        end = stop.clone();
        found_end = true;
        break;
      }
    }

    if found_start && found_end {
      let t = (time - start.time) / (end.time - start.time);
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
  pub fn get_color_type(&self, time: f32) -> Color {
    let (r, g, b, a) = self.get_color(time);
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
  fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
    let mut results = vec![];
    for stop in self.stops.iter() {
      results.push(format!(
        "stop=rgba({}, {}, {}, {}) at {}",
        stop.color.r, stop.color.g, stop.color.b, stop.color.a, stop.time
      ));
    }
    write!(f, "{}", results.join("; "))
  }
}

impl Default for Gradient {
  /// Creates a new gradient with the default values.
  /// The default gradient goes from black to white.
  fn default() -> Gradient {
    Gradient {
      stops: vec![
        ColorStop::new(Color::from_hex(0x000000), 0.0),
        ColorStop::new(Color::from_hex(0xFFFFFF), 1.0),
      ],
      direction: None,
    }
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
