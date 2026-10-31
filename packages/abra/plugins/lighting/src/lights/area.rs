use abra::prelude::Color;

use crate::light::{Incident, Light, PositionedLight, share, toward};

/// A light with a size, like a window or a softbox: it shines in every direction from one place, and the nearer a point is
/// the more of the light's size it sees, so its edges are softer than a [`OmniLight`]'s.
#[derive(Clone, Debug)]
pub struct AreaLight {
  position: [f32; 3],
  color: Color,
  intensity: f32,
  radius: f32,
  size: f32,
}

impl AreaLight {
  /// A white light of full strength a little in front of the picture's top left, 200 pixels wide, reaching 500 pixels.
  pub fn new() -> Self {
    Self {
      position: [0.0, 0.0, 300.0],
      color: Color::from_rgb(255, 255, 255),
      intensity: 1.0,
      radius: 500.0,
      size: 200.0,
    }
  }





  /// Sets how wide the light is, in the units of the picture. `0` makes it an omni light.
  pub fn with_size(mut self, p_size: f32) -> Self {
    self.size = p_size.max(0.0);
    self
  }
}

impl Default for AreaLight {
  fn default() -> Self {
    Self::new()
  }
}

impl Light for AreaLight {
  fn color_mut(&mut self) -> &mut Color {
    &mut self.color
  }

  fn intensity_mut(&mut self) -> &mut f32 {
    &mut self.intensity
  }

  fn illuminate(&self, p_position: [f32; 3]) -> Incident {
    let (direction, distance) = toward(p_position, self.position);
    // The light's own width is distance a point does not have to travel, so the falloff starts at its edge.
    let ratio = (distance - self.size * 0.5).max(0.0) / self.radius;
    Incident {
      distance: distance,
      direction,
      strength: self.intensity / (1.0 + ratio * ratio),
      color: share(self.color),
    }
  }
}

impl PositionedLight for AreaLight {
  fn position_mut(&mut self) -> &mut [f32; 3] {
    &mut self.position
  }

  fn radius_mut(&mut self) -> &mut f32 {
    &mut self.radius
  }
}
