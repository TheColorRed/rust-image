use abra::prelude::Color;

use crate::light::{Incident, Light, share, toward};

/// A light so far away that it shines the same way on every point, like the sun.
#[derive(Clone, Debug)]
pub struct DirectionalLight {
  direction: [f32; 3],
  color: Color,
  intensity: f32,
}

impl DirectionalLight {
  /// A white light of full strength coming from the top left, in front of the picture.
  pub fn new() -> Self {
    Self {
      direction: toward([0.0; 3], [-0.5, -0.5, 0.7]).0,
      color: Color::from_rgb(255, 255, 255),
      intensity: 1.0,
    }
  }

  /// Sets the way to the light, seen from the scene. It does not need to be a unit vector.
  pub fn with_direction(mut self, p_direction: [f32; 3]) -> Self {
    self.direction = toward([0.0; 3], p_direction).0;
    self
  }


}

impl Default for DirectionalLight {
  fn default() -> Self {
    Self::new()
  }
}

impl Light for DirectionalLight {
  fn color_mut(&mut self) -> &mut Color {
    &mut self.color
  }

  fn intensity_mut(&mut self) -> &mut f32 {
    &mut self.intensity
  }

  fn illuminate(&self, _p_position: [f32; 3]) -> Incident {
    Incident {
      distance: f32::INFINITY,
      direction: self.direction,
      strength: self.intensity,
      color: share(self.color),
    }
  }
}
