//! Presents a rendered texture to a native window surface without reading it back to the CPU.

use crate::context::{FORMAT, GpuContext};
use anyhow::{Result, anyhow};

const BLIT_SHADER: &str = r#"
struct VsOut {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

struct Present {
  // The part of the source to show, as x, y, width, height in 0..1 of the source.
  view: vec4<f32>,
  // The color of what is past the source, straight RGBA.
  outside: vec4<f32>,
};
@group(0) @binding(2) var<uniform> present: Present;

@vertex
fn vs(@builtin(vertex_index) index: u32) -> VsOut {
  // One oversized triangle covers the whole target.
  let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
  var out: VsOut;
  out.position = vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
  out.uv = present.view.xy + vec2<f32>(corner.x, 1.0 - corner.y) * present.view.zw;
  return out;
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
  var color = textureSample(source, source_sampler, in.uv);
  // A view can reach past the picture (zooming out, or panning off an edge): that part is the outside color, not the edge
  // stretched.
  if (any(in.uv < vec2<f32>(0.0)) || any(in.uv > vec2<f32>(1.0))) {
    color = present.outside;
  }
  return color;
}
"#;

/// How a [`Presenter`] draws. [`Default`] never waits for vsync and shows black past a picture; an app changes either to
/// suit what it shows (a game or a video wants vsync, an editor on a light page may want its page color).
#[derive(Clone, Debug, PartialEq)]
pub struct PresentOptions {
  /// When a frame reaches the screen. `AutoNoVsync` (the default) shows the newest frame at once, which suits dragging a
  /// slider; `AutoVsync` waits for the screen, which suits steady animation and saves power. A mode the surface does not
  /// support is refused by [`Presenter::set_options`].
  pub present_mode: wgpu::PresentMode,
  /// The color, straight RGBA from 0 to 1, of what a view shows past the picture. Default opaque black.
  pub outside: [f32; 4],
}

impl Default for PresentOptions {
  fn default() -> Self {
    Self { present_mode: wgpu::PresentMode::AutoNoVsync, outside: [0.0, 0.0, 0.0, 1.0] }
  }
}

impl PresentOptions {
  /// Sets [`present_mode`](Self::present_mode).
  pub fn with_present_mode(mut self, p_mode: wgpu::PresentMode) -> Self {
    self.present_mode = p_mode;
    self
  }

  /// Sets [`outside`](Self::outside).
  pub fn with_outside(mut self, p_rgba: [f32; 4]) -> Self {
    self.outside = p_rgba;
    self
  }
}

/// Draws textures onto a window surface. Effects work on sRGB-encoded values, so the surface uses a non-sRGB format and
/// the pixels are copied through unchanged.
pub struct Presenter {
  ctx: GpuContext,
  surface: wgpu::Surface<'static>,
  config: wgpu::SurfaceConfiguration,
  pipeline: wgpu::RenderPipeline,
  sampler: wgpu::Sampler,
  view_buffer: wgpu::Buffer,
  options: PresentOptions,
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
    // By default never wait for vsync: a slider drag should show the newest frame, and the caller's thread must not block.
    // `set_options` changes it.
    config.present_mode = PresentOptions::default().present_mode;
    p_ctx.configure(&surface, &config);

    let device = &p_ctx.device;
    // Metal in wgpu 30 labels its transparent mode PostMultiplied, but Core Animation expects premultiplied pixels
    // (gfx-rs/wgpu#9896). Both advertised modes need premultiplication on our iOS layer.
    let shader_source = if cfg!(target_os = "ios") || config.alpha_mode == wgpu::CompositeAlphaMode::PreMultiplied {
      BLIT_SHADER.replace(
        "return color;",
        "return vec4<f32>(color.rgb * color.a, color.a);",
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
    let view_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("present::view"),
      size: 32,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    Ok(Presenter {
      ctx: p_ctx.clone(),
      surface,
      config,
      pipeline,
      sampler,
      view_buffer,
      options: PresentOptions::default(),
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

  /// Changes how this presenter draws. A present mode the surface does not support is an error and the mode stays as it was;
  /// the rest of the options are still applied.
  pub fn set_options(&mut self, p_options: &PresentOptions) -> Result<()> {
    self.options = PresentOptions { present_mode: self.config.present_mode, ..p_options.clone() };
    if p_options.present_mode != self.config.present_mode {
      let supported = self.surface.get_capabilities(&self.ctx.adapter).present_modes;
      let automatic = matches!(p_options.present_mode, wgpu::PresentMode::AutoVsync | wgpu::PresentMode::AutoNoVsync);
      if !automatic && !supported.contains(&p_options.present_mode) {
        return Err(anyhow!("the surface does not support the present mode {:?}", p_options.present_mode));
      }
      self.config.present_mode = p_options.present_mode;
      self.options.present_mode = p_options.present_mode;
      self.ctx.configure(&self.surface, &self.config);
    }
    Ok(())
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
    self.try_present_view(p_texture, [0.0, 0.0, 1.0, 1.0])
  }

  /// Presents only part of a texture: `p_view` is `[x, y, width, height]` in 0..1 of the texture. The GPU does the crop and
  /// the scaling, so a zoom costs one draw however large the texture is. A view may reach past 0..1; what falls outside the
  /// texture is drawn black.
  pub fn try_present_view(&mut self, p_texture: &wgpu::Texture, p_view: [f32; 4]) -> Result<bool> {
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
    self.ctx.queue.write_buffer(&self.view_buffer, 0, &[p_view, self.options.outside].concat().iter().flat_map(|value| value.to_ne_bytes()).collect::<Vec<u8>>());
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
        wgpu::BindGroupEntry {
          binding: 2,
          resource: self.view_buffer.as_entire_binding(),
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
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: self.options.outside[0] as f64,
              g: self.options.outside[1] as f64,
              b: self.options.outside[2] as f64,
              a: self.options.outside[3] as f64,
            }),
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::target::TextureFrame;
  use vessel_engine::GpuFrame;

  #[test]
  fn a_view_shows_only_that_part_of_the_texture() -> Result<()> {
    // The right half is pure blue.
    assert_eq!(shown_pixel([0.5, 0.0, 0.5, 1.0], [0.0, 0.0, 0.0, 1.0])?, vec![0, 0, 255, 255]);
    Ok(())
  }

  #[test]
  fn what_a_view_shows_past_the_texture_is_the_outside_color() -> Result<()> {
    // The view starts at the texture's right edge, so the one pixel shown is past it: not the edge stretched.
    assert_eq!(shown_pixel([1.0, 0.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0])?, vec![0, 0, 0, 255]);
    assert_eq!(shown_pixel([1.0, 0.0, 1.0, 1.0], [1.0, 0.0, 0.0, 1.0])?, vec![255, 0, 0, 255]);
    assert_eq!(shown_pixel([-1.0, 0.0, 1.0, 1.0], [0.0, 1.0, 0.0, 0.0])?, vec![0, 255, 0, 0]);
    Ok(())
  }

  /// The one pixel a view of a four pixel picture (red, red, blue, blue) is drawn as, with `p_outside` past the picture.
  fn shown_pixel(p_view: [f32; 4], p_outside: [f32; 4]) -> Result<Vec<u8>> {
    let ctx = GpuContext::new_default_blocking()?;
    let device = &ctx.device;
    let size = |width| wgpu::Extent3d { width, height: 1, depth_or_array_layers: 1 };
    let make = |width, usage| {
      device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: size(width),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage,
        view_formats: &[],
      })
    };
    // Four pixels: red, red, blue, blue. The right half is pure blue.
    let source = make(4, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
    let pixels = [255u8, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255];
    ctx.queue.write_texture(
      source.as_image_copy(),
      &pixels,
      wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(16), rows_per_image: Some(1) },
      size(4),
    );
    let target = make(1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: None,
      source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: None,
      layout: None,
      vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs"),
        compilation_options: Default::default(),
        targets: &[Some(FORMAT.into())],
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
    let view_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: None,
      size: 32,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    ctx.queue.write_buffer(&view_buffer, 0, &[p_view, p_outside].concat().iter().flat_map(|value| value.to_ne_bytes()).collect::<Vec<u8>>());
    let source_view = source.create_view(&Default::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &pipeline.get_bind_group_layout(0),
      entries: &[
        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&source_view) },
        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
        wgpu::BindGroupEntry { binding: 2, resource: view_buffer.as_entire_binding() },
      ],
    });
    let target_view = target.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &target_view,
          depth_slice: None,
          resolve_target: None,
          ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
      });
      pass.set_pipeline(&pipeline);
      pass.set_bind_group(0, &bind_group, &[]);
      pass.draw(0..3, 0..1);
    }
    ctx.submit(encoder.finish());
    let shown = TextureFrame::new(&ctx, target).read_back().ok_or_else(|| anyhow!("read back failed"))?;
    Ok(shown.pixels.to_vec())
  }
}
