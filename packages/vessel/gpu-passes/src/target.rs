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

use crate::{GpuContext, PresentOptions, Presenter};
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
  /// The part of the texture to show, `[x, y, width, height]` in 0..1 of it.
  view: [f32; 4],
  /// How a window that shows this picture draws it. Only the first picture a window shows decides, since that is when its
  /// target is made; change it later with the target's presenter.
  present: PresentOptions,
}

impl TextureFrame {
  /// Wraps `p_texture`, which holds the picture and was made by `p_context`'s GPU.
  pub fn new(p_context: &GpuContext, p_texture: wgpu::Texture) -> Self {
    Self {
      context: p_context.clone(),
      texture: p_texture,
      view: [0.0, 0.0, 1.0, 1.0],
      present: PresentOptions::default(),
    }
  }

  /// The same texture, drawn on a window as `p_options` say (when it is vsynced, what color shows past the picture).
  pub fn with_present_options(&self, p_options: PresentOptions) -> Self {
    Self { present: p_options, ..self.clone() }
  }

  /// The same texture, showing only `p_view` (`[x, y, width, height]` in 0..1 of it). Nothing is copied.
  pub fn with_view(&self, p_view: [f32; 4]) -> Self {
    Self { view: p_view, ..self.clone() }
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

  fn view(&self) -> [f32; 4] {
    self.view
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
    if let Err(error) = self.context.wait() {
      surface::log(&format!("vessel: a {width}x{height} frame could not be read back, waiting for the GPU failed: {error}"));
      return None;
    }
    match receiver.recv() {
      Ok(Ok(())) => {}
      Ok(Err(error)) => {
        surface::log(&format!("vessel: a {width}x{height} frame could not be read back, the buffer could not be mapped: {error}"));
        return None;
      }
      Err(_) => {
        surface::log(&format!("vessel: a {width}x{height} frame could not be read back, the GPU never answered"));
        return None;
      }
    }
    let mapped = match buffer.slice(..).get_mapped_range() {
      Ok(mapped) => mapped,
      Err(error) => {
        surface::log(&format!("vessel: a {width}x{height} frame could not be read back, the mapped range failed: {error}"));
        return None;
      }
    };
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for row in mapped.chunks_exact(padded) {
      pixels.extend_from_slice(&row[..width as usize * 4]);
    }
    drop(mapped);
    buffer.unmap();
    let [view_x, view_y, view_width, view_height] = self.view;
    if self.view == [0.0, 0.0, 1.0, 1.0] {
      return Some(Frame { width, height, pixels: Arc::new(pixels) });
    }
    // Only the part that is shown, the same rectangle the screen target would show. Where the view reaches past the
    // texture the screen target draws black, so the cut is black there too.
    let left = (view_x * width as f32).floor() as i64;
    let top = (view_y * height as f32).floor() as i64;
    let cut_width = ((view_width * width as f32).round() as i64).max(1);
    let cut_height = ((view_height * height as f32).round() as i64).max(1);
    let mut cut = [0u8, 0, 0, 255].repeat(cut_width as usize * cut_height as usize);
    let (from, to) = (left.max(0), (left + cut_width).min(width as i64));
    if from < to {
      for row in top.max(0)..(top + cut_height).min(height as i64) {
        let source = (row as usize * width as usize + from as usize) * 4;
        let target = (((row - top) * cut_width + (from - left)) as usize) * 4;
        let length = (to - from) as usize * 4;
        cut[target..target + length].copy_from_slice(&pixels[source..source + length]);
      }
    }
    Some(Frame { width: cut_width as u32, height: cut_height as u32, pixels: Arc::new(cut) })
  }

  fn target_for(&self, p_surface_id: i32) -> Option<Box<dyn GpuTarget>> {
    #[cfg(target_os = "ios")]
    return Some(Box::new(IosTarget::with_options(&self.context, p_surface_id, &self.present)?));

    #[cfg(not(target_os = "ios"))]
    Some(Box::new(AndroidTarget::with_options(&self.context, p_surface_id, &self.present)?))
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
    Self::with_options(p_context, p_surface_id, &PresentOptions::default())
  }

  /// Like [`new`](Self::new), drawing as `p_options` say.
  pub fn with_options(p_context: &GpuContext, p_surface_id: i32, p_options: &PresentOptions) -> Option<Self> {
    let surface = surface::get_as::<IosSurface>(p_surface_id)?;
    let mut created = None;
    surface.with_layer(|layer, size| {
      // SAFETY: the target retains the surface until after the presenter is dropped.
      match unsafe { Presenter::from_metal_layer(p_context, layer, size.0, size.1) } {
        Ok(mut presenter) => {
          if let Err(error) = presenter.set_options(p_options) {
            eprintln!("vessel: {error}");
          }
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

  fn present_view(&mut self, p_frame: &dyn GpuFrame, p_view: [f32; 4]) -> bool {
    let Some(frame) = p_frame.as_any().downcast_ref::<TextureFrame>() else { return false };
    self.draw(|presenter| match presenter.try_present_view(frame.texture(), p_view) {
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
    Self::with_options(p_context, p_surface_id, &PresentOptions::default())
  }

  /// Like [`new`](Self::new), drawing as `p_options` say. A present mode the window cannot do is reported and the default
  /// one is used.
  pub fn with_options(p_context: &GpuContext, p_surface_id: i32, p_options: &PresentOptions) -> Option<Self> {
    let window = surface::get_as::<AndroidWindow>(p_surface_id)?;
    let (width, height) = {
      let state = window.lock_state();
      if !state.alive {
        return None;
      }
      (state.width, state.height)
    };
    // SAFETY: the window is held by this target for as long as the presenter, and is destroyed after it.
    // The driver refuses a window that an earlier surface is still attached to (`ERROR_NATIVE_WINDOW_IN_USE_KHR`), which wgpu
    // reports as a panic. A view replaced quickly leaves the old surface for a moment, so ask again a few times.
    let mut presenter = None;
    for attempt in 0..5 {
      if attempt > 0 {
        std::thread::sleep(std::time::Duration::from_millis(40));
      }
      let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        Presenter::from_android_window(p_context, window.ptr(), width, height)
      }));
      if let Ok(Ok(found)) = made {
        presenter = Some(found);
        break;
      }
    }
    let mut presenter = presenter?;
    if let Err(error) = presenter.set_options(p_options) {
      surface::log(&format!("vessel: {error}"));
    }
    Some(Self {
      presenter,
      surface_id: p_surface_id,
      window,
      size: (width, height),
    })
  }

  /// Runs `p_draw` on the presenter with the window locked, so the platform cannot destroy the surface mid-frame. Returns
  /// whether the frame was drawn; `false` means the window is gone or was replaced, or the GPU panicked on it (a failed
  /// surface reconfigure, say). A panic must not unwind through the window's lock, which the platform needs to report a
  /// new size, so it is stopped here and the frame is reported as not drawn: the session then draws from the CPU instead.
  fn draw(&mut self, p_draw: impl FnOnce(&mut Presenter) -> bool) -> bool {
    let window = Arc::clone(&self.window);
    let state = window.lock_state();
    let current = surface::get_as::<AndroidWindow>(self.surface_id).is_some_and(|other| Arc::ptr_eq(&other, &window));
    if !state.alive || !current {
      return false;
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      if self.size != (state.width, state.height) {
        self.presenter.resize(state.width, state.height);
        self.size = (state.width, state.height);
      }
      p_draw(&mut self.presenter)
    }))
    .unwrap_or(false)
  }
}

impl GpuTarget for AndroidTarget {
  fn present(&mut self, p_frame: &dyn GpuFrame) -> bool {
    let Some(frame) = p_frame.as_any().downcast_ref::<TextureFrame>() else { return false };
    self.draw(|presenter| presenter.present(frame.texture()).is_ok())
  }

  fn present_view(&mut self, p_frame: &dyn GpuFrame, p_view: [f32; 4]) -> bool {
    let Some(frame) = p_frame.as_any().downcast_ref::<TextureFrame>() else { return false };
    self.draw(|presenter| presenter.try_present_view(frame.texture(), p_view).is_ok())
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
  fn a_panic_while_the_gate_is_held_does_not_stop_later_gpu_calls() -> anyhow::Result<()> {
    let context = GpuContext::new_default_blocking()?;
    let held = context.submission_gate.clone();
    let panicked = std::thread::spawn(move || {
      let _guard = held.lock().unwrap();
      panic!("a validation error while the gate is held");
    })
    .join();
    assert!(panicked.is_err());
    assert!(context.submission_gate.is_poisoned(), "the panic poisoned the gate");
    // A later submission still gets the gate, as it did before the panic.
    drop(context.gate());
    let mut renderer = LiveRenderer::new(context.clone());
    renderer.set_source(2, 2, &[10u8; 16])?;
    renderer.render(&[])?;
    assert_eq!(renderer.read_pixels()?, vec![10u8; 16]);
    Ok(())
  }

  #[test]
  fn waiting_for_the_gpu_finishes_the_work_and_leaves_the_gate_free() -> anyhow::Result<()> {
    let context = GpuContext::new_default_blocking()?;
    let mut renderer = LiveRenderer::new(context.clone());
    renderer.set_source(2, 2, &[10u8; 16])?;
    renderer.render(&[])?;
    context.wait()?;
    // The gate is taken for each ask and let go of, so another thread's submission is never kept out for the whole wait.
    assert!(context.submission_gate.try_lock().is_ok());
    Ok(())
  }

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
            assert_eq!(*readback.pixels, pixels);
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
