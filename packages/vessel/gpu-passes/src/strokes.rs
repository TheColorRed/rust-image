//! A picture that lines are drawn into a piece at a time, like a pen on paper.
//!
//! A [`GpuPass`](crate::GpuPass) makes a whole picture from its input, so drawing a line that grows by a point with one costs the
//! size of the picture every time. A [`StrokeCanvas`] keeps its texture between frames and draws only the segments it is given, each
//! as a small quad, so a frame costs the length of what is new. Where segments overlap the larger coverage wins, so a line drawn
//! in many pieces is the same as one drawn at once, with no beads at its joints.

use crate::context::{FORMAT, GpuContext};

/// How segments are drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
  /// Half the width of the line, in pixels.
  pub radius: f32,
  /// How wide the fade at the edge is, in pixels. Values under one are drawn as one, so the edge is never jagged.
  pub edge: f32,
  /// The color, straight (not multiplied by its alpha), each from 0 to 1.
  pub color: [f32; 4],
}

/// A texture of lines, drawn a segment at a time.
pub struct StrokeCanvas {
  ctx: GpuContext,
  texture: wgpu::Texture,
  pipeline: wgpu::RenderPipeline,
}

impl StrokeCanvas {
  /// A clear canvas of `p_width` x `p_height` pixels.
  pub fn new(p_ctx: &GpuContext, p_width: u32, p_height: u32) -> StrokeCanvas {
    let device = &p_ctx.device;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("strokes::canvas"),
      size: wgpu::Extent3d { width: p_width.max(1), height: p_height.max(1), depth_or_array_layers: 1 },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: FORMAT,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
      view_formats: &[],
    });
    let shader = p_ctx.compile_wgsl(include_str!("./strokes.wgsl"), Some("strokes::shader"));
    // The larger of what is there and what is drawn, for the color and for the coverage.
    let keep_larger = wgpu::BlendComponent {
      src_factor: wgpu::BlendFactor::One,
      dst_factor: wgpu::BlendFactor::One,
      operation: wgpu::BlendOperation::Max,
    };
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("strokes::pipeline"),
      layout: None,
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: Some("vs"),
        compilation_options: Default::default(),
        buffers: &[Some(wgpu::VertexBufferLayout {
          array_stride: 16,
          step_mode: wgpu::VertexStepMode::Instance,
          attributes: &wgpu::vertex_attr_array![0 => Float32x4],
        })],
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs"),
        compilation_options: Default::default(),
        targets: &[Some(wgpu::ColorTargetState {
          format: FORMAT,
          blend: Some(wgpu::BlendState { color: keep_larger, alpha: keep_larger }),
          write_mask: wgpu::ColorWrites::ALL,
        })],
      }),
      primitive: wgpu::PrimitiveState::default(),
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview_mask: None,
      cache: None,
    });
    let canvas = StrokeCanvas { ctx: p_ctx.clone(), texture, pipeline };
    canvas.clear();
    canvas
  }

  /// The size of the canvas, in pixels.
  pub fn size(&self) -> (u32, u32) {
    (self.texture.width(), self.texture.height())
  }

  /// The texture that holds the lines. It changes as segments are drawn.
  pub fn texture(&self) -> &wgpu::Texture {
    &self.texture
  }

  /// Takes every line away.
  pub fn clear(&self) {
    let view = self.texture.create_view(&Default::default());
    let mut encoder = self.ctx.device.create_command_encoder(&Default::default());
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("strokes::clear"),
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
      })],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
      multiview_mask: None,
    });
    self.ctx.submit(encoder.finish());
  }

  /// Draws each segment (`[from_x, from_y, to_x, to_y]`, in pixels of the canvas) over what is there. A segment whose two ends are
  /// the same is a dot.
  pub fn draw(&self, p_segments: &[[f32; 4]], p_style: &StrokeStyle) {
    if p_segments.is_empty() {
      return;
    }
    let (width, height) = self.size();
    let uniforms: Vec<u8> = p_style
      .color
      .into_iter()
      .chain([p_style.radius, p_style.edge.max(1.0), width as f32, height as f32])
      .flat_map(f32::to_ne_bytes)
      .collect();
    // The buffers are filled as they are made, so the GPU never reads one that a copy is still writing.
    let filled = |label: &str, usage: wgpu::BufferUsages, bytes: &[u8]| {
      let buffer = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes.len() as u64,
        usage,
        mapped_at_creation: true,
      });
      buffer.slice(..).get_mapped_range_mut().expect("the buffer was made mapped").copy_from_slice(bytes);
      buffer.unmap();
      buffer
    };
    let style = filled("strokes::style", wgpu::BufferUsages::UNIFORM, &uniforms);
    let segments = filled("strokes::segments", wgpu::BufferUsages::VERTEX, bytemuck::cast_slice(p_segments));
    let bind_group = self.ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("strokes::bind"),
      layout: &self.pipeline.get_bind_group_layout(0),
      entries: &[wgpu::BindGroupEntry { binding: 0, resource: style.as_entire_binding() }],
    });
    let view = self.texture.create_view(&Default::default());
    let mut encoder = self.ctx.device.create_command_encoder(&Default::default());
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("strokes::draw"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          depth_slice: None,
          resolve_target: None,
          ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
      });
      pass.set_pipeline(&self.pipeline);
      pass.set_bind_group(0, &bind_group, &[]);
      pass.set_vertex_buffer(0, segments.slice(..));
      pass.draw(0..6, 0..p_segments.len() as u32);
    }
    self.ctx.submit(encoder.finish());
  }
}
