//! GPU context helpers
//!
//! Provides a small wrapper around `wgpu::Device` and `wgpu::Queue` for easy
//! creation and shader compilation in headless contexts used in tests and examples.

use crate::renderer::{Pipeline, PipelineKey};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Texture format used for compute input and output.
pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// Workgroup size (x and y) that compute shaders must declare with `@workgroup_size(8, 8)`.
pub(crate) const WORKGROUP_SIZE: u32 = 8;

/// Compiled compute pipelines, shared by every renderer on a context so a new one starts warm.
pub(crate) type PipelineCache = Arc<Mutex<HashMap<PipelineKey, Pipeline>>>;

/// Compiled render pipelines (tiles, strokes, the presenter's blit), shared by every user on a context so making a second
/// `TiledRenderer`, `StrokeCanvas` or `Presenter` does not compile its shader again.
///
/// Like the compute pipelines, the number of entries is bounded: a key is a shader that is part of this crate plus a texture
/// format, so the map cannot grow past the shaders there are. Dropping entries would only make them compile again.
#[derive(Clone)]
pub(crate) struct RenderEntry {
  pub pipeline: wgpu::RenderPipeline,
  /// The bind group layout the pipeline was made with, for a pipeline that was not given an automatic layout.
  pub layout: Option<wgpu::BindGroupLayout>,
}

#[derive(Hash, PartialEq, Eq)]
pub(crate) struct RenderKey {
  /// Names the shader and how it is built; a shader with a variant (such as premultiplied alpha) has a name for each.
  pub name: &'static str,
  pub format: wgpu::TextureFormat,
}

pub(crate) type RenderCache = Arc<Mutex<HashMap<RenderKey, RenderEntry>>>;

/// The driver's own compiled-shader cache and the file it is kept in. See [`GpuContext::use_pipeline_cache_file`].
pub(crate) struct DiskCache {
  cache: wgpu::PipelineCache,
  path: std::path::PathBuf,
}

/// A minimal GPU context wrapper that owns a `wgpu::Device` and `wgpu::Queue`.
///
/// Clones share a synchronization gate: runtime submissions, device polling and surface configuration cannot overlap.
/// This prevents configuring a surface while another renderer submits work on the same device. Direct use of the
/// public device or queue bypasses that protection and must not race the runtime.
#[derive(Clone)]
pub struct GpuContext {
  /// The device handle
  pub device: wgpu::Device,
  /// The queue handle
  pub queue: wgpu::Queue,
  /// Backend adapter
  pub adapter: wgpu::Adapter,
  /// The instance the adapter came from; surfaces must be created from it.
  pub instance: wgpu::Instance,
  pub(crate) pipelines: PipelineCache,
  pub(crate) render_pipelines: RenderCache,
  // Set once by `use_pipeline_cache_file`; clones share it. Never locked, so it cannot take part in a lock cycle.
  disk_cache: Arc<std::sync::OnceLock<DiskCache>>,
  // Surface configuration waits for idle and must not overlap submissions from another renderer.
  pub(crate) submission_gate: Arc<Mutex<()>>,
}

impl GpuContext {
  /// Create a new async context by requesting an adapter and device.
  pub async fn new_default_async() -> anyhow::Result<Self> {
    // Starting only Vulkan/Metal is much faster than starting every backend (~200ms vs ~600ms on
    // Windows). Fall back to every backend for machines without them.
    let (instance, adapter) = match Self::request_adapter(wgpu::Backends::VULKAN | wgpu::Backends::METAL).await {
      Ok(found) => found,
      Err(_) => Self::request_adapter(wgpu::Backends::all()).await?,
    };

    // The driver's compiled-shader cache can be saved to a file only where the adapter supports it (Vulkan).
    let descriptor = wgpu::DeviceDescriptor {
      required_features: adapter.features() & wgpu::Features::PIPELINE_CACHE,
      ..Default::default()
    };
    let (device, queue) = adapter.request_device(&descriptor).await?;

    Ok(Self {
      device,
      queue,
      adapter,
      instance,
      pipelines: PipelineCache::default(),
      render_pipelines: RenderCache::default(),
      disk_cache: Arc::default(),
      submission_gate: Arc::default(),
    })
  }

  /// Keeps the driver's compiled shaders in the file at `p_path`, so a shader compiled in an earlier run does not compile again
  /// when the app starts. Call it once, right after making the context and before any pipeline is built; pipelines built before
  /// it are not covered. The caller picks the place (the library never chooses where files go) and a missing or unreadable file
  /// just starts an empty cache. Data from another driver or version is ignored by the driver.
  ///
  /// Returns whether the cache is in use. It is not on a backend that cannot save compiled shaders (everything but Vulkan), or if
  /// a cache was set already; nothing else changes then. The file is rewritten each time a new pipeline is compiled, and by
  /// [`save_pipeline_cache`](Self::save_pipeline_cache).
  pub fn use_pipeline_cache_file(&self, p_path: impl Into<std::path::PathBuf>) -> bool {
    if !self.device.features().contains(wgpu::Features::PIPELINE_CACHE) {
      return false;
    }
    let path = p_path.into();
    let data = std::fs::read(&path).ok();
    // SAFETY: the data is either from a file this method wrote from `PipelineCache::get_data`, or something else that the
    // driver rejects; `fallback: true` makes a rejected file start an empty cache instead of failing.
    let cache = unsafe {
      self.device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
        label: Some("gpu-passes::pipeline_cache"),
        data: data.as_deref(),
        fallback: true,
      })
    };
    self.disk_cache.set(DiskCache { cache, path }).is_ok()
  }

  /// Writes the driver's compiled shaders to the file given to [`use_pipeline_cache_file`](Self::use_pipeline_cache_file). It does
  /// nothing without one. The file is written whole to a side file and moved into place, so a crash cannot leave half of it.
  pub fn save_pipeline_cache(&self) {
    let Some(disk) = self.disk_cache.get() else { return };
    let Some(data) = disk.cache.get_data() else { return };
    let side = disk.path.with_extension("tmp");
    if std::fs::write(&side, &data).is_ok() && std::fs::rename(&side, &disk.path).is_err() {
      let _ = std::fs::remove_file(&side);
    }
  }

  /// The cache to pass as `cache:` when building a pipeline.
  pub(crate) fn driver_cache(&self) -> Option<&wgpu::PipelineCache> {
    self.disk_cache.get().map(|disk| &disk.cache)
  }

  /// The render pipeline for `p_key`, built by `p_build` the first time and shared afterwards. The lock is not held while
  /// building, so two threads may both build one; the first to finish is kept.
  pub(crate) fn render_pipeline(&self, p_key: RenderKey, p_build: impl FnOnce() -> RenderEntry) -> RenderEntry {
    let found = self.render_pipelines.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get(&p_key).cloned();
    if let Some(entry) = found {
      return entry;
    }
    let built = p_build();
    let kept = self
      .render_pipelines
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .entry(p_key)
      .or_insert(built)
      .clone();
    self.save_pipeline_cache();
    kept
  }

  async fn request_adapter(p_backends: wgpu::Backends) -> anyhow::Result<(wgpu::Instance, wgpu::Adapter)> {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = p_backends;
    // Debug builds default to emitting shader debug info, which some mobile drivers (PowerVR) abort on.
    desc.flags.remove(wgpu::InstanceFlags::DEBUG);
    let instance = wgpu::Instance::new(desc);
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await?;
    Ok((instance, adapter))
  }

  /// Blocking helper for tests or simple CLI use.
  pub fn new_default_blocking() -> anyhow::Result<Self> {
    pollster::block_on(Self::new_default_async())
  }

  /// Compile a WGSL shader module from the given source string.
  pub fn compile_wgsl(&self, p_source: &str, p_label: Option<&str>) -> wgpu::ShaderModule {
    self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: p_label,
      source: wgpu::ShaderSource::Wgsl(p_source.into()),
    })
  }

  /// Takes the submission gate. It guards the order of GPU calls and no data, so a panic while it was held (wgpu panics on a
  /// validation error, such as a failed surface reconfigure) leaves nothing half-changed. The gate is then poisoned, and a
  /// later call goes on as normal instead of panicking for the rest of the process.
  pub(crate) fn gate(&self) -> std::sync::MutexGuard<'_, ()> {
    self.submission_gate.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
  }

  /// Waits until the GPU has finished everything submitted so far. A wait with no limit reaches the driver as the largest timeout
  /// there is, which some Android drivers cannot take: they report a timeout at once, so a read-back failed whenever the GPU was
  /// still busy. A limit they can take, asked again until the work is done, waits the same. The gate is taken for each ask and
  /// not for all of them, so other threads can still submit while this one waits.
  pub(crate) fn wait(&self) -> Result<(), wgpu::PollError> {
    const ASK: std::time::Duration = std::time::Duration::from_millis(100);
    const GIVE_UP: std::time::Duration = std::time::Duration::from_secs(2);
    let started = std::time::Instant::now();
    loop {
      let asked = {
        let _guard = self.gate();
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(ASK) })
      };
      match asked {
        Err(wgpu::PollError::Timeout) if started.elapsed() < GIVE_UP => {}
        other => return other.map(|_| ()),
      }
    }
  }

  pub(crate) fn submit(&self, p_command: wgpu::CommandBuffer) -> wgpu::SubmissionIndex {
    let _guard = self.gate();
    self.queue.submit(Some(p_command))
  }

  pub(crate) fn configure(&self, p_surface: &wgpu::Surface<'_>, p_config: &wgpu::SurfaceConfiguration) {
    let _guard = self.gate();
    p_surface.configure(&self.device, p_config);
  }
}
