use crate::{color::Color, common::*, path::Path};
use abra::abra_core::blend::blend_images;
use abra::abra_core::{Channels, Gradient as AbraGradient, Image, blend};
use abra::drawing::prelude::fill;
use abra::prelude::*;

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
    AbraGradient::from_to(from.inner, abra::abra_core::Color::black()).into()
  }

  #[napi(factory)]
  /// Creates a new gradient that goes from one color to white.
  /// @param from The starting color.
  /// @return The resulting gradient from the color to white.
  pub fn to_white(from: &Color) -> Self {
    AbraGradient::from_to(from.inner, abra::abra_core::Color::white()).into()
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
    self.inner.color_at(t as f32).into()
  }

  #[napi]
  /// Reverses the gradient.
  /// @return The reversed gradient.
  pub fn reverse(&mut self) -> Gradient {
    self.inner.clone().reverse().into()
  }

  #[napi]
  /// Fills an area of the specified width and height with the gradient starting from the specified (x, y) coordinates.
  /// @param layer The layer to use as a base for the gradient fill.
  /// @param x The x coordinate to start the gradient fill.
  /// @param y The y coordinate to start the gradient fill.
  /// @param width The width of the area to fill.
  /// @param height The height of the area to fill.
  /// @return The ImageData containing the filled gradient.
  pub fn fill_at(&self, layer: &Layer, x: u32, y: u32, width: u32, height: u32) -> ImageData {
    let (img_width, img_height, rgba) = layer.inner.with_image_mut(|layer_image| {
      let (w, h) = layer_image.dimensions::<u32>();
      let data = layer_image.rgba().to_vec();
      (w, h, data)
    });
    let mut result_image = Image::new_from_pixels(img_width, img_height, rgba, Channels::RGBA);

    let area = Area::rect((x as f32, y as f32), (width as f32, height as f32));
    let gradient = fill(&area, &self.inner);
    blend_images(&mut result_image, &gradient, BlendMode::Normal);
    ImageData::from_image(&result_image)
  }

  #[napi]
  /// Fills an area of the specified width and height with the gradient starting from the top-left corner.
  /// @param layer The layer to use as a base for the gradient fill.
  /// @param width The width of the area to fill.
  /// @param height The height of the area to fill.
  /// @return The ImageData containing the filled gradient.
  pub fn fill(&self, layer: &Layer, width: u32, height: u32) -> ImageData {
    self.fill_at(layer, 0, 0, width, height)
  }
}

impl From<AbraGradient> for Gradient {
  fn from(inner: AbraGradient) -> Self {
    Self { inner }
  }
}
