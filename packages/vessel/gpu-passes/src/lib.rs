//! Compute shader passes over RGBA images, and a wgpu runtime that runs them.
//!
//! A [`GpuPass`] is plain data: WGSL source, uniform bytes, and an optional small texture. Whoever has an effect to run
//! describes it as passes and hands them to the runtime, which runs them over RGBA pixels. The runtime knows nothing
//! about effects or about whoever supplies the passes, so a new effect needs a shader and a description, and no change
//! here.
//!
//! The data ([`GpuPass`], [`GpuAux`]) and the [`GpuSession`] trait are always available. The wgpu runtime
//! (`GpuContext`, `LiveRenderer`, `Presenter`) is behind the `runtime` feature, so a crate that only describes passes
//! does not pull in wgpu.
#![deny(missing_docs)]

#[cfg(feature = "runtime")]
pub mod context;
#[cfg(feature = "runtime")]
pub mod present;
#[cfg(feature = "runtime")]
pub mod renderer;
#[cfg(feature = "vessel-target")]
pub mod target;

#[cfg(feature = "runtime")]
pub use context::GpuContext;
#[cfg(feature = "runtime")]
pub use present::Presenter;
#[cfg(feature = "runtime")]
pub use renderer::{Frame, LiveRenderer};

/// A small RGBA texture handed to a shader at binding 3, such as a gradient lookup table.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuAux {
  /// Width in pixels.
  pub width: u32,
  /// Height in pixels.
  pub height: u32,
  /// `width * height` RGBA pixels. Shared, so handing the same texture to the renderer every frame copies nothing and
  /// the renderer can tell it has not changed by comparing the pointers.
  pub rgba: std::sync::Arc<[u8]>,
}

/// One compute shader dispatch over the whole image.
///
/// The shader must use the bindings: 0 = input `texture_2d<f32>`, 1 = output
/// `texture_storage_2d<rgba8unorm, write>`, 2 = uniform (only when `uniforms` is non-empty),
/// 3 = `texture_2d<f32>` (only when `aux` is set), 4 = `texture_2d<f32>` (only when `base` is set), with an
/// `@workgroup_size(8, 8)` entry point named `main`.
///
/// The renderer compiles each distinct `shader` once and reuses it, so changing `uniforms` or `aux` between frames
/// is cheap and never rebuilds a pipeline.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuPass {
  /// The WGSL source. A `&'static str` so the renderer can tell two passes use the same shader.
  pub shader: &'static str,
  /// The bytes of the uniform buffer, or empty when the shader has none.
  pub uniforms: Vec<u8>,
  /// A small texture bound at binding 3.
  pub aux: Option<GpuAux>,
  /// Also binds, at binding 4, the image as it was before the first pass of the effect this pass belongs to. Used by
  /// a pass that combines an effect's result with its input, such as a masked blend.
  pub base: bool,
}

impl GpuPass {
  /// Describes a pass with the given uniform bytes.
  /// - `p_shader`: The WGSL source, usually from `include_str!`.
  /// - `p_uniforms`: The bytes of the uniform buffer, or empty when the shader has none.
  pub fn new(p_shader: &'static str, p_uniforms: impl Into<Vec<u8>>) -> GpuPass {
    GpuPass {
      shader: p_shader,
      uniforms: p_uniforms.into(),
      aux: None,
      base: false,
    }
  }

  /// Binds the effect's input image at binding 4. See [`GpuPass::base`].
  pub fn with_base(mut self) -> GpuPass {
    self.base = true;
    self
  }

  /// Adds an auxiliary texture, bound at binding 3.
  pub fn with_aux(mut self, p_aux: GpuAux) -> GpuPass {
    self.aux = Some(p_aux);
    self
  }
}

/// A source image held on the GPU, which chains of shader effects are run over. This is the GPU-only layer: it
/// knows nothing about effects.
pub trait GpuSession: Send {
  /// Uploads the image that effects are run over, replacing the current source.
  /// - `p_rgba`: `p_width * p_height` RGBA pixels.
  fn set_source(&mut self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> Result<(), String>;

  /// Runs the effects in order over the source. This only submits the work; use [`read_pixels`](Self::read_pixels) to
  /// get the result.
  /// - `p_effects`: The passes of each effect, in order. Each pass reads the previous pass's output, so a separable blur
  ///   is two passes and a point adjustment is one. An effect with no passes leaves the image unchanged.
  fn render(&mut self, p_effects: &[Vec<GpuPass>]) -> Result<(), String>;

  /// Waits for the last [`render`](Self::render) and returns the result as RGBA pixels.
  fn read_pixels(&mut self) -> Result<Vec<u8>, String>;
}

impl<S: GpuSession + ?Sized> GpuSession for Box<S> {
  fn set_source(&mut self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> Result<(), String> {
    (**self).set_source(p_width, p_height, p_rgba)
  }

  fn render(&mut self, p_effects: &[Vec<GpuPass>]) -> Result<(), String> {
    (**self).render(p_effects)
  }

  fn read_pixels(&mut self) -> Result<Vec<u8>, String> {
    (**self).read_pixels()
  }
}
