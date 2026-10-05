//! Draws GPU textures on a Vessel surface without copying them to the CPU.
//!
//! [`TextureFrame`] is a finished picture in GPU memory, in the form the engine passes around (`vessel_engine::GpuFrame`).
//! [`AndroidTarget`] is the `vessel_engine::GpuTarget` for an Android window: it presents a texture on the window's swap chain.
//! A session makes it by itself from the first texture it shows.
//! On iOS, `IosTarget` presents directly to the host's retained Metal layer.
//! Once a window has one, everything shown on it goes through it, pixels included, because a swap chain and a CPU blit
//! cannot share a window.

use std::any::Any;
use std::sync::Arc;

use vessel_engine::surface::{self, AndroidWindow};
use vessel_engine::{Frame, GpuFrame, GpuTarget};

use crate::{GpuContext, Presenter};
#[cfg(target_os = "ios")]
use vessel_engine::surface::IosSurface;

/// A finished picture in GPU memory, made by `context`'s GPU. Cloning is cheap: it is another handle to the same texture.
///
/// It knows its GPU, so a session can draw it on a window ([`target_for`](GpuFrame::target_for)) or read it back to the CPU
/// ([`read_back`](GpuFrame::read_back)) without the app saying anything about either.
#[derive(Clone)]
pub struct TextureFrame {
  context: GpuContext,
  texture: wgpu::Texture,
}

impl TextureFrame {
  /// Wraps `p_texture`, which holds the picture and was made by `p_context`'s GPU.
  pub fn new(p_context: &GpuContext, p_texture: wgpu::Texture) -> Self {
    Self {
      context: p_context.clone(),
      texture: p_texture,
    }
  }

  /// The texture that holds the picture.
  pub fn texture(&self) -> &wgpu::Texture {
    &self.texture
  }
}

impl GpuFrame for TextureFrame {
  fn size(&self) -> (u32, u32) {
    (self.texture.width(), self.texture.height())
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn read_back(&self) -> Option<Frame> {
    let (width, height) = (self.texture.width(), self.texture.height());
    let padded = (4 * width as usize).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize)
      * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let device = &self.context.device;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("texture-frame::read_back"),
      size: (padded * height as usize) as u64,
      usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("texture-frame::read_back"),
    });
    encoder.copy_texture_to_buffer(
      self.texture.as_image_copy(),
      wgpu::TexelCopyBufferInfo {
        buffer: &buffer,
        layout: wgpu::TexelCopyBufferLayout {
          offset: 0,
          bytes_per_row: Some(padded as u32),
          rows_per_image: Some(height),
        },
      },
      wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
    );
    self.context.submit(encoder.finish());
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
      let _ = sender.send(result);
    });
    {
      let _guard = self.context.submission_gate.lock().unwrap();
      device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
    }
    receiver.recv().ok()?.ok()?;
    let mapped = buffer.slice(..).get_mapped_range().ok()?;
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for row in mapped.chunks_exact(padded) {
      pixels.extend_from_slice(&row[..width as usize * 4]);
    }
    drop(mapped);
    buffer.unmap();
    Some(Frame { width, height, pixels: Arc::new(pixels) })
  }

  fn target_for(&self, p_surface_id: i32) -> Option<Box<dyn GpuTarget>> {
    #[cfg(target_os = "ios")]
    return Some(Box::new(IosTarget::new(&self.context, p_surface_id)?));

    #[cfg(not(target_os = "ios"))]
    Some(Box::new(AndroidTarget::new(&self.context, p_surface_id)?))
  }
}

/// Draws GPU frames directly on an iOS host's Metal layer and uploads subsequent CPU frames to the same presenter.
#[cfg(target_os = "ios")]
pub struct IosTarget {
  // Dropped before the surface that retains the layer.
  presenter: Presenter,
  surface: Arc<IosSurface>,
  size: (u32, u32),
}

#[cfg(target_os = "ios")]
impl IosTarget {
  /// Creates a target using the frame's GPU, or returns `None` if that GPU cannot present to the layer.
  pub fn new(p_context: &GpuContext, p_surface_id: i32) -> Option<Self> {
    let surface = surface::get_as::<IosSurface>(p_surface_id)?;
    let mut created = None;
    surface.with_layer(|layer, size| {
      // SAFETY: the target retains the surface until after the presenter is dropped.
      match unsafe { Presenter::from_metal_layer(p_context, layer, size.0, size.1) } {
        Ok(presenter) => {
          created = Some((presenter, size));
          true
        }
        Err(error) => {
          eprintln!("vessel: could not create iOS GPU target: {error}");
          false
        }
      }
    });
    let (presenter, size) = created?;
    Some(Self {
      presenter,
      surface,
      size,
    })
  }

  fn draw(&mut self, p_draw: impl FnOnce(&mut Presenter) -> bool) -> bool {
    let surface = Arc::clone(&self.surface);
    surface.with_layer(|_, size| {
      if self.size != size {
        self.size = size;
        self.presenter.resize(size.0, size.1);
      }
      p_draw(&mut self.presenter)
    })
  }
}

#[cfg(target_os = "ios")]
impl GpuTarget for IosTarget {
  fn present(&mut self, p_frame: &dyn GpuFrame) -> bool {
    let Some(frame) = p_frame.as_any().downcast_ref::<TextureFrame>() else { return false };
    self.draw(|presenter| match presenter.try_present(frame.texture()) {
      Ok(drawn) => drawn,
      Err(error) => {
        eprintln!("vessel: iOS GPU presentation failed: {error}");
        false
      }
    })
  }

  fn present_pixels(&mut self, p_frame: &Frame) -> bool {
    self.draw(|presenter| match presenter.try_present_pixels(p_frame.width, p_frame.height, &p_frame.pixels) {
      Ok(drawn) => drawn,
      Err(error) => {
        eprintln!("vessel: iOS pixel presentation failed: {error}");
        false
      }
    })
  }
}

/// Draws on the Android window registered under an id, from GPU memory.
pub struct AndroidTarget {
  // Declared first so it is dropped before the window it draws on.
  presenter: Presenter,
  surface_id: i32,
  window: Arc<AndroidWindow>,
  size: (u32, u32),
}

impl AndroidTarget {
  /// A target for the window registered under `p_surface_id`, or `None` if its view has not reported a window yet or the
  /// GPU cannot draw on it.
  pub fn new(p_context: &GpuContext, p_surface_id: i32) -> Option<Self> {
    let window = surface::get_as::<AndroidWindow>(p_surface_id)?;
    let (width, height) = {
      let state = window.state.lock().ok()?;
      if !state.alive {
        return None;
      }
      (state.width, state.height)
    };
    // SAFETY: the window is held by this target for as long as the presenter, and is destroyed after it.
    let presenter = unsafe { Presenter::from_android_window(p_context, window.ptr(), width, height) }.ok()?;
    Some(Self {
      presenter,
      surface_id: p_surface_id,
      window,
      size: (width, height),
    })
  }

  /// Runs `p_draw` on the presenter with the window locked, so the platform cannot destroy the surface mid-frame. Returns
  /// whether the frame was drawn; `false` means the window is gone or was replaced.
  fn draw(&mut self, p_draw: impl FnOnce(&mut Presenter) -> bool) -> bool {
    let window = Arc::clone(&self.window);
    let state = window.state.lock().unwrap();
    let current = surface::get_as::<AndroidWindow>(self.surface_id).is_some_and(|other| Arc::ptr_eq(&other, &window));
    if !state.alive || !current {
      return false;
    }
    if self.size != (state.width, state.height) {
      self.size = (state.width, state.height);
      self.presenter.resize(state.width, state.height);
    }
    p_draw(&mut self.presenter)
  }
}

impl GpuTarget for AndroidTarget {
  fn present(&mut self, p_frame: &dyn GpuFrame) -> bool {
    let Some(frame) = p_frame.as_any().downcast_ref::<TextureFrame>() else { return false };
    self.draw(|presenter| presenter.present(frame.texture()).is_ok())
  }

  fn present_pixels(&mut self, p_frame: &Frame) -> bool {
    self.draw(|presenter| presenter.present_pixels(p_frame.width, p_frame.height, &p_frame.pixels).is_ok())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::LiveRenderer;

  #[test]
  fn cloned_contexts_share_a_gate_and_concurrent_readbacks_preserve_pixels() -> anyhow::Result<()> {
    let context = GpuContext::new_default_blocking()?;
    assert!(Arc::ptr_eq(&context.submission_gate, &context.clone().submission_gate));
    let workers: Vec<_> = (0..4u8)
      .map(|index| {
        let context = context.clone();
        std::thread::spawn(move || -> anyhow::Result<()> {
          let color = [50 * index, 20, 100, 255];
          let pixels = color.repeat(16 * 16);
          let mut renderer = LiveRenderer::new(context.clone());
          renderer.set_source(16, 16, &pixels)?;
          for _ in 0..10 {
            renderer.render(&[])?;
            let frame = TextureFrame::new(&context, renderer.snapshot_texture()?);
            let readback = frame.read_back().ok_or_else(|| anyhow::anyhow!("concurrent texture readback failed"))?;
            assert_eq!(readback.pixels, pixels);
          }
          Ok(())
        })
      })
      .collect();
    for worker in workers {
      worker.join().expect("GPU worker panicked")?;
    }
    Ok(())
  }
}
