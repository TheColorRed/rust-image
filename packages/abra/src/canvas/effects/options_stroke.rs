use crate::color::{Color, Fill};

#[derive(Clone, Debug)]
/// Position of the outline relative to the shape.
pub enum OutlinePosition {
  /// Outline is drawn inside the shape.
  Inside,
  /// Outline is drawn outside the shape.
  Outside,
  /// Outline is centered on the shape's edge. Half inside, half outside.
  Center,
}

#[derive(Clone, Debug)]
/// Options for configuring a stroke effect.
pub struct StrokeOptions {
  /// The color of the outline in RGBA format.
  pub fill: Fill,
  /// The blend mode used to combine the outline with the layer.
  pub opacity: f32,
  /// The thickness of the outline.
  pub size: u32,
  /// The position of the outline relative to the shape.
  pub position: OutlinePosition,
}

impl StrokeOptions {
  /// Creates a new StrokeOptions with default settings.
  /// Default values:
  /// - size: 3.0 pixels
  /// - color: black with 100% opacity (0, 0, 0, 255)
  pub fn new() -> Self {
    StrokeOptions {
      fill: Fill::Solid(Color::black()),
      opacity: 1.0,
      size: 3,
      position: OutlinePosition::Inside,
    }
  }

  /// Sets the size of the outline.
  pub fn with_size(mut self, size: u32) -> Self {
    self.size = size;
    self
  }

  /// Sets the fill of the outline.
  pub fn with_fill(mut self, fill: Fill) -> Self {
    self.fill = fill;
    self
  }

  /// Sets the opacity of the outline.
  pub fn with_opacity(mut self, opacity: f32) -> Self {
    self.opacity = opacity;
    self
  }
}
