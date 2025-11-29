//! GPU context helpers
//!
//! Provides a small wrapper around `wgpu::Device` and `wgpu::Queue` for easy
//! creation and shader compilation in headless contexts used in tests and examples.
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// A minimal GPU context wrapper that owns a `wgpu::Device` and `wgpu::Queue`.
#[derive(Clone)]
pub struct GpuContext {
    /// The device handle
    pub device: Arc<wgpu::Device>,
    /// The queue handle
    pub queue: Arc<wgpu::Queue>,
    /// Backend adapter
    pub adapter: wgpu::Adapter,
}

impl GpuContext {
    /// Create a new async context by requesting an adapter and device.
    pub async fn new_default_async() -> anyhow::Result<Self> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter,
        })
    }

    /// Blocking helper for tests or simple CLI use.
    pub fn new_default_blocking() -> anyhow::Result<Self> {
        pollster::block_on(Self::new_default_async())
    }

    /// Compile a WGSL shader module from the given source string.
    pub fn compile_wgsl(&self, source: &str, label: Option<&str>) -> wgpu::ShaderModule {
        self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label,
            source: wgpu::ShaderSource::Wgsl(source.into()),
        })
    }

    /// Run a compute shader that reads from an input RGBA image, writes to an
    /// output RGBA image, and (optionally) takes a uniform buffer. This wraps
    /// the common flow (upload, create pipeline, dispatch, readback) that many
    /// GPU operations use. The shader should accept bindings:
    ///  - 0: sampled texture (read-only)
    ///  - 1: storage texture (write-only)
    ///  - 2: uniform buffer (optional)
    ///
    /// The `work_group` argument describes the compute workgroup size used for
    /// calculating dispatch counts (e.g., (8,8)).
    pub fn run_compute_with_image_io(
        &self,
        shader_source: &str,
        shader_label: Option<&str>,
        entry_point: &str,
        in_pixels: &[u8],
        width: u32,
        height: u32,
        work_group: (u32, u32),
        uniform_bytes: Option<&[u8]>,
        in_format: wgpu::TextureFormat,
        out_format: wgpu::TextureFormat,
    ) -> anyhow::Result<Vec<u8>> {
        // Create textures
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let in_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gpu::compute_input"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: in_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let out_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gpu::compute_output"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: out_format,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Write input pixels into input texture
        let bytes_per_row = 4u32 * width;
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &in_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            in_pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bytes_per_row), rows_per_image: Some(height) },
            size,
        );

        // Shader & pipeline
        let shader = self.compile_wgsl(shader_source, shader_label);

        // Build bind group layout entries
        let mut entries = vec![
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: out_format, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
        ];
        if uniform_bytes.is_some() {
            entries.push(wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None });
        }

        let bgl = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("compute::bgl"), entries: &entries });
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("compute::pl"), bind_group_layouts: &[&bgl], push_constant_ranges: &[] });
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some("compute::pipeline"), layout: Some(&pipeline_layout), module: &shader, entry_point: Some(entry_point), cache: None, compilation_options: wgpu::PipelineCompilationOptions::default() });

        // Bind group entries
        let in_view = in_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let out_view = out_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut bg_entries = vec![
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&in_view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&out_view) },
        ];
        let mut uniform_buf: Option<wgpu::Buffer> = None;
        if let Some(data) = uniform_bytes {
            let buf = (&*self.device).create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("compute::uniform"), contents: data, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST });
            uniform_buf = Some(buf);
        }
        if let Some(ref ub) = uniform_buf {
            bg_entries.push(wgpu::BindGroupEntry { binding: 2, resource: ub.as_entire_binding() });
        }

        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("compute::bg"), layout: &bgl, entries: &bg_entries });

        // Dispatch
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("compute::enc") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("compute::pass"), ..Default::default() });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            let x_groups = (width + (work_group.0 - 1)) / work_group.0;
            let y_groups = (height + (work_group.1 - 1)) / work_group.1;
            pass.dispatch_workgroups(x_groups as u32, y_groups as u32, 1);
        }
        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::PollType::wait_indefinitely())?;

        // Readback using the GpuImage helper
        let out_img = crate::image::GpuImage { texture: out_texture, view: out_view, width, height, format: out_format };
        let img = out_img.to_image_blocking(self)?;
        Ok(img.into_rgba_vec())
    }
}
