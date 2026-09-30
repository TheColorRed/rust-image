//! Optional GPU hook for `apply_in_area`.
//!
//! `core` must stay independent of the `gpu` crate. Adjustments/filters describe the work as a [`GpuEffect`] passed to
//! `apply_in_area`, and the `gpu` crate registers a [`GpuProvider`] that runs it. When either is missing,
//! `apply_in_area` uses the CPU path.
use std::sync::RwLock;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Hardware {
  /// Follows the `gpu.enabled` setting in settings.yml.
  /// If enabled, the GPU is used when available, falling back to the CPU when the effect has no
  /// GPU implementation or no GPU is available.
  /// If disabled, the CPU is used.
  /// If the `abra` crate's "gpu" feature is not enabled, the CPU is always used.
  #[default]
  Auto,
  /// Always uses the CPU for the task.
  /// This overrides the settings.yml configuration for hardware selection.
  /// Only has an effect when the `abra` crate's "gpu" feature is enabled, since the CPU is always
  /// used otherwise.
  Cpu,
  /// Prefers the GPU for the task, falling back to the CPU when the effect has no GPU
  /// implementation, no GPU is available, or the GPU fails.
  /// This overrides the settings.yml configuration for hardware selection.
  /// Requires the `abra` crate's "gpu" feature; without it the CPU is always used.
  Gpu,
}

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
  pub shader: &'static str,
  pub uniforms: Vec<u8>,
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

/// Something that can run on the GPU as one or more [`GpuPass`]es.
///
/// Effects describe themselves; the renderer knows nothing about specific effects. Each pass reads the previous
/// pass's output, so a separable blur is two passes and a point adjustment is one.
pub trait GpuEffect {
  /// The passes to run, in order, for an image of the given size. An empty list leaves the image unchanged.
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass>;
}

/// A single-pass effect: one compute shader over an RGBA area, plus the bytes for its uniform buffer.
///
/// See [`GpuPass`] for the shader's binding contract.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuOp {
  pub shader: &'static str,
  pub uniforms: Vec<u8>,
}

impl GpuOp {
  /// Describes a shader run with the given uniform bytes.
  /// - `p_shader`: The WGSL source, usually from `include_str!`.
  /// - `p_uniforms`: The bytes of the uniform buffer, or empty when the shader has none.
  pub fn new(p_shader: &'static str, p_uniforms: impl Into<Vec<u8>>) -> GpuOp {
    GpuOp {
      shader: p_shader,
      uniforms: p_uniforms.into(),
    }
  }
}

impl GpuEffect for GpuOp {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![GpuPass::new(self.shader, self.uniforms.clone())]
  }
}

/// A [`GpuEffect`] that also knows how to run on the CPU, so a layer can hold it as live, re-renderable parameters and
/// still compose on machines without a usable GPU.
pub trait LiveEffect: GpuEffect + Send + Sync {
  /// Whether [`GpuEffect::passes`] describes this effect. An effect that returns `false` has no shader yet: a live
  /// chain runs it on the CPU between GPU runs, so it can be used before it is ported.
  fn has_gpu(&self) -> bool {
    true
  }

  /// Applies the same effect to `p_image` on the CPU.
  fn apply_cpu(&self, p_image: &mut crate::Image);
}

/// The CPU implementation every adjustment, filter and effect provides. It is the only required part: an effect that
/// also has a shader implements [`GpuProcessor`] and returns itself from [`gpu`](Self::gpu).
///
/// Implementing this is enough to be used one-shot and in live chains: [`GpuEffect`] and [`LiveEffect`] are provided
/// for every `CpuProcessor`.
pub trait CpuProcessor: Send + Sync {
  /// Applies the effect to the whole of `p_image`.
  fn process(&self, p_image: &mut crate::Image);

  /// The GPU version of this effect, or `None` (the default) when it only runs on the CPU. Override with
  /// `Some(self)` after implementing [`GpuProcessor`].
  fn gpu(&self) -> Option<&dyn GpuProcessor> {
    None
  }
}

/// The GPU implementation of an effect that has a shader. The shader source is a `&'static str`, so the renderer
/// compiles it once and reuses it; only the uniform values change from frame to frame.
pub trait GpuProcessor {
  /// The passes to run, in order, for an image of the given size. See [`GpuEffect::passes`].
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass>;
}

impl<T: CpuProcessor> GpuEffect for T {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    self.gpu().map(|gpu| gpu.passes(p_width, p_height)).unwrap_or_default()
  }
}

impl<T: CpuProcessor> LiveEffect for T {
  fn has_gpu(&self) -> bool {
    self.gpu().is_some()
  }

  fn apply_cpu(&self, p_image: &mut crate::Image) {
    self.process(p_image);
  }
}

/// A source image held on the GPU, which chains of shader effects are run over. This is the GPU-only layer: it
/// knows nothing about CPU effects. See [`ChainRenderer`] for chains that mix in effects without a shader.
pub trait GpuSession: Send {
  /// Uploads the image that effects are run over, replacing the current source.
  /// - `p_rgba`: `p_width * p_height` RGBA pixels.
  fn set_source(&mut self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> Result<(), String>;

  /// Runs `p_effects` in order over the source. This only submits the work; use [`read_pixels`](Self::read_pixels) to
  /// get the result.
  fn render(&mut self, p_effects: &[&dyn GpuEffect]) -> Result<(), String>;

  /// Waits for the last [`render`](Self::render) and returns the result as RGBA pixels.
  fn read_pixels(&mut self) -> Result<Vec<u8>, String>;
}

impl<S: GpuSession + ?Sized> GpuSession for Box<S> {
  fn set_source(&mut self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> Result<(), String> {
    (**self).set_source(p_width, p_height, p_rgba)
  }

  fn render(&mut self, p_effects: &[&dyn GpuEffect]) -> Result<(), String> {
    (**self).render(p_effects)
  }

  fn read_pixels(&mut self) -> Result<Vec<u8>, String> {
    (**self).read_pixels()
  }
}

/// Renders chains of [`LiveEffect`]s over a source image on a [`GpuSession`]. Runs of effects with a shader stay on the
/// GPU. An effect without one ([`LiveEffect::has_gpu`] is `false`) runs on the CPU in between: the pixels are read
/// back, processed, and uploaded as the new source, so a chain works before every effect has a shader.
pub struct ChainRenderer<S: GpuSession> {
  /// The GPU session underneath, for callers that also need what is specific to it (async readback, presenting).
  pub session: S,
  original: Vec<u8>,
  width: u32,
  height: u32,
  /// The session's source was replaced by a CPU effect's output and must be restored before the next render.
  source_replaced: bool,
}

impl<S: GpuSession> ChainRenderer<S> {
  /// Uploads `p_rgba` (`p_width * p_height` RGBA pixels) to `p_session` as the image effects are applied to.
  pub fn new(mut p_session: S, p_width: u32, p_height: u32, p_rgba: &[u8]) -> Result<ChainRenderer<S>, String> {
    p_session.set_source(p_width, p_height, p_rgba)?;
    Ok(ChainRenderer {
      session: p_session,
      original: p_rgba.to_vec(),
      width: p_width,
      height: p_height,
      source_replaced: false,
    })
  }

  /// The size of the source image in pixels.
  pub fn size(&self) -> (u32, u32) {
    (self.width, self.height)
  }

  /// Puts the original image back as the source if a CPU effect replaced it. Call before rendering a chain that
  /// stays on the GPU.
  pub fn restore_source(&mut self) -> Result<(), String> {
    if self.source_replaced {
      self.session.set_source(self.width, self.height, &self.original)?;
      self.source_replaced = false;
    }
    Ok(())
  }

  /// Runs the chain and returns the final RGBA pixels, waiting for the GPU.
  pub fn render_blocking(&mut self, p_effects: &[&dyn LiveEffect]) -> Result<Vec<u8>, String> {
    self.restore_source()?;
    if p_effects.is_empty() {
      // Nothing to run, but the session must still forget the previous chain's result so the source comes back.
      self.session.render(&[])?;
    }
    let mut index = 0;
    while index < p_effects.len() {
      let start = index;
      while index < p_effects.len() && p_effects[index].has_gpu() {
        index += 1;
      }
      if index > start {
        let run: Vec<&dyn GpuEffect> = p_effects[start..index].iter().map(|effect| *effect as &dyn GpuEffect).collect();
        self.session.render(&run)?;
      }
      if index < p_effects.len() {
        let pixels = self.session.read_pixels()?;
        let mut image = crate::Image::new_from_pixels(self.width, self.height, pixels, crate::Channels::RGBA);
        p_effects[index].apply_cpu(&mut image);
        self.session.set_source(self.width, self.height, &image.into_rgba_vec())?;
        self.source_replaced = true;
        index += 1;
      }
    }
    self.session.read_pixels()
  }
}

/// The functions a GPU implementation registers with `core`.
#[derive(Clone, Copy)]
pub struct GpuProvider {
  /// Blocks until the GPU has finished starting up. Returns `false` when no GPU is available.
  pub wait_until_ready: fn() -> bool,
  /// Runs an effect over `width * height` RGBA pixels and returns the processed pixels.
  pub process: fn(p_effect: &dyn GpuEffect, p_width: u32, p_height: u32, p_pixels: &[u8]) -> Result<Vec<u8>, String>,
  /// Creates an empty session. Wrap it in a [`ChainRenderer`] to upload an image and render chains.
  pub new_session: fn() -> Result<Box<dyn GpuSession>, String>,
}

static GPU_PROVIDER: RwLock<Option<GpuProvider>> = RwLock::new(None);

/// Register the GPU provider. Replaces any previously registered provider.
pub fn register_gpu_provider(p_provider: GpuProvider) {
  *GPU_PROVIDER.write().unwrap() = Some(p_provider);
}

/// Clear the registered GPU provider (used in tests or to disable GPU at runtime).
pub fn clear_gpu_provider() {
  *GPU_PROVIDER.write().unwrap() = None;
}

/// The registered provider when `p_hardware` allows the GPU and it has finished starting, else `None` (use the CPU).
/// `Auto` follows the `gpu.enabled` setting. Blocks until the GPU has finished starting up.
pub fn ready_gpu_provider(p_hardware: Hardware) -> Option<GpuProvider> {
  let use_gpu = match p_hardware {
    Hardware::Auto => crate::Settings::gpu_enabled(),
    Hardware::Cpu => false,
    Hardware::Gpu => true,
  };
  let provider = get_gpu_provider().filter(|_| use_gpu)?;
  (provider.wait_until_ready)().then_some(provider)
}

/// Get the registered GPU provider, if any.
pub fn get_gpu_provider() -> Option<GpuProvider> {
  *GPU_PROVIDER.read().unwrap()
}
