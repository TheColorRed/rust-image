use abra::prelude::Color;

use crate::light::{Incident, Light, PositionedLight, share, toward};

/// A light at one place that shines in every direction, like a bulb, and fades with distance.
#[derive(Clone, Debug)]
pub struct OmniLight {
  position: [f32; 3],
  color: Color,
  intensity: f32,
  radius: f32,
}

impl OmniLight {
  /// A white light of full strength a little in front of the picture's top left, reaching 500 pixels.
  pub fn new() -> Self {
    Self {
      position: [0.0, 0.0, 300.0],
      color: Color::from_rgb(255, 255, 255),
      intensity: 1.0,
      radius: 500.0,
    }
  }




}

impl Default for OmniLight {
  fn default() -> Self {
    Self::new()
  }
}

impl Light for OmniLight {
  fn color_mut(&mut self) -> &mut Color {
    &mut self.color
  }

  fn intensity_mut(&mut self) -> &mut f32 {
    &mut self.intensity
  }

  fn illuminate(&self, p_position: [f32; 3]) -> Incident {
    let (direction, distance) = toward(p_position, self.position);
    let ratio = distance / self.radius;
    Incident {
      distance: distance,
      direction,
      strength: self.intensity / (1.0 + ratio * ratio),
      color: share(self.color),
    }
  }
}

impl PositionedLight for OmniLight {
  fn position_mut(&mut self) -> &mut [f32; 3] {
    &mut self.position
  }

  fn radius_mut(&mut self) -> &mut f32 {
    &mut self.radius
  }
}
