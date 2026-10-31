use abra::prelude::Color;

/// What a light gives to one point of a scene.
///
/// Positions and directions use the picture's own axes: `x` goes right, `y` goes down and `z` comes out of the picture
/// toward the viewer. `x` and `y` are in pixels and `z` is in the same units, so a nearer part of the scene has a larger `z`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Incident {
  /// The unit vector from the point toward the light.
  pub direction: [f32; 3],
  /// How far away the light is, in the same units. A light that is infinitely far away, like the sun, is `f32::INFINITY`.
  pub distance: f32,
  /// How much light arrives, before the light's color: `0` is none and `1` is the light's full strength.
  pub strength: f32,
  /// The color of the light, from `0` to `1` for each of red, green and blue.
  pub color: [f32; 3],
}

/// A source of light that can shine on any point of a scene.
///
/// Every light has a color and an intensity, set with [`Light::with_color`] and [`Light::with_intensity`]. A light that sits at a
/// place in the scene is a [`PositionedLight`], which adds a position and a reach.
pub trait Light: Send + Sync {
  /// What this light gives to a point.
  /// - `p_position`: The point, as `[x, y, z]`.
  fn illuminate(&self, p_position: [f32; 3]) -> Incident;

  /// The color of the light.
  fn color_mut(&mut self) -> &mut Color;

  /// How strong the light is. `1` is full strength.
  fn intensity_mut(&mut self) -> &mut f32;

  /// Sets the color of the light.
  fn with_color(mut self, p_color: Color) -> Self
  where
    Self: Sized,
  {
    *self.color_mut() = p_color;
    self
  }

  /// Sets how strong the light is. `1` is full strength.
  fn with_intensity(mut self, p_intensity: f32) -> Self
  where
    Self: Sized,
  {
    *self.intensity_mut() = p_intensity;
    self
  }
}

/// A light that is at one place in the scene and fades with distance from it.
pub trait PositionedLight: Light {
  /// Where the light is, as `[x, y, z]` in the units of the picture (see [`Incident`]).
  fn position_mut(&mut self) -> &mut [f32; 3];

  /// The distance at which the light has fallen to half its strength, in the units of the picture.
  fn radius_mut(&mut self) -> &mut f32;

  /// Sets where the light is, as `[x, y, z]` in the units of the picture (see [`Incident`]).
  fn with_position(mut self, p_position: [f32; 3]) -> Self
  where
    Self: Sized,
  {
    *self.position_mut() = p_position;
    self
  }

  /// Sets the distance at which the light has fallen to half its strength, in the units of the picture.
  fn with_radius(mut self, p_radius: f32) -> Self
  where
    Self: Sized,
  {
    *self.radius_mut() = p_radius.max(f32::EPSILON);
    self
  }
}

/// A color as the share of red, green and blue it has, from `0` to `1` each.
pub(crate) fn share(p_color: Color) -> [f32; 3] {
  let (red, green, blue) = p_color.rgb();
  [red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0]
}

/// The unit vector from `p_from` to `p_to` and the distance between them.
pub(crate) fn toward(p_from: [f32; 3], p_to: [f32; 3]) -> ([f32; 3], f32) {
  let delta = [p_to[0] - p_from[0], p_to[1] - p_from[1], p_to[2] - p_from[2]];
  let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt().max(f32::EPSILON);
  ([delta[0] / length, delta[1] / length, delta[2] / length], length)
}
