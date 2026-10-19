use crate::{Color, Gradient, Image};

use std::borrow::Cow;
use std::fmt::Display;
use std::sync::Arc;

/// Which directions an image fill repeats in. An image that does not repeat in a direction is drawn once there, and the area
/// beyond it is clear.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageRepeat {
  /// Drawn once, at the top-left corner of the area.
  #[default]
  NoRepeat,
  /// Tiled in both directions.
  Both,
  /// Tiled horizontally only.
  X,
  /// Tiled vertically only.
  Y,
}

impl ImageRepeat {
  /// Whether the image is tiled horizontally.
  pub fn repeats_x(self) -> bool {
    matches!(self, ImageRepeat::Both | ImageRepeat::X)
  }

  /// Whether the image is tiled vertically.
  pub fn repeats_y(self) -> bool {
    matches!(self, ImageRepeat::Both | ImageRepeat::Y)
  }
}

#[derive(Clone, Debug)]
/// The fill style for drawing shapes, effects, and other graphical that require a fill.
pub enum Fill<'a> {
  /// A solid color fill.
  Solid(Color),
  /// A gradient fill.
  Gradient(Cow<'a, Gradient>),
  /// An image fill, and how it repeats.
  Image(Arc<Image>, ImageRepeat),
}

impl<'a> Fill<'a> {
  /// Creates a solid color fill.
  /// - `p_color`: The color to use for the solid fill.
  pub fn solid(p_color: impl Into<Color>) -> Self {
    Self::Solid(p_color.into())
  }
  /// Creates a gradient fill.
  /// - `p_gradient`: The gradient to use for the fill.
  pub fn gradient(p_gradient: impl Into<Cow<'a, Gradient>>) -> Self {
    Self::Gradient(p_gradient.into())
  }
  /// Creates an image fill that is drawn once, without repeating.
  /// - `p_image`: The image to use for the fill.
  pub fn image(p_image: Arc<Image>) -> Self {
    Self::Image(p_image, ImageRepeat::NoRepeat)
  }
  /// Sets how an image fill repeats. Solid and gradient fills are returned unchanged.
  /// - `p_repeat`: The directions to tile the image in.
  pub fn with_repeat(self, p_repeat: ImageRepeat) -> Self {
    match self {
      Self::Image(image, _) => Self::Image(image, p_repeat),
      other => other,
    }
  }
}

impl From<&Color> for Fill<'_> {
  fn from(p_color: &Color) -> Self {
    Fill::Solid(*p_color)
  }
}

impl From<Color> for Fill<'_> {
  fn from(p_color: Color) -> Self {
    Fill::Solid(p_color)
  }
}

impl<'a> From<&'a Gradient> for Fill<'a> {
  fn from(p_gradient: &'a Gradient) -> Self {
    Fill::Gradient(Cow::Borrowed(p_gradient))
  }
}

impl From<Gradient> for Fill<'_> {
  fn from(p_gradient: Gradient) -> Self {
    Fill::Gradient(Cow::Owned(p_gradient))
  }
}

impl From<Arc<Image>> for Fill<'_> {
  fn from(p_image: Arc<Image>) -> Self {
    Fill::image(p_image)
  }
}

impl From<Image> for Fill<'_> {
  fn from(p_image: Image) -> Self {
    Fill::image(Arc::new(p_image))
  }
}

impl From<&Image> for Fill<'_> {
  fn from(p_image: &Image) -> Self {
    Fill::image(Arc::new(p_image.clone()))
  }
}

impl Display for Fill<'_> {
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Fill::Solid(c) => write!(p_f, "Solid(rgba({}, {}, {}, {}))", c.r, c.g, c.b, c.a),
      Fill::Gradient(_) => write!(p_f, "Gradient(...)"),
      Fill::Image(_, _) => write!(p_f, "Image(...)"),
    }
  }
}
