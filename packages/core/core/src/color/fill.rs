use crate::{Color, Gradient, Image};

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

impl From<Gradient> for Fill {
  fn from(gradient: Gradient) -> Self {
    Fill::Gradient(gradient)
  }
}

impl From<Color> for Fill {
  fn from(color: Color) -> Self {
    Fill::Solid(color)
  }
}

impl From<Arc<Image>> for Fill {
  fn from(image: Arc<Image>) -> Self {
    Fill::Image(image)
  }
}

impl From<Image> for Fill {
  fn from(image: Image) -> Self {
    Fill::Image(Arc::new(image))
  }
}

impl From<&Image> for Fill {
  fn from(image: &Image) -> Self {
    Fill::Image(Arc::new(image.clone()))
  }
}
