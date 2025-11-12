use crate::{
  Image,
  color::{color::Color, gradient::Gradient},
};

use std::fmt::Display;
use std::sync::Arc;

#[derive(Clone, Debug)]
/// The fill style for drawing shapes, effects, and other graphical that require a fill.
pub enum Fill {
  /// A solid color fill.
  Solid(Color),
  /// A gradient fill.
  Gradient(Gradient),
  /// An image fill.
  Image(Arc<Image>),
}

impl Display for Fill {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Fill::Solid(c) => write!(f, "Solid(rgba({}, {}, {}, {}))", c.r, c.g, c.b, c.a),
      Fill::Gradient(_) => write!(f, "Gradient(...)"),
      Fill::Image(_) => write!(f, "Image(...)"),
    }
  }
}
