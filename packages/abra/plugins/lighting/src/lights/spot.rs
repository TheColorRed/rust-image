use abra::prelude::Color;

use crate::light::{Incident, Light, PositionedLight, share, toward};

/// A light at one place that shines in a cone toward a target, like a stage spotlight.
#[derive(Clone, Debug)]
pub struct SpotLight {
  position: [f32; 3],
  target: [f32; 3],
  color: Color,
  intensity: f32,
  radius: f32,
  angle: f32,
  softness: f32,
}

impl SpotLight {
  /// A white light of full strength in front of the picture's top left, aimed at the origin with a 30 degree cone.
  pub fn new() -> Self {
    Self {
      position: [0.0, 0.0, 600.0],
      target: [0.0; 3],
      color: Color::from_rgb(255, 255, 255),
      intensity: 1.0,
      radius: 1000.0,
      angle: 30.0,
      softness: 0.3,
    }
  }


  /// Sets the point the cone is aimed at.
  pub fn with_target(mut self, p_target: [f32; 3]) -> Self {
    self.target = p_target;
    self
  }




  /// Sets the half width of the cone, in degrees.
  pub fn with_angle(mut self, p_degrees: f32) -> Self {
    self.angle = p_degrees.clamp(0.1, 89.0);
    self
  }

  /// Sets how much of the cone's edge fades out, from `0` (a hard edge) to `1` (fading from the middle).
  pub fn with_softness(mut self, p_softness: f32) -> Self {
    self.softness = p_softness.clamp(0.0, 1.0);
    self
  }
}

impl Default for SpotLight {
  fn default() -> Self {
    Self::new()
  }
}

impl Light for SpotLight {
  fn color_mut(&mut self) -> &mut Color {
    &mut self.color
  }

  fn intensity_mut(&mut self) -> &mut f32 {
    &mut self.intensity
  }

  fn illuminate(&self, p_position: [f32; 3]) -> Incident {
    let (direction, distance) = toward(p_position, self.position);
    let (aim, _) = toward(self.position, self.target);
    // 1 on the cone's axis, fading to 0 at its edge. The fade covers the outer `softness` share of the angle.
    let along = -(direction[0] * aim[0] + direction[1] * aim[1] + direction[2] * aim[2]);
    let edge = self.angle.to_radians().cos();
    let core = (self.angle * (1.0 - self.softness)).to_radians().cos();
    let inside = ((along - edge) / (core - edge).max(f32::EPSILON)).clamp(0.0, 1.0);
    let ratio = distance / self.radius;
    Incident {
      distance: distance,
      direction,
      strength: self.intensity * inside * inside * (3.0 - 2.0 * inside) / (1.0 + ratio * ratio),
      color: share(self.color),
    }
  }
}

impl PositionedLight for SpotLight {
  fn position_mut(&mut self) -> &mut [f32; 3] {
    &mut self.position
  }

  fn radius_mut(&mut self) -> &mut f32 {
    &mut self.radius
  }
}
