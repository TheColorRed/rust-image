//! Shader trait for computing pixel colors.

use crate::color::{Color, Fill};
use crate::geometry::{Path, PointF};

/// Trait for shading computations at arbitrary pixel locations.
pub(crate) trait Shader: Sync {
  /// Returns the RGBA color at position (x, y).
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8);
}

/// Solid color shader.
pub(crate) struct SolidShader {
  color: (u8, u8, u8, u8),
}

impl SolidShader {
  pub fn new(p_color: Color) -> Self {
    SolidShader { color: p_color.rgba() }
  }
}

impl Shader for SolidShader {
  fn shade(&self, _p_x: f32, _p_y: f32) -> (u8, u8, u8, u8) {
    self.color
  }
}

/// Linear gradient shader using path-based parameter mapping.
pub(crate) struct LinearGradientShader {
  path: Path,
  gradient: crate::color::gradient::Gradient,
}

impl LinearGradientShader {
  pub fn new(p_path: Path, p_gradient: crate::color::gradient::Gradient) -> Self {
    LinearGradientShader {
      path: p_path,
      gradient: p_gradient,
    }
  }
}

impl Shader for LinearGradientShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let t = self.path.closest_time(p_x, p_y);
    self.gradient.get_color(t)
  }
}

/// Radial gradient shader.
pub(crate) struct RadialGradientShader {
  center_x: f32,
  center_y: f32,
  radius: f32,
  gradient: crate::color::gradient::Gradient,
}

impl RadialGradientShader {
  pub fn new(p_center_x: f32, p_center_y: f32, p_radius: f32, p_gradient: crate::color::gradient::Gradient) -> Self {
    RadialGradientShader {
      center_x: p_center_x,
      center_y: p_center_y,
      radius: p_radius,
      gradient: p_gradient,
    }
  }
}

impl Shader for RadialGradientShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let dx = p_x - self.center_x;
    let dy = p_y - self.center_y;
    let distance = (dx * dx + dy * dy).sqrt();
    let t = (distance / self.radius).min(1.0).max(0.0);
    self.gradient.get_color(t)
  }
}

/// Image shader (samples from an image).
pub(crate) struct ImageShader {
  pixels: Vec<u8>,
  width: i32,
  height: i32,
  offset_x: f32,
  offset_y: f32,
}

impl ImageShader {
  pub fn new(p_image: crate::Image, p_offset_x: f32, p_offset_y: f32) -> Self {
    let (width, height) = p_image.dimensions::<i32>();
    let pixels = p_image.rgba();
    ImageShader {
      pixels,
      width,
      height,
      offset_x: p_offset_x,
      offset_y: p_offset_y,
    }
  }
}

impl Shader for ImageShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let sample_x = (p_x - self.offset_x).floor() as i32;
    let sample_y = (p_y - self.offset_y).floor() as i32;

    if sample_x < 0 || sample_y < 0 || sample_x >= self.width || sample_y >= self.height {
      return (0, 0, 0, 0);
    }

    let idx = ((sample_y * self.width + sample_x) as usize) * 4;
    (self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2], self.pixels[idx + 3])
  }
}

/// Brush shader with hardness-based alpha falloff.
/// Wraps another shader and applies smooth alpha falloff based on distance from center.
pub(crate) struct BrushShader {
  inner: Box<dyn Shader + Send + Sync>,
  center_x: f32,
  center_y: f32,
  max_distance: f32,
  hardness: f32,
}

impl BrushShader {
  pub fn new(
    p_inner: Box<dyn Shader + Send + Sync>, p_center_x: f32, p_center_y: f32, p_max_distance: f32, p_hardness: f32,
  ) -> Self {
    BrushShader {
      inner: p_inner,
      center_x: p_center_x,
      center_y: p_center_y,
      max_distance: p_max_distance,
      hardness: p_hardness.clamp(0.0, 1.0),
    }
  }

  /// Computes alpha falloff based on distance from center and hardness.
  fn compute_alpha_falloff(&self, p_x: f32, p_y: f32) -> f32 {
    let dx = p_x - self.center_x;
    let dy = p_y - self.center_y;
    let distance = (dx * dx + dy * dy).sqrt();

    if distance >= self.max_distance {
      return 0.0;
    }

    let normalized_dist = distance / self.max_distance;

    let falloff = if self.hardness < 0.5 {
      let t = self.hardness * 2.0;
      let quadratic = 1.0 - (normalized_dist * normalized_dist);
      let linear = 1.0 - normalized_dist;
      quadratic * (1.0 - t) + linear * t
    } else {
      let t = (self.hardness - 0.5) * 2.0;
      let linear = 1.0 - normalized_dist;
      let hard_edge = if normalized_dist < 0.5 { 1.0 } else { 0.0 };
      linear * (1.0 - t) + hard_edge * t
    };

    falloff.max(0.0)
  }
}

impl Shader for BrushShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let (r, g, b, mut a) = self.inner.shade(p_x, p_y);

    // Apply alpha falloff based on hardness
    let falloff = self.compute_alpha_falloff(p_x, p_y);
    a = ((a as f32) * falloff) as u8;

    (r, g, b, a)
  }
}

/// Stroke brush shader: applies hardness-based falloff relative to a path centerline.
pub(crate) struct StrokeBrushShader {
  inner: Box<dyn Shader + Send + Sync>,
  path: crate::geometry::Path,
  max_distance: f32,
  hardness: f32,
  // Pre-flattened path points for fast closest-point queries during shading.
  flattened: Vec<PointF>,
}

impl StrokeBrushShader {
  pub fn new(
    p_inner: Box<dyn Shader + Send + Sync>, p_path: crate::geometry::Path, p_max_distance: f32, p_hardness: f32,
  ) -> Self {
    // Pre-flatten the path to a set of points; choose a tolerance that balances accuracy and performance.
    let flattened = p_path.flatten(1.0);
    StrokeBrushShader {
      inner: p_inner,
      path: p_path,
      max_distance: p_max_distance,
      hardness: p_hardness.clamp(0.0, 1.0),
      flattened,
    }
  }

  fn compute_alpha_falloff_from_distance(&self, distance: f32) -> f32 {
    if distance >= self.max_distance {
      return 0.0;
    }
    let normalized_dist = distance / self.max_distance;

    if self.hardness < 0.5 {
      let t = self.hardness * 2.0;
      let quadratic = 1.0 - (normalized_dist * normalized_dist);
      let linear = 1.0 - normalized_dist;
      (quadratic * (1.0 - t) + linear * t).max(0.0)
    } else {
      let t = (self.hardness - 0.5) * 2.0;
      let linear = 1.0 - normalized_dist;
      let hard_edge = if normalized_dist < 0.5 { 1.0 } else { 0.0 };
      (linear * (1.0 - t) + hard_edge * t).max(0.0)
    }
  }
}

impl Shader for StrokeBrushShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let (r, g, b, mut a) = self.inner.shade(p_x, p_y);
    // Compute closest point on the path and distance to it
    let center_point = self.closest_point_on_flattened(p_x, p_y);

    let dx = p_x - center_point.x;
    let dy = p_y - center_point.y;
    let dist = (dx * dx + dy * dy).sqrt();
    let falloff = self.compute_alpha_falloff_from_distance(dist);
    // (Debug prints removed)
    a = ((a as f32) * falloff) as u8;
    (r, g, b, a)
  }
}

impl StrokeBrushShader {
  // Finds the closest point on the pre-flattened path to (p_x, p_y).
  fn closest_point_on_flattened(&self, p_x: f32, p_y: f32) -> PointF {
    if self.flattened.len() < 2 {
      return self.path.start();
    }
    let query = PointF::new(p_x, p_y);
    let mut min_distance = f32::MAX;
    let mut closest = self.flattened[0];
    for i in 0..self.flattened.len() - 1 {
      let p1 = self.flattened[i];
      let p2 = self.flattened[i + 1];
      let seg_vec = PointF::new(p2.x - p1.x, p2.y - p1.y);
      let seg_len_sq = seg_vec.length_squared();
      if seg_len_sq == 0.0 {
        continue;
      }
      let query_vec = PointF::new(query.x - p1.x, query.y - p1.y);
      let t = (query_vec.dot(seg_vec) / seg_len_sq).clamp(0.0, 1.0);
      let candidate = p1.lerp(p2, t);
      let dist = query.distance_to(candidate);
      if dist < min_distance {
        min_distance = dist;
        closest = candidate;
      }
    }
    closest
  }
}

/// Creates a shader from a Fill variant.
pub(crate) fn shader_from_fill(p_fill: &Fill) -> Box<dyn Shader + Send + Sync> {
  match p_fill {
    Fill::Solid(color) => Box::new(SolidShader::new(*color)),
    Fill::Gradient(gradient) => {
      let path = gradient.direction().unwrap_or_else(|| Path::new());
      Box::new(LinearGradientShader::new(path, gradient.clone()))
    }
    Fill::Image(image) => Box::new(ImageShader::new((**image).clone(), 0.0, 0.0)),
  }
}
