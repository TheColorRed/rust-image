//! Core drawing primitives for unified rasterization.

pub(crate) mod compositor;
pub(crate) mod coverage;
pub(crate) mod rasterize;
pub(crate) mod sampling;
pub(crate) mod shader;

pub(crate) use compositor::{Compositor, SourceOverCompositor};
pub(crate) use coverage::{BrushCoverageMask, CoverageMask, PolygonCoverage};
pub(crate) use rasterize::Rasterizer;
pub(crate) use sampling::SampleGrid;
pub(crate) use shader::{BrushShader, Shader, StrokeBrushShader, shader_from_fill};
