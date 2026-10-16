//! Persistent GPU renderer.
//!
//! [`LiveRenderer`] keeps pipelines, textures and uniform buffers alive between frames, so re-rendering after a
//! parameter change only writes a few bytes and re-encodes the passes. Effects are given as [`GpuPass`]es; the
//! renderer knows nothing about specific effects.
use crate::context::{FORMAT, GpuContext, WORKGROUP_SIZE};
use crate::{GpuPass, GpuSession};
use anyhow::{Result, anyhow};

/// Smallest uniform buffer allocated, and the granularity buffers are rounded up to.
const UNIFORM_ALIGN: usize = 16;

/// A texture with its default view.
struct Target {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
}

impl Target {
  fn new(p_ctx: &GpuContext, p_label: &str, p_width: u32, p_height: u32, p_usage: wgpu::TextureUsages) -> Target {
    Target::new_in(p_ctx, p_label, p_width, p_height, p_usage, FORMAT)
  }

  fn new_in(
    p_ctx: &GpuContext, p_label: &str, p_width: u32, p_height: u32, p_usage: wgpu::TextureUsages,
    p_format: wgpu::TextureFormat,
  ) -> Target {
    let texture = p_ctx.device.create_texture(&wgpu::TextureDescriptor {
      label: Some(p_label),
      size: wgpu::Extent3d {
        width: p_width,
        height: p_height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: p_format,
      usage: p_usage,
      view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Target { texture, view }
  }

  fn write(&self, p_ctx: &GpuContext, p_width: u32, p_height: u32, p_rgba: &[u8]) {
    p_ctx.queue.write_texture(
      wgpu::TexelCopyTextureInfo {
        texture: &self.texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
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
  }
}

#[derive(Hash, PartialEq, Eq)]
pub(crate) struct PipelineKey {
  shader: &'static str,
  uniforms: bool,
  aux: bool,
  base: bool,
  window: bool,
  float_out: bool,
}

/// The format of the working texture a shader writes to its output at binding 1: `rgba32float` when the shader declares it
/// (`texture_storage_2d<rgba32float, write>`), and `rgba8unorm` otherwise. A pass that writes `rgba32float` keeps real numbers
/// (sums, means, variances) that 8 bits cannot hold, for a later pass to read. The shader says what it writes, so a pass needs
/// nothing more to ask for it.
fn writes_float(p_pass: &GpuPass) -> bool {
  p_pass.shader.contains("texture_storage_2d<rgba32float")
}

impl PipelineKey {
  fn of(p_pass: &GpuPass) -> PipelineKey {
    PipelineKey {
      shader: p_pass.shader,
      uniforms: !p_pass.uniforms.is_empty(),
      aux: p_pass.aux.is_some(),
      base: p_pass.base,
      window: p_pass.window,
      float_out: writes_float(p_pass),
    }
  }
}

#[derive(Clone)]
pub(crate) struct Pipeline {
  pipeline: wgpu::ComputePipeline,
  layout: wgpu::BindGroupLayout,
}

struct UniformSlot {
  buffer: wgpu::Buffer,
  size: usize,
}

struct AuxSlot {
  target: Target,
  width: u32,
  height: u32,
  rgba: std::sync::Arc<[u8]>,
}

/// Number of staging buffers readback rotates through unless [`LiveRenderer::with_readback_slots`] says otherwise, so a new
/// frame can be requested while earlier ones are still
/// being copied back.
const READBACK_SLOTS: usize = 3;

/// A finished frame from [`LiveRenderer::poll_frame`].
#[derive(Clone, Debug)]
pub struct Frame {
  /// Increases with every [`LiveRenderer::request_frame`], so a newer frame always has a larger id.
  pub id: u64,
  /// Width in pixels.
  pub width: u32,
  /// Height in pixels.
  pub height: u32,
  /// `width * height` RGBA pixels.
  pub pixels: Vec<u8>,
}

struct PendingFrame {
  id: u64,
  receiver: std::sync::mpsc::Receiver<std::result::Result<(), wgpu::BufferAsyncError>>,
  done: Option<std::result::Result<(), wgpu::BufferAsyncError>>,
  /// A newer frame has finished, so this one is discarded when it completes.
  stale: bool,
}

struct ReadbackSlot {
  buffer: wgpu::Buffer,
  pending: Option<PendingFrame>,
}

struct Readback {
  width: u32,
  height: u32,
  padded_row: usize,
  slots: Vec<ReadbackSlot>,
}

/// Where the last [`LiveRenderer::render`] left its result.
#[derive(Clone, Copy)]
enum Output {
  Source,
  Work(usize),
}

/// Renders a source image through a chain of effects, each given as its [`GpuPass`]es, on the GPU, reusing every GPU resource across frames.
///
/// Typical live use:
/// 1. [`set_source`](Self::set_source) once when the image changes.
/// 2. [`render`](Self::render) whenever an effect parameter changes.
/// 3. Present [`output_texture`](Self::output_texture), or read the result back: [`read_pixels`](Self::read_pixels)
///    blocks and suits export, while [`request_frame`](Self::request_frame) and [`poll_frame`](Self::poll_frame) never
///    block and suit interactive editing.
pub struct LiveRenderer {
  ctx: GpuContext,
  source: Option<(Target, u32, u32)>,
  work: Option<[Target; 2]>,
  /// The working textures of the passes that write `rgba32float` (see `writes_float`), made when a chain has one.
  float_work: Option<[Target; 2]>,
  /// A copy of the image an effect started from, for passes that combine the effect's result with its input.
  base: Option<Target>,
  uniforms: Vec<Option<UniformSlot>>,
  aux: Vec<Option<AuxSlot>>,
  readback: Option<Readback>,
  next_frame: u64,
  output: Output,
  /// Where the next render's image sits in the photo, for the passes that ask: `[origin x, origin y, scale, scale y, photo
  /// width, photo height, 0, 0]`. Taken by the render; without it the image is the whole photo.
  window: Option<[f32; 8]>,
  window_buffer: Option<wgpu::Buffer>,
  readback_slots: usize,
}

impl LiveRenderer {
  /// Creates a renderer on the given context.
  pub fn new(p_ctx: GpuContext) -> LiveRenderer {
    LiveRenderer {
      ctx: p_ctx,
      source: None,
      work: None,
      float_work: None,
      base: None,
      uniforms: Vec::new(),
      aux: Vec::new(),
      readback: None,
      next_frame: 0,
      output: Output::Source,
      window: None,
      window_buffer: None,
      readback_slots: READBACK_SLOTS,
    }
  }

  /// How many staging buffers readback rotates through (at least one; the default is 3). More let a caller that reads back
  /// every frame have more in flight at once, at the cost of memory for each; one means a frame waits for the one before.
  /// Set it before the first [`request_frame`](Self::request_frame).
  pub fn with_readback_slots(mut self, p_slots: usize) -> Self {
    self.readback_slots = p_slots.max(1);
    self
  }

  /// The context this renderer runs on.
  pub fn context(&self) -> &GpuContext {
    &self.ctx
  }

  /// The number of distinct shaders compiled so far.
  pub fn cached_pipelines(&self) -> usize {
    self.ctx.pipelines.lock().map_or(0, |pipelines| pipelines.len())
  }

  /// The source image size, or `None` before [`set_source`](Self::set_source).
  pub fn size(&self) -> Option<(u32, u32)> {
    self.source.as_ref().map(|(_, width, height)| (*width, *height))
  }

  /// Uploads the image that effects are applied to. Working textures are only reallocated when the size changes.
  /// - `p_rgba`: `p_width * p_height` RGBA pixels.
  pub fn set_source(&mut self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> Result<()> {
    if p_width == 0 || p_height == 0 {
      return Err(anyhow!("image must not be empty"));
    }
    if p_rgba.len() != (p_width as usize) * (p_height as usize) * 4 {
      return Err(anyhow!(
        "expected {} bytes of RGBA, got {}",
        (p_width as usize) * (p_height as usize) * 4,
        p_rgba.len()
      ));
    }
    self.ensure_size(p_width, p_height);
    if let Some((target, width, height)) = &self.source {
      target.write(&self.ctx, *width, *height, p_rgba);
    }
    self.output = Output::Source;
    Ok(())
  }

  /// Makes the source and working textures `p_width` x `p_height`, keeping them when they already are.
  fn ensure_size(&mut self, p_width: u32, p_height: u32) {
    if self.size() == Some((p_width, p_height)) {
      return;
    }
    let usage = wgpu::TextureUsages::TEXTURE_BINDING
      | wgpu::TextureUsages::COPY_DST
      | wgpu::TextureUsages::COPY_SRC
      | wgpu::TextureUsages::RENDER_ATTACHMENT;
    self.source = Some((Target::new(&self.ctx, "live::source", p_width, p_height, usage), p_width, p_height));
    let work_usage =
      wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC;
    self.work = Some([
      Target::new(&self.ctx, "live::work_a", p_width, p_height, work_usage),
      Target::new(&self.ctx, "live::work_b", p_width, p_height, work_usage),
    ]);
    let base_usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    self.base = Some(Target::new(&self.ctx, "live::base", p_width, p_height, base_usage));
    self.float_work = None;
  }

  /// The source texture at `p_width` x `p_height`, for something other than an upload to draw the image into. The next
  /// [`render`](Self::render) runs the effects over what was drawn.
  pub(crate) fn source_view(&mut self, p_width: u32, p_height: u32) -> wgpu::TextureView {
    self.ensure_size(p_width, p_height);
    self.output = Output::Source;
    self.source.as_ref().expect("just made").0.view.clone()
  }

  /// Says where the image of the next [`render`](Self::render) sits in the photo, for the passes that ask.
  pub(crate) fn set_window(&mut self, p_window: [f32; 8]) {
    self.window = Some(p_window);
  }

  /// Runs the effects over the source image in order. This only submits GPU work and does not wait for it, so it is
  /// cheap to call every frame. Use [`read_pixels`](Self::read_pixels) or [`output_texture`](Self::output_texture)
  /// to get the result.
  pub fn render(&mut self, p_effects: &[Vec<GpuPass>]) -> Result<()> {
    let (width, height) = self.size().ok_or_else(|| anyhow!("set_source must be called before render"))?;
    let window = self.window.take().unwrap_or([0.0, 0.0, 1.0, 1.0, width as f32, height as f32, 0.0, 0.0]);
    let window_buffer = self
      .window_buffer
      .get_or_insert_with(|| {
        self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
          label: Some("live::window"),
          size: 32,
          usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
          mapped_at_creation: false,
        })
      })
      .clone();
    self.ctx.queue.write_buffer(&window_buffer, 0, &window.map(f32::to_ne_bytes).concat());
    // An effect with a pass that needs its input image has that image copied aside before its first pass runs.
    let mut passes: Vec<(&GpuPass, bool)> = Vec::new();
    for effect_passes in p_effects {
      let needs_base = effect_passes.iter().any(|pass| pass.base);
      for (index, pass) in effect_passes.iter().enumerate() {
        passes.push((pass, needs_base && index == 0));
      }
    }

    // A pass that writes `rgba32float` hands its numbers to the pass after it, so a chain cannot end on one, and a pass that wants the
    // effect's input image (`base`, which is 8-bit) cannot come straight after one.
    let floats: Vec<bool> = passes.iter().map(|(pass, _)| writes_float(pass)).collect();
    if floats.last() == Some(&true) {
      return Err(anyhow!("a chain of passes must end with a pass that writes rgba8unorm, not rgba32float"));
    }
    if passes.iter().enumerate().any(|(ordinal, (pass, snapshot))| (*snapshot || pass.base) && ordinal > 0 && floats[ordinal - 1]) {
      return Err(anyhow!("a pass that reads the effect's input image cannot follow a pass that writes rgba32float"));
    }
    if floats.contains(&true) && self.float_work.is_none() {
      let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING;
      let make = |label: &str| Target::new_in(&self.ctx, label, width, height, usage, wgpu::TextureFormat::Rgba32Float);
      self.float_work = Some([make("live::float_a"), make("live::float_b")]);
    }

    for (ordinal, (pass, _)) in passes.iter().enumerate() {
      self.ensure_pipeline(pass);
      self.write_uniforms(ordinal, pass);
      self.write_aux(ordinal, pass);
    }

    let float_work = &self.float_work;
    let (Some((source, _, _)), Some(work), Some(base)) = (&self.source, &self.work, &self.base) else {
      return Err(anyhow!("set_source must be called before render"));
    };
    let mut encoder = self.ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("live::enc"),
    });
    for (ordinal, (pass, snapshot)) in passes.iter().enumerate() {
      let pipeline = self.pipeline(&PipelineKey::of(pass))?;
      let previous = (ordinal + 1) % 2;
      let (input_texture, input) = if ordinal == 0 {
        (&source.texture, &source.view)
      } else if floats[ordinal - 1] {
        let held = &float_work.as_ref().expect("made for the chain")[previous];
        (&held.texture, &held.view)
      } else {
        (&work[previous].texture, &work[previous].view)
      };
      let output = if floats[ordinal] {
        &float_work.as_ref().expect("made for the chain")[ordinal % 2].view
      } else {
        &work[ordinal % 2].view
      };

      if *snapshot {
        encoder.copy_texture_to_texture(
          wgpu::TexelCopyTextureInfo {
            texture: input_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
          },
          wgpu::TexelCopyTextureInfo {
            texture: &base.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
          },
          wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
          },
        );
      }

      let mut entries = vec![
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(input),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::TextureView(output),
        },
      ];
      if !pass.uniforms.is_empty()
        && let Some(slot) = &self.uniforms[ordinal]
      {
        entries.push(wgpu::BindGroupEntry {
          binding: 2,
          resource: slot.buffer.as_entire_binding(),
        });
      }
      if pass.aux.is_some()
        && let Some(slot) = &self.aux[ordinal]
      {
        entries.push(wgpu::BindGroupEntry {
          binding: 3,
          resource: wgpu::BindingResource::TextureView(&slot.target.view),
        });
      }
      if pass.base {
        entries.push(wgpu::BindGroupEntry {
          binding: 4,
          resource: wgpu::BindingResource::TextureView(&base.view),
        });
      }
      if pass.window {
        entries.push(wgpu::BindGroupEntry {
          binding: 5,
          resource: window_buffer.as_entire_binding(),
        });
      }
      let bind_group = self.ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("live::bg"),
        layout: &pipeline.layout,
        entries: &entries,
      });

      let mut compute = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("live::pass"),
        ..Default::default()
      });
      compute.set_pipeline(&pipeline.pipeline);
      compute.set_bind_group(0, &bind_group, &[]);
      compute.dispatch_workgroups(width.div_ceil(WORKGROUP_SIZE), height.div_ceil(WORKGROUP_SIZE), 1);
    }
    self.ctx.submit(encoder.finish());

    self.output = if passes.is_empty() { Output::Source } else { Output::Work((passes.len() - 1) % 2) };
    Ok(())
  }

  /// The texture holding the last rendered frame, for presenting or copying on the GPU.
  pub fn output_texture(&self) -> Option<&wgpu::Texture> {
    match (self.output, &self.source, &self.work) {
      (Output::Source, Some((source, _, _)), _) => Some(&source.texture),
      (Output::Work(index), _, Some(work)) => Some(&work[index].texture),
      _ => None,
    }
  }

  /// Copies the completed output into its own texture, without reading it back to the CPU.
  ///
  /// Unlike [`output_texture`](Self::output_texture), this picture is not overwritten when the source or the working
  /// textures are reused for the next render. Use it when a consumer may keep displaying an earlier frame.
  pub fn snapshot_texture(&self) -> Result<wgpu::Texture> {
    let source = self.output_texture().ok_or_else(|| anyhow!("nothing has been rendered"))?;
    let (width, height) = (source.width(), source.height());
    let snapshot = Target::new(
      &self.ctx,
      "live::snapshot",
      width,
      height,
      wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
    );
    let mut encoder = self.ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("live::snapshot"),
    });
    encoder.copy_texture_to_texture(
      source.as_image_copy(),
      snapshot.texture.as_image_copy(),
      wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
    );
    self.ctx.submit(encoder.finish());
    Ok(snapshot.texture)
  }

  /// Waits for the last [`render`](Self::render) and copies the result back as `width * height` RGBA pixels.
  pub fn read_pixels(&mut self) -> Result<Vec<u8>> {
    loop {
      if self.request_frame()?.is_some() {
        break;
      }
      self.wait_frame()?;
    }
    let frame = self.wait_frame()?.ok_or_else(|| anyhow!("readback produced no frame"))?;
    Ok(frame.pixels)
  }

  /// Starts copying the last [`render`](Self::render) back to the CPU without waiting for it, and returns the frame's
  /// id. Returns `None` when every staging buffer is still in flight; try again after a later
  /// [`poll_frame`](Self::poll_frame).
  pub fn request_frame(&mut self) -> Result<Option<u64>> {
    let (width, height) = self.size().ok_or_else(|| anyhow!("set_source must be called before request_frame"))?;
    let texture = self.output_texture().cloned().ok_or_else(|| anyhow!("nothing has been rendered"))?;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = (4 * width as usize).div_ceil(align) * align;

    if self.readback.as_ref().is_none_or(|readback| readback.width != width || readback.height != height) {
      if self.readback.is_some() {
        self.ctx.wait()?;
      }
      let slots = (0..self.readback_slots)
        .map(|_| ReadbackSlot {
          buffer: self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("live::readback"),
            size: (padded * height as usize) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
          }),
          pending: None,
        })
        .collect();
      self.readback = Some(Readback {
        width,
        height,
        padded_row: padded,
        slots,
      });
    }
    let Some(readback) = self.readback.as_mut() else { return Ok(None) };
    let Some(slot) = readback.slots.iter_mut().find(|slot| slot.pending.is_none()) else {
      return Ok(None);
    };

    let mut encoder = self.ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("live::readback"),
    });
    encoder.copy_texture_to_buffer(
      wgpu::TexelCopyTextureInfo {
        texture: &texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      wgpu::TexelCopyBufferInfo {
        buffer: &slot.buffer,
        layout: wgpu::TexelCopyBufferLayout {
          offset: 0,
          bytes_per_row: Some(readback.padded_row as u32),
          rows_per_image: Some(height),
        },
      },
      wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
    );
    self.ctx.submit(encoder.finish());

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    slot.buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
      let _ = sender.send(result);
    });
    let id = self.next_frame;
    self.next_frame += 1;
    slot.pending = Some(PendingFrame {
      id,
      receiver,
      done: None,
      stale: false,
    });
    Ok(Some(id))
  }

  /// Returns the newest frame that has finished copying back, without waiting. Older finished frames are discarded
  /// and frames still in flight that are older than the returned one are dropped when they complete, so a caller
  /// that cannot keep up always sees the most recent state rather than a backlog. Returns `None` when nothing new
  /// has finished.
  pub fn poll_frame(&mut self) -> Result<Option<Frame>> {
    {
      let _guard = self.ctx.gate();
      self.ctx.device.poll(wgpu::PollType::Poll)?;
    }
    self.take_newest_frame()
  }

  /// Like [`poll_frame`](Self::poll_frame), but first waits for all requested frames to finish.
  pub fn wait_frame(&mut self) -> Result<Option<Frame>> {
    self.ctx.wait()?;
    self.take_newest_frame()
  }

  fn take_newest_frame(&mut self) -> Result<Option<Frame>> {
    let Some(readback) = self.readback.as_mut() else { return Ok(None) };

    for slot in readback.slots.iter_mut() {
      if let Some(pending) = slot.pending.as_mut()
        && pending.done.is_none()
        && let Ok(result) = pending.receiver.try_recv()
      {
        pending.done = Some(result);
      }
    }

    // Free failed and stale slots, and keep only the newest finished frame.
    let mut newest: Option<usize> = None;
    let mut failure = None;
    for index in 0..readback.slots.len() {
      let Some(pending) = readback.slots[index].pending.as_ref() else { continue };
      let (id, stale) = (pending.id, pending.stale);
      match &pending.done {
        None => {}
        Some(Err(error)) => {
          failure = Some(anyhow!("readback failed: {error}"));
          readback.slots[index].pending = None;
        }
        Some(Ok(())) if stale => {
          readback.slots[index].buffer.unmap();
          readback.slots[index].pending = None;
        }
        Some(Ok(())) => match newest {
          Some(best) if readback.slots[best].pending.as_ref().is_some_and(|best_pending| best_pending.id > id) => {
            readback.slots[index].buffer.unmap();
            readback.slots[index].pending = None;
          }
          Some(best) => {
            readback.slots[best].buffer.unmap();
            readback.slots[best].pending = None;
            newest = Some(index);
          }
          None => newest = Some(index),
        },
      }
    }
    if let Some(error) = failure {
      return Err(error);
    }
    let Some(index) = newest else { return Ok(None) };
    let id = readback.slots[index].pending.as_ref().map_or(0, |pending| pending.id);

    for slot in readback.slots.iter_mut() {
      if let Some(pending) = slot.pending.as_mut()
        && pending.done.is_none()
        && pending.id < id
      {
        pending.stale = true;
      }
    }

    let (width, height, padded) = (readback.width, readback.height, readback.padded_row);
    let slot = &mut readback.slots[index];
    let unpadded = 4 * width as usize;
    let data = slot.buffer.slice(..).get_mapped_range()?;
    let mut pixels = vec![0u8; unpadded * height as usize];
    for (row, out) in pixels.chunks_exact_mut(unpadded).enumerate() {
      out.copy_from_slice(&data[row * padded..row * padded + unpadded]);
    }
    drop(data);
    slot.buffer.unmap();
    slot.pending = None;
    Ok(Some(Frame {
      id,
      width,
      height,
      pixels,
    }))
  }

  /// Uploads `p_rgba`, runs the effects and reads the result back. This is the one-shot equivalent of
  /// [`set_source`](Self::set_source), [`render`](Self::render) and [`read_pixels`](Self::read_pixels), and still
  /// benefits from cached pipelines.
  pub fn process(&mut self, p_effects: &[Vec<GpuPass>], p_width: u32, p_height: u32, p_rgba: &[u8]) -> Result<Vec<u8>> {
    self.set_source(p_width, p_height, p_rgba)?;
    self.render(p_effects)?;
    self.read_pixels()
  }

  fn ensure_pipeline(&mut self, p_pass: &GpuPass) {
    let key = PipelineKey::of(p_pass);
    if self.pipeline(&key).is_ok() {
      return;
    }

    let mut entries = vec![
      wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
          sample_type: wgpu::TextureSampleType::Float { filterable: false },
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled: false,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::StorageTexture {
          access: wgpu::StorageTextureAccess::WriteOnly,
          format: if key.float_out { wgpu::TextureFormat::Rgba32Float } else { FORMAT },
          view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
      },
    ];
    if key.uniforms {
      entries.push(wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      });
    }
    if key.aux {
      entries.push(wgpu::BindGroupLayoutEntry {
        binding: 3,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
          sample_type: wgpu::TextureSampleType::Float { filterable: false },
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled: false,
        },
        count: None,
      });
    }

    if key.base {
      entries.push(wgpu::BindGroupLayoutEntry {
        binding: 4,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
          sample_type: wgpu::TextureSampleType::Float { filterable: false },
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled: false,
        },
        count: None,
      });
    }

    if key.window {
      entries.push(wgpu::BindGroupLayoutEntry {
        binding: 5,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      });
    }

    let layout = self.ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("live::bgl"),
      entries: &entries,
    });
    let pipeline_layout = self.ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("live::pl"),
      bind_group_layouts: &[Some(&layout)],
      immediate_size: 0,
    });
    let module = self.ctx.compile_wgsl(p_pass.shader, Some("live::shader"));
    let pipeline = self.ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
      label: Some("live::pipeline"),
      layout: Some(&pipeline_layout),
      module: &module,
      entry_point: Some("main"),
      cache: None,
      compilation_options: wgpu::PipelineCompilationOptions::default(),
    });
    if let Ok(mut pipelines) = self.ctx.pipelines.lock() {
      pipelines.insert(key, Pipeline { pipeline, layout });
    }
  }

  /// The compiled pipeline for `p_key`, shared by every renderer on this context.
  fn pipeline(&self, p_key: &PipelineKey) -> Result<Pipeline> {
    let pipelines = self.ctx.pipelines.lock().map_err(|_| anyhow!("GPU pipeline cache lock poisoned"))?;
    pipelines.get(p_key).cloned().ok_or_else(|| anyhow!("pipeline was not compiled"))
  }

  /// Writes the pass's uniform bytes into the buffer reserved for this position in the frame, growing it if needed.
  fn write_uniforms(&mut self, p_ordinal: usize, p_pass: &GpuPass) {
    if p_pass.uniforms.is_empty() {
      return;
    }
    if self.uniforms.len() <= p_ordinal {
      self.uniforms.resize_with(p_ordinal + 1, || None);
    }
    let size = p_pass.uniforms.len().div_ceil(UNIFORM_ALIGN) * UNIFORM_ALIGN;
    if self.uniforms[p_ordinal].as_ref().is_none_or(|slot| slot.size < size) {
      let buffer = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("live::uniform"),
        size: size as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
      });
      self.uniforms[p_ordinal] = Some(UniformSlot { buffer, size });
    }

    if let Some(slot) = &self.uniforms[p_ordinal] {
      let mut bytes = p_pass.uniforms.clone();
      bytes.resize(slot.size, 0);
      self.ctx.queue.write_buffer(&slot.buffer, 0, &bytes);
    }
  }

  /// Keeps the pass's auxiliary texture in the slot for this position in the frame, only re-uploading when its
  /// contents changed.
  fn write_aux(&mut self, p_ordinal: usize, p_pass: &GpuPass) {
    if self.aux.len() <= p_ordinal {
      self.aux.resize_with(p_ordinal + 1, || None);
    }
    let Some(aux) = &p_pass.aux else {
      return;
    };

    let reusable =
      self.aux[p_ordinal].as_ref().is_some_and(|slot| slot.width == aux.width && slot.height == aux.height);
    if !reusable {
      let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
      self.aux[p_ordinal] = Some(AuxSlot {
        target: Target::new(&self.ctx, "live::aux", aux.width, aux.height, usage),
        width: aux.width,
        height: aux.height,
        rgba: std::sync::Arc::from(Vec::new()),
      });
    }
    if let Some(slot) = &mut self.aux[p_ordinal]
      && slot.rgba != aux.rgba
    {
      slot.target.write(&self.ctx, aux.width, aux.height, &aux.rgba);
      slot.rgba = aux.rgba.clone();
    }
  }
}

impl GpuSession for LiveRenderer {
  fn set_source(&mut self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> std::result::Result<(), String> {
    LiveRenderer::set_source(self, p_width, p_height, p_rgba).map_err(|e| e.to_string())
  }

  fn render(&mut self, p_effects: &[Vec<GpuPass>]) -> std::result::Result<(), String> {
    LiveRenderer::render(self, p_effects).map_err(|e| e.to_string())
  }

  fn read_pixels(&mut self) -> std::result::Result<Vec<u8>, String> {
    LiveRenderer::read_pixels(self).map_err(|e| e.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Writes the red channel times 1000 (far past what 8 bits hold) as `rgba32float`.
  const SCALE_UP: &str = "
    @group(0) @binding(0) var input_tex: texture_2d<f32>;
    @group(0) @binding(1) var output_tex: texture_storage_2d<rgba32float, write>;
    @compute @workgroup_size(8, 8)
    fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
      let dims = vec2<i32>(textureDimensions(input_tex));
      let pos = vec2<i32>(gid.xy);
      if (pos.x >= dims.x || pos.y >= dims.y) { return; }
      textureStore(output_tex, pos, vec4<f32>(textureLoad(input_tex, pos, 0).r * 1000.0 + 0.25, 0.0, 0.0, 1.0));
    }";

  /// Reads those numbers back and writes them divided by 1000 as an ordinary 8-bit pixel.
  const SCALE_DOWN: &str = "
    @group(0) @binding(0) var input_tex: texture_2d<f32>;
    @group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
    @compute @workgroup_size(8, 8)
    fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
      let dims = vec2<i32>(textureDimensions(input_tex));
      let pos = vec2<i32>(gid.xy);
      if (pos.x >= dims.x || pos.y >= dims.y) { return; }
      let value = (textureLoad(input_tex, pos, 0).r - 0.25) / 1000.0;
      textureStore(output_tex, pos, vec4<f32>(value, value, value, 1.0));
    }";

  fn ramp(p_width: u32, p_height: u32) -> Vec<u8> {
    (0..p_width * p_height).flat_map(|i| [(i * 7 % 256) as u8, 0, 0, 255]).collect()
  }

  #[test]
  fn a_pass_that_writes_rgba32float_hands_numbers_8_bits_cannot_hold_to_the_next_pass() -> Result<()> {
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let (width, height) = (19u32, 11u32);
    let pixels = ramp(width, height);
    let chain = vec![vec![GpuPass::new(SCALE_UP, Vec::new()), GpuPass::new(SCALE_DOWN, Vec::new())]];
    let out = renderer.process(&chain, width, height, &pixels)?;
    for (index, (got, put)) in out.chunks_exact(4).zip(pixels.chunks_exact(4)).enumerate() {
      assert!(got[0].abs_diff(put[0]) <= 1, "pixel {index}: {} came back as {}", put[0], got[0]);
    }
    Ok(())
  }

  #[test]
  fn a_chain_cannot_end_on_a_pass_that_writes_rgba32float() -> Result<()> {
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let result = renderer.process(&[vec![GpuPass::new(SCALE_UP, Vec::new())]], 8, 8, &ramp(8, 8));
    assert!(result.is_err());
    Ok(())
  }

  #[test]
  fn float_passes_do_not_change_a_chain_that_has_none() -> Result<()> {
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let pixels = ramp(8, 8);
    // The same shader as a plain 8-bit pass: the chain is exactly what it was before float passes existed.
    let copy = "
      @group(0) @binding(0) var input_tex: texture_2d<f32>;
      @group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
      @compute @workgroup_size(8, 8)
      fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
        let pos = vec2<i32>(gid.xy);
        textureStore(output_tex, pos, textureLoad(input_tex, pos, 0));
      }";
    assert_eq!(renderer.process(&[vec![GpuPass::new(copy, Vec::new())]], 8, 8, &pixels)?, pixels);
    assert!(renderer.float_work.is_none());
    Ok(())
  }
}
