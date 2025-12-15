//! Shaders compute an RGBA color at a sample coordinate used by the rasterizer.
//!
//! The `Shader` trait defines an interface that maps device-space
//! coordinates `(x, y)` to an `(r, g, b, a)` color tuple. Shaders are
//! evaluated at subpixel sample locations from a `SampleGrid` and their
//! results are averaged and composited onto the destination by the
//! rasterizer. Shaders must be `Sync` because the rasterizer may invoke
//! them concurrently on several threads.
//!
//! Implementations
//! - `SolidShader`: returns a constant color.
//! - `LinearGradientShader` / `RadialGradientShader`: evaluate gradients
//!   using a `Path` or radial parameter respectively.
//! - `ImageShader`: samples pixel data from a source `Image`.
//! - `BrushShader` / `StrokeBrushShader`: wrappers that modulate alpha
//!   according to brush geometry and hardness.
//!
//! Design notes:
//! - Shaders operate in device coordinates and should take offsets into
//!   account if needed (e.g., `ImageShader`).
//! - Builders or conversion helpers like `shader_from_fill` are provided
//!   to simplify common conversions from public `Fill` types.
//!
//! Example
//! ```ignore
//! use abra::draw::abra_core::shader_from_fill;
//! let fill = abra::color::Fill::Solid(abra::color::Color::from_rgba(255, 0, 0, 255));
//! let shader = shader_from_fill(&fill); // Box<dyn Shader>
//! let (r, g, b, a) = shader.shade(12.5, 30.25);
//! ```

use abra_core::Fill;
use abra_core::Path;

use crate::shaders::image_shader::ImageShader;
use crate::shaders::linear_gradient_shader::LinearGradientShader;
use crate::shaders::solid_shader::SolidShader;

/// Trait for shading computations at arbitrary pixel locations.
///
/// Implementors compute the `(r, g, b, a)` color for each sample
/// coordinate: `(x, y)` in device space. The returned components are in
/// the 0..255 range as `u8` values.
///
/// Example
/// ```ignore
/// let shader: Box<dyn Shader + Send + Sync> = Box::new(SolidShader::new(Color::white()));
/// let (r, g, b, a) = shader.shade(10.5, 20.25);
/// ```
pub trait Shader: Sync {
  /// Returns the RGBA color at position (x, y).
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8);
}

/// Creates a shader from a Fill variant.
///
/// Convenience function that constructs a boxed `Shader` from a `Fill`.
///
/// - For `Fill::Solid` a `SolidShader` is created.
/// - For `Fill::Gradient` a `LinearGradientShader` is created using the
///   gradient's direction path (if available) or an empty path otherwise.
/// - For `Fill::Image` an `ImageShader` is created.
///
/// Example
/// ```ignore
/// let shader = shader_from_fill(&Fill::Solid(Color::from_rgba(255,255,255,255)));
/// ```
pub fn shader_from_fill(p_fill: impl Into<Fill>) -> Box<dyn Shader + Send + Sync> {
  match p_fill.into() {
    Fill::Solid(color) => Box::new(SolidShader::new(color)),
    Fill::Gradient(gradient) => {
      let path = gradient.direction().unwrap_or_else(|| Path::new());
      Box::new(LinearGradientShader::new(path, gradient.clone()))
    }
    Fill::Image(image) => Box::new(ImageShader::new(image.clone(), 0.0, 0.0)),
  }
}

/// Creates a shader from a Fill variant with the provided fallback path to be used when
/// the fill has no explicit gradient direction. The fallback path is only used when
/// the gradient has no direction.
pub fn shader_from_fill_with_path(
  p_fill: impl Into<Fill>, fallback_path: Option<Path>,
) -> Box<dyn Shader + Send + Sync> {
  match p_fill.into() {
    Fill::Solid(color) => Box::new(SolidShader::new(color)),
    Fill::Gradient(gradient) => {
      let path = gradient
        .direction()
        .unwrap_or_else(|| fallback_path.unwrap_or_else(|| Path::new()));
      let gradient_clone = gradient.clone();
      // Ensure gradient has a direction so gradient::get_color uses path parameterization
      gradient_clone.with_direction(path.clone());
      Box::new(LinearGradientShader::new(path, gradient.clone()))
    }
    Fill::Image(image) => Box::new(ImageShader::new(image.clone(), 0.0, 0.0)),
  }
}
