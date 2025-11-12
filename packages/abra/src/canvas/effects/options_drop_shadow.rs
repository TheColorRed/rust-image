use crate::color::{Color, Fill};
use crate::combine::blend::{RGBA, normal};

#[derive(Clone, Debug)]
/// Options for configuring a drop shadow effect.
pub struct DropShadowOptions {
  /// The color of the shadow in RGBA format.
  pub fill: Fill,
  /// The blend mode used to combine the shadow with the layer.
  pub blend_mode: fn(RGBA, RGBA) -> RGBA,
  /// The opacity of the shadow (0.0 to 1.0).
  pub opacity: f32,
  /// The angle of the shadow in degrees.
  pub angle: f32,
  /// The distance of the shadow from the object.
  pub distance: f32,
  /// The spread of the shadow between 0.0 and 1.0
  pub spread: f32,
  /// The blur radius of the shadow.
  pub size: f32,
}

impl DropShadowOptions {
  /// Creates a new DropShadowOptions with default settings.
  /// Default values:
  /// - distance: 5.0 pixels
  /// - angle: 45.0 degrees
  /// - blur_radius: 5.0 pixels
  /// - color: black with 60% opacity (0, 0, 0, 153)
  pub fn new() -> Self {
    DropShadowOptions {
      fill: Fill::Solid(Color::black()),
      blend_mode: normal,
      opacity: 0.35,
      angle: 45.0,
      distance: 5.0,
      spread: 0.0,
      size: 5.0,
    }
  }

  /// Sets the distance of the shadow from the object.
  pub fn with_distance(mut self, distance: f32) -> Self {
    self.distance = distance;
    self
  }

  /// Sets the angle of the shadow in degrees.
  pub fn with_angle(mut self, angle: f32) -> Self {
    self.angle = angle;
    self
  }

  /// Sets the size of the shadow blur.
  pub fn with_size(mut self, size: f32) -> Self {
    self.size = size;
    self
  }

  /// Sets the spread of the shadow between 0.0 and 1.0
  pub fn with_spread(mut self, spread: f32) -> Self {
    self.spread = spread.max(0.0).min(1.0);
    self
  }

  /// Sets the color of the shadow in RGBA format.
  pub fn with_fill(mut self, fill: Fill) -> Self {
    self.fill = fill;
    self
  }

  /// Sets the opacity of the shadow (0.0 to 1.0).
  pub fn with_opacity(mut self, opacity: f32) -> Self {
    self.opacity = opacity;
    self
  }

  /// Sets the blend mode used to combine the shadow with the layer.
  pub fn with_blend_mode(mut self, blend_mode: fn(RGBA, RGBA) -> RGBA) -> Self {
    self.blend_mode = blend_mode;
    self
  }
}
