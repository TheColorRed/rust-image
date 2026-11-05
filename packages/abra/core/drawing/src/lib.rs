//! Core drawing primitives for unified rasterization.
//!
//! This module contains the core types used by the rasterizer: compositors,
//! coverage masks, rasterizer orchestration, supersampling grid and shading
//! primitives. These components form a minimal rasterization pipeline used
//! for filling paths/areas and strokes with various shading and blending
//! behaviors.
//!
//! Examples
//! ```ignore
//! use abra::draw::abra_core::{Rasterizer, FullCoverage, SampleGrid, SourceOverCompositor};
//! // Create a simple rasterization pipeline that fills an area and writes
//! // it back into an `Image` with default compositing.
//! ```

mod core {
  pub mod compositor;
  pub mod coverage;
  pub mod coverage_map;
  pub mod painter;
  pub mod rasterize;
  pub mod sampling;
  pub mod sdf;
  pub mod shader;
}
mod shaders {
  pub mod brush_dabs_shader;
  pub mod brush_shader;
  pub mod fill_feather_shader;
  pub mod image_shader;
  pub mod linear_gradient_shader;
  pub mod solid_shader;
  pub mod stroke_brush_shader;
}
mod brush {
  pub mod brush;
}
mod falloff;
mod fill;
mod sprites;
mod stroke_overlay;
mod performance;

pub use brush::brush::Brush;
pub use core::compositor::{Blend, Compositor, SourceOverCompositor};
pub use core::coverage::{CoverageMask, PolygonCoverage, RectCoverage, StrokeCoverage};
pub use core::coverage_map::CoverageMap;
pub use core::painter::*;
pub use core::rasterize::Rasterizer;
pub use core::sampling::SampleGrid;
pub use core::sdf::{draw_area_fill, draw_area_stroke, draw_ellipse_stroke};
pub use core::shader::{Shader, shader_from_fill, shader_from_fill_with_path};
pub use fill::*;
pub use sprites::{SPRITE_MAX_WORDS, SPRITE_VALUES, SPRITE_WORDS, Sprite, SpriteTrail, Sprites, TRAIL_POINT_VALUES, TRAIL_POINT_WORDS, TRAIL_WORDS, TrailPoint, sprites};
pub use stroke_overlay::{MAX_STROKE_POINTS, StrokeLook, stroke_overlay, stroke_overlay_deferred};
pub use performance::DrawingAlgorithm;
