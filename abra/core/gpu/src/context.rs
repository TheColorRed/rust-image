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

/// A minimal GPU context wrapper that owns a `wgpu::Device` and `wgpu::Queue`.
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

    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default()).await?;

    Ok(Self {
      device,
      queue,
      adapter,
      instance,
      pipelines: PipelineCache::default(),
    })
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
}
