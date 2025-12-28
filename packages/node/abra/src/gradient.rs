use crate::{color::Color, common::*, path::Path};
use abra::abra_core::Gradient as AbraGradient;

#[napi]
pub struct Gradient {
  pub(crate) inner: AbraGradient,
}

#[napi]
impl Gradient {
  #[napi(constructor)]
  /// Creates a new gradient with the default values starting from black to white.
  pub fn new() -> Self {
    AbraGradient::default().into()
  }

  #[napi(factory)]
  /// Creates a new gradient that goes from one color to another.
  /// @param from The starting color.
  /// @param to The ending color.
  pub fn from_to(from: &Color, to: &Color) -> Self {
    AbraGradient::from_to(from.inner.clone(), to.inner.clone()).into()
  }

  #[napi(factory)]
  /// Creates a new gradient that goes from one color to black.
  /// @param from The starting color.
  /// @return The resulting gradient from the color to black.
  pub fn to_black(from: &Color) -> Self {
    AbraGradient::to_black(from.inner.clone()).into()
  }

  #[napi(factory)]
  /// Creates a new gradient that goes from one color to white.
  /// @param from The starting color.
  /// @return The resulting gradient from the color to white.
  pub fn to_white(from: &Color) -> Self {
    AbraGradient::to_white(from.inner.clone()).into()
  }

  #[napi(factory)]
  /// Creates a new gradient with evenly spaced colors.
  /// @param colors The colors to use in the gradient.
  /// @return The resulting gradient with evenly spaced colors.
  pub fn evenly(colors: Vec<&Color>) -> Self {
    AbraGradient::evenly(colors.into_iter().map(|c| c.inner.clone()).collect()).into()
  }

  #[napi(factory)]
  /// Creates a rainbow gradient.
  /// @return The resulting rainbow gradient.
  pub fn rainbow() -> Self {
    AbraGradient::rainbow().into()
  }

  #[napi(factory)]
  /// Creates a hue gradient.
  /// @return The resulting hue gradient.
  pub fn hue() -> Gradient {
    AbraGradient::hue().into()
  }

  #[napi]
  /// Sets the length of the gradient using a path where the first point is the start and the last point is the end.
  /// @param path The path defining the gradient direction.
  /// @return The updated gradient.
  pub fn set_direction(&mut self, path: &Path) {
    self.inner = self.inner.clone().with_direction(path.inner.clone());
  }

  #[napi(getter)]
  /// Gets the direction of the gradient as a path.
  pub fn direction(&self) -> Option<Path> {
    self.inner.direction().map(|p| Path { inner: p })
  }

  #[napi]
  /// Gets the color at a specific position in the gradient.
  /// @param t A value between 0.0 and 1.0 representing the position in the gradient.
  /// @return The color at the specified position.
  pub fn get_color(&self, t: f64) -> Color {
    self.inner.get_color(t as f32).into()
  }

  #[napi]
  /// Reverses the gradient.
  /// @return The reversed gradient.
  pub fn reverse(&mut self) -> Gradient {
    self.inner.clone().reverse().into()
  }
}

impl From<AbraGradient> for Gradient {
  fn from(inner: AbraGradient) -> Self {
    Self { inner }
  }
}
