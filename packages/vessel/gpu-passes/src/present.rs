//! Presents a rendered texture to a native window surface without reading it back to the CPU.

use crate::context::{FORMAT, GpuContext};
use anyhow::{Result, anyhow};

const BLIT_SHADER: &str = r#"
struct VsOut {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) index: u32) -> VsOut {
  // One oversized triangle covers the whole target.
  let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
  var out: VsOut;
  out.position = vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
  out.uv = vec2<f32>(corner.x, 1.0 - corner.y);
  return out;
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
  return textureSample(source, source_sampler, in.uv);
}
"#;

/// Draws textures onto a window surface. Effects work on sRGB-encoded values, so the surface uses a non-sRGB format and
/// the pixels are copied through unchanged.
pub struct Presenter {
  ctx: GpuContext,
  surface: wgpu::Surface<'static>,
  config: wgpu::SurfaceConfiguration,
  pipeline: wgpu::RenderPipeline,
  sampler: wgpu::Sampler,
  uploaded: Option<(wgpu::Texture, u32, u32)>,
}

impl Presenter {
  /// Creates a presenter for a window.
  ///
  /// # Safety
  /// The handles in `p_target` must stay valid until the presenter is dropped.
  pub unsafe fn new(
    p_ctx: &GpuContext, p_target: wgpu::SurfaceTargetUnsafe, p_width: u32, p_height: u32,
  ) -> Result<Presenter> {
    let surface = unsafe { p_ctx.instance.create_surface_unsafe(p_target)? };
    let caps = surface.get_capabilities(&p_ctx.adapter);
    let format = caps
      .formats
      .iter()
      .copied()
      .find(|format| !format.is_srgb())
      .or_else(|| caps.formats.first().copied())
      .ok_or_else(|| anyhow!("surface supports no formats"))?;
    let mut config = surface
      .get_default_config(&p_ctx.adapter, p_width.max(1), p_height.max(1))
      .ok_or_else(|| anyhow!("surface is not supported by the adapter"))?;
    config.format = format;
    #[cfg(target_os = "ios")]
    {
      config.alpha_mode = [
        wgpu::CompositeAlphaMode::PreMultiplied,
        wgpu::CompositeAlphaMode::PostMultiplied,
      ]
      .into_iter()
      .find(|mode| caps.alpha_modes.contains(mode))
      .ok_or_else(|| anyhow!("iOS surface supports no transparent alpha mode"))?;
    }
    // Never wait for vsync: a slider drag should show the newest frame, and the caller's thread must not block.
    config.present_mode = wgpu::PresentMode::AutoNoVsync;
    p_ctx.configure(&surface, &config);

    let device = &p_ctx.device;
    // Metal in wgpu 30 labels its transparent mode PostMultiplied, but Core Animation expects premultiplied pixels
    // (gfx-rs/wgpu#9896). Both advertised modes need premultiplication on our iOS layer.
    let shader_source = if cfg!(target_os = "ios") || config.alpha_mode == wgpu::CompositeAlphaMode::PreMultiplied {
      BLIT_SHADER.replace(
        "return textureSample(source, source_sampler, in.uv);",
        "let color = textureSample(source, source_sampler, in.uv);\n  return vec4<f32>(color.rgb * color.a, color.a);",
      )
    } else {
      BLIT_SHADER.to_string()
    };
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("present::blit"),
      source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("present::pipeline"),
      layout: None,
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: Some("vs"),
        compilation_options: Default::default(),
        buffers: &[],
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs"),
        compilation_options: Default::default(),
        targets: &[Some(format.into())],
      }),
      primitive: wgpu::PrimitiveState::default(),
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview_mask: None,
      cache: None,
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      mag_filter: wgpu::FilterMode::Linear,
      min_filter: wgpu::FilterMode::Linear,
      ..Default::default()
    });
    Ok(Presenter {
      ctx: p_ctx.clone(),
      surface,
      config,
      pipeline,
      sampler,
      uploaded: None,
    })
  }

  /// Creates a presenter for an Android `ANativeWindow`.
  ///
  /// # Safety
  /// `p_window` must be a valid `ANativeWindow` pointer that stays valid until the presenter is dropped.
  pub unsafe fn from_android_window(
    p_ctx: &GpuContext, p_window: *mut std::ffi::c_void, p_width: u32, p_height: u32,
  ) -> Result<Presenter> {
    let window = std::ptr::NonNull::new(p_window).ok_or_else(|| anyhow!("null window"))?;
    let target = wgpu::SurfaceTargetUnsafe::RawHandle {
      raw_display_handle: Some(wgpu::rwh::RawDisplayHandle::Android(wgpu::rwh::AndroidDisplayHandle::new())),
      raw_window_handle: wgpu::rwh::RawWindowHandle::AndroidNdk(wgpu::rwh::AndroidNdkWindowHandle::new(window)),
    };
    unsafe { Presenter::new(p_ctx, target, p_width, p_height) }
  }

  /// Creates a presenter for an iOS host's `CAMetalLayer`.
  ///
  /// # Safety
  /// `p_layer` must be a retained `CAMetalLayer` that stays valid until this presenter is dropped.
  #[cfg(target_os = "ios")]
  pub unsafe fn from_metal_layer(
    p_ctx: &GpuContext, p_layer: *mut std::ffi::c_void, p_width: u32, p_height: u32,
  ) -> Result<Presenter> {
    if p_layer.is_null() {
      return Err(anyhow!("null Metal layer"));
    }
    unsafe { Presenter::new(p_ctx, wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(p_layer), p_width, p_height) }
  }

  /// Follows a change in the window's size.
  pub fn resize(&mut self, p_width: u32, p_height: u32) {
    let (width, height) = (p_width.max(1), p_height.max(1));
    if (self.config.width, self.config.height) != (width, height) {
      self.config.width = width;
      self.config.height = height;
      self.ctx.configure(&self.surface, &self.config);
    }
  }

  /// Draws `p_texture` (an `Rgba8Unorm` texture with `TEXTURE_BINDING`) over the whole surface. Returns immediately;
  /// a frame the surface cannot take right now is skipped.
  pub fn present(&mut self, p_texture: &wgpu::Texture) -> Result<()> {
    self.try_present(p_texture).map(|_| ())
  }

  /// Presents a texture and reports whether a frame was actually submitted, rather than treating a skipped frame as drawn.
  pub fn try_present(&mut self, p_texture: &wgpu::Texture) -> Result<bool> {
    let frame = match self.surface.get_current_texture() {
      wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
      wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
        self.ctx.configure(&self.surface, &self.config);
        return Ok(false);
      }
      wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return Ok(false),
      wgpu::CurrentSurfaceTexture::Validation => return Err(anyhow!("surface rejected the frame")),
    };
    let device = &self.ctx.device;
    let source = p_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("present::bind_group"),
      layout: &self.pipeline.get_bind_group_layout(0),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(&source),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::Sampler(&self.sampler),
        },
      ],
    });
    let target = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("present::encoder"),
    });
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("present::pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &target,
          depth_slice: None,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
      });
      pass.set_pipeline(&self.pipeline);
      pass.set_bind_group(0, &bind_group, &[]);
      pass.draw(0..3, 0..1);
    }
    self.ctx.submit(encoder.finish());
    self.ctx.queue.present(frame);
    Ok(true)
  }

  /// Uploads `p_rgba` (`p_width * p_height` pixels) and presents it. For frames computed on the CPU.
  pub fn present_pixels(&mut self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> Result<()> {
    self.try_present_pixels(p_width, p_height, p_rgba).map(|_| ())
  }

  /// Uploads pixels and reports whether a frame was actually submitted to the surface.
  pub fn try_present_pixels(&mut self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> Result<bool> {
    let expected = (p_width as usize).checked_mul(p_height as usize).and_then(|size| size.checked_mul(4));
    if p_width == 0 || p_height == 0 || expected != Some(p_rgba.len()) {
      return Err(anyhow!("invalid frame dimensions or RGBA length"));
    }
    if self.uploaded.as_ref().map(|(_, w, h)| (*w, *h)) != Some((p_width, p_height)) {
      let texture = self.ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("present::upload"),
        size: wgpu::Extent3d {
          width: p_width,
          height: p_height,
          depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
      });
      self.uploaded = Some((texture, p_width, p_height));
    }
    let (texture, _, _) = self.uploaded.as_ref().expect("just created");
    self.ctx.queue.write_texture(
      texture.as_image_copy(),
      p_rgba,
      wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(4 * p_width),
        rows_per_image: Some(p_height),
      },
      wgpu::Extent3d {
        width: p_width,
        height: p_height,
        depth_or_array_layers: 1,
      },
    );
    let texture = texture.clone();
    self.try_present(&texture)
  }
}
