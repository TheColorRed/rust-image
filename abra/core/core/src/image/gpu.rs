//! Optional GPU hook for `apply_in_area`.
//!
//! `core` must stay independent of the `gpu` crate. Adjustments/filters describe the work as a [`GpuEffect`] passed to
//! `apply_in_area`, and the `gpu` crate registers a [`GpuProvider`] that runs it. When either is missing,
//! `apply_in_area` uses the CPU path.
//!
//! The data a GPU runs ([`GpuPass`], [`GpuAux`]) and the [`GpuSession`] trait live in the standalone `gpu-passes`
//! package, which knows nothing about abra. An effect here only describes itself as passes ([`GpuEffect`]); whoever runs
//! them is a separate crate.
use std::sync::RwLock;

pub use gpu_passes::{GpuAux, GpuPass, GpuSession};

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

/// The CPU entry point for an effect: what it does to an image on the CPU, limited to its area and mask. It always
/// runs on the CPU and never uses the GPU, whatever the settings. Every [`Apply`](crate::image::effect::Apply) type has this provided, so an effect does not
/// implement it; it is what [`ChainRenderer`] and live images call for an effect that has no shader.
///
/// [`GpuEffect`] and [`LiveEffect`] are provided for every `CpuProcessor`.
pub trait CpuProcessor: Send + Sync {
  /// Applies the effect to `p_image` on the CPU.
  fn process(&self, p_image: &mut crate::Image);

  /// The GPU version of this effect, or `None` (the default) when it only runs on the CPU.
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
    // The session still holds the previous chain's result. Rendering nothing makes the source the result again, which
    // a chain that is empty, or starts with a CPU effect, would otherwise read back instead of the source.
    self.session.render(&[])?;
    let mut index = 0;
    while index < p_effects.len() {
      let start = index;
      while index < p_effects.len() && p_effects[index].has_gpu() {
        index += 1;
      }
      if index > start {
        let run: Vec<Vec<GpuPass>> =
          p_effects[start..index].iter().map(|effect| effect.passes(self.width, self.height)).collect();
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
  /// Runs the passes of effects, in order, over `width * height` RGBA pixels and returns the processed pixels.
  pub process: fn(p_effects: &[Vec<GpuPass>], p_width: u32, p_height: u32, p_pixels: &[u8]) -> Result<Vec<u8>, String>,
  /// Creates an empty session. Wrap it in a [`ChainRenderer`] to upload an image and render chains.
  pub new_session: fn() -> Result<Box<dyn GpuSession>, String>,
}

static GPU_PROVIDER: RwLock<Option<GpuProvider>> = RwLock::new(None);

/// The GPU provider is global, so tests that register or clear one must hold this, or they change it under each other.
#[doc(hidden)]
pub static GPU_PROVIDER_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
