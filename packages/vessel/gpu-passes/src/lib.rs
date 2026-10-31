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
#[cfg(feature = "runtime")]
pub mod strokes;
#[cfg(feature = "runtime")]
pub mod tiles;
#[cfg(feature = "vessel-target")]
pub mod target;

#[cfg(feature = "runtime")]
pub use context::GpuContext;
#[cfg(feature = "runtime")]
pub use present::{PresentOptions, Presenter};
#[cfg(feature = "runtime")]
pub use renderer::{Frame, LiveRenderer};
#[cfg(feature = "runtime")]
pub use strokes::{StrokeCanvas, StrokeStyle};
#[cfg(feature = "runtime")]
pub use tiles::{Regions, TileOptions, TiledRenderer, Window};

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
/// `@workgroup_size(8, 8)` entry point named `main`, and 5 = uniform (only when `window` is set).
///
/// A pass whose shader declares its output as `texture_storage_2d<rgba32float, write>` instead keeps real numbers in a 32-bit float
/// texture for the pass after it to read (sums, means, variances that 8 bits cannot hold), with nothing to set on the pass: the renderer
/// reads the format from the shader. Such a pass cannot be the last of a chain, which must end in an 8-bit pass, and a pass that reads
/// `base` cannot follow one. A float texture is four times the size of an 8-bit one.
///
/// A pass that sets `window` also gets, at binding 5, where the image it is run on sits in the photo it is a view of:
/// `struct Window { origin: vec2<f32>, scale: f32, scale_y: f32, full: vec2<f32>, pad2: vec2<f32> }`. A pixel `p` of the image
/// is the photo's pixel `origin + (p + 0.5) * scale`, in a photo `full` pixels across. (`pad2` holds the cosine and sine of
/// a turn of the picture; a turned picture is never given to a pass that asks for the window, so these two are only for the
/// renderer's own passes.) (`scale_y` is the same as `scale`
/// unless the picture is stretched, and a pass that asks for the window is only given a picture that is not.) For an image that is the whole
/// photo, `origin` is zero and `scale` is one. A pass whose result depends on where a pixel is in the photo (a gradient,
/// a vignette) or on a distance in photo pixels (a blur radius) reads it, so it is the same however the photo is zoomed.
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
  /// Also binds, at binding 5, where the image sits in the photo. See above.
  pub window: bool,
  /// How far, in photo pixels, this pass reads beyond a pixel (a blur's radius). A renderer that draws part of a photo
  /// pads the picture by this much, so the pixels at its edge have the neighbours they need. Zero for a pass that reads
  /// only the pixel it writes.
  pub reach: f32,
  /// The pass reads other pixels, or depends on where a pixel is, and has no `window` version: it is only right when the
  /// image is drawn at its own size. A renderer that draws part of a photo at another size refuses it.
  pub pixel_exact: bool,
  /// The size of the image this pass writes, when that is not the size of the image it reads (a resize, a rotation that grows
  /// the image to hold its corners). Only the last pass of a chain can have one, because the textures the passes before it
  /// write are the source's size. The pass is dispatched over this size, so the shader's own size is `textureDimensions(output_tex)`
  /// and the size of what it reads is `textureDimensions(input_tex)`. `None` writes an image the size of the one it reads.
  pub output: Option<(u32, u32)>,
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
      window: false,
      reach: 0.0,
      pixel_exact: false,
      output: None,
    }
  }

  /// Says the pass writes an image of `p_width` x `p_height`, which is not the size of the image it reads. See [`GpuPass::output`].
  pub fn with_output(mut self, p_width: u32, p_height: u32) -> GpuPass {
    self.output = Some((p_width, p_height));
    self
  }

  /// Says how far, in photo pixels, the pass reads beyond a pixel. See [`GpuPass::reach`].
  pub fn with_reach(mut self, p_pixels: f32) -> GpuPass {
    self.reach = p_pixels;
    self
  }

  /// Marks the pass as right only at the image's own size. See [`GpuPass::pixel_exact`].
  pub fn with_pixel_exact(mut self) -> GpuPass {
    self.pixel_exact = true;
    self
  }

  /// Binds where the image sits in the photo at binding 5. See [`GpuPass::window`].
  pub fn with_window(mut self) -> GpuPass {
    self.window = true;
    self
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

  /// The width and height of what the last [`render`](Self::render) made, which [`read_pixels`](Self::read_pixels) returns
  /// `width * height` pixels of. That is the source's size unless the last pass has an [`output`](GpuPass::output). `None`
  /// before a source is set.
  fn output_size(&self) -> Option<(u32, u32)>;
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

  fn output_size(&self) -> Option<(u32, u32)> {
    (**self).output_size()
  }
}
