//! The GPU side of a live image: a source uploaded once, re-rendered through effect chains without blocking.

use abra_core::image::gpu::{ChainRenderer, GpuEffect, LiveEffect};
use gpu::{Frame, LiveRenderer, Presenter};
use std::sync::Arc;

/// A source image uploaded once, plus the state needed to render effect chains over it interactively.
///
/// Effects with a shader run on the GPU; effects without one run on the CPU in between (see [`ChainRenderer`]). Two
/// ways to get frames out:
/// - [`submit`](Self::submit) and [`poll`](Self::poll) never block, and a caller that submits faster than frames
///   come back only ever sees the latest state.
/// - [`present`](Self::present) draws straight onto a window surface without reading anything back.
pub(crate) struct GpuBackend {
  chain: ChainRenderer<LiveRenderer>,
  ready: Option<Frame>,
  deferred: Option<Vec<Arc<dyn LiveEffect>>>,
  next_cpu_frame: u64,
}

impl GpuBackend {
  /// Uploads `p_rgba` (`p_width * p_height` RGBA pixels) as the image effects are applied to.
  pub(crate) fn new(p_width: u32, p_height: u32, p_rgba: &[u8]) -> Result<GpuBackend, String> {
    let context = gpu::context().ok_or("no GPU adapter available")?;
    let chain = ChainRenderer::new(LiveRenderer::new(context.clone()), p_width, p_height, p_rgba)?;
    Ok(GpuBackend {
      chain,
      ready: None,
      deferred: None,
      next_cpu_frame: u64::MAX / 2,
    })
  }

  /// Runs the chain on the CPU and GPU together and holds the pixels for the next [`poll`](Self::poll).
  fn submit_with_cpu_effects(&mut self, p_effects: &[Arc<dyn LiveEffect>]) -> Result<(), String> {
    let refs: Vec<&dyn LiveEffect> = p_effects.iter().map(|effect| effect.as_ref()).collect();
    let pixels = self.chain.render_blocking(&refs)?;
    let (width, height) = self.chain.size();
    self.ready = Some(Frame {
      id: self.next_cpu_frame,
      width,
      height,
      pixels,
    });
    self.next_cpu_frame += 1;
    Ok(())
  }

  /// Starts rendering the chain without waiting. Collect the result with [`poll`](Self::poll).
  ///
  /// A chain made only of GPU effects is fully asynchronous. A chain with a CPU-only effect is computed here and is
  /// ready at the next `poll`. If the GPU has too many frames in flight the chain is remembered and submitted by a
  /// later `poll`, so the final state is always delivered.
  pub(crate) fn submit(&mut self, p_effects: Vec<Arc<dyn LiveEffect>>) -> Result<(), String> {
    self.deferred = None;
    if p_effects.iter().any(|effect| !effect.has_gpu()) {
      return self.submit_with_cpu_effects(&p_effects);
    }
    self.chain.restore_source()?;
    let run: Vec<&dyn GpuEffect> = p_effects.iter().map(|effect| effect.as_ref() as &dyn GpuEffect).collect();
    gpu_render(&mut self.chain.session, &run)?;
    if self.chain.session.request_frame().map_err(|e| e.to_string())?.is_none() {
      self.deferred = Some(p_effects);
    }
    Ok(())
  }

  /// Renders the chain and draws it on `p_presenter`'s surface without reading anything back. GPU-only chains never
  /// leave the GPU; a chain with a CPU-only effect is computed on the CPU and uploaded.
  pub(crate) fn present(&mut self, p_effects: &[Arc<dyn LiveEffect>], p_presenter: &mut Presenter) -> Result<(), String> {
    self.deferred = None;
    if p_effects.iter().any(|effect| !effect.has_gpu()) {
      let refs: Vec<&dyn LiveEffect> = p_effects.iter().map(|effect| effect.as_ref()).collect();
      let pixels = self.chain.render_blocking(&refs)?;
      let (width, height) = self.chain.size();
      return p_presenter.present_pixels(width, height, &pixels).map_err(|e| e.to_string());
    }
    self.chain.restore_source()?;
    let run: Vec<&dyn GpuEffect> = p_effects.iter().map(|effect| effect.as_ref() as &dyn GpuEffect).collect();
    gpu_render(&mut self.chain.session, &run)?;
    let texture = self.chain.session.output_texture().ok_or("nothing has been rendered")?;
    p_presenter.present(texture).map_err(|e| e.to_string())
  }

  /// The newest finished frame, if any, without waiting.
  pub(crate) fn poll(&mut self) -> Result<Option<Frame>, String> {
    let frame = match self.ready.take() {
      Some(frame) => Some(frame),
      None => self.chain.session.poll_frame().map_err(|e| e.to_string())?,
    };
    if let Some(effects) = self.deferred.take() {
      self.submit(effects)?;
    }
    Ok(frame)
  }

  /// Runs the chain and returns the final pixels, waiting for the GPU.
  #[cfg(test)]
  pub(crate) fn render_blocking(&mut self, p_effects: &[&dyn LiveEffect]) -> Result<Vec<u8>, String> {
    self.chain.render_blocking(p_effects)
  }
}

fn gpu_render(p_renderer: &mut LiveRenderer, p_effects: &[&dyn GpuEffect]) -> Result<(), String> {
  p_renderer.render(p_effects).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Image;
  use abra_core::image::gpu::{GpuOp, GpuPass};

  const BRIGHTNESS: &str = include_str!("../../adjustments/src/levels/brightness.wgsl");

  struct Bright(f32);

  impl GpuEffect for Bright {
    fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
      GpuOp::new(BRIGHTNESS, self.0.to_le_bytes()).passes(p_width, p_height)
    }
  }

  impl LiveEffect for Bright {
    fn apply_cpu(&self, p_image: &mut Image) {
      let pixels: Vec<u8> = p_image
        .to_rgba_vec()
        .chunks_exact(4)
        .flat_map(|p| [(p[0] as f32 * self.0) as u8, (p[1] as f32 * self.0) as u8, (p[2] as f32 * self.0) as u8, p[3]])
        .collect();
      p_image.set_rgba(pixels);
    }
  }

  /// No shader: only the CPU implementation exists.
  struct Invert;

  impl GpuEffect for Invert {
    fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
      Vec::new()
    }
  }

  impl LiveEffect for Invert {
    fn has_gpu(&self) -> bool {
      false
    }

    fn apply_cpu(&self, p_image: &mut Image) {
      let pixels: Vec<u8> =
        p_image.to_rgba_vec().chunks_exact(4).flat_map(|p| [255 - p[0], 255 - p[1], 255 - p[2], p[3]]).collect();
      p_image.set_rgba(pixels);
    }
  }

  fn backend() -> Option<GpuBackend> {
    let pixels: Vec<u8> = (0..13 * 7).flat_map(|i| [(i % 200) as u8, 100, 50, 255]).collect();
    GpuBackend::new(13, 7, &pixels).ok()
  }

  fn close(p_a: &[u8], p_b: &[u8]) {
    assert_eq!(p_a.len(), p_b.len());
    for (i, (a, b)) in p_a.iter().zip(p_b).enumerate() {
      assert!(a.abs_diff(*b) <= 2, "byte {i}: {a} vs {b}");
    }
  }

  #[test]
  fn a_cpu_only_effect_runs_between_gpu_effects() {
    let Some(mut backend) = backend() else { return };
    let chain: [&dyn LiveEffect; 3] = [&Bright(0.5), &Invert, &Bright(0.5)];
    let out = backend.render_blocking(&chain).unwrap();

    let mut expected = Image::new_from_pixels(13, 7, backend.render_blocking(&[]).unwrap(), abra_core::Channels::RGBA);
    for effect in chain {
      effect.apply_cpu(&mut expected);
    }
    close(&out, expected.rgba());
  }

  #[test]
  fn rendering_again_starts_from_the_original_source() {
    let Some(mut backend) = backend() else { return };
    let original = backend.render_blocking(&[]).unwrap();
    backend.render_blocking(&[&Invert]).unwrap();
    assert_eq!(backend.render_blocking(&[]).unwrap(), original);
  }

  #[test]
  fn submit_and_poll_deliver_the_last_chain_even_when_the_pool_was_full() {
    let Some(mut backend) = backend() else { return };
    let original = backend.render_blocking(&[]).unwrap();
    for factor in [0.1f32, 0.2, 0.3, 0.4, 0.5] {
      backend.submit(vec![Arc::new(Bright(factor))]).unwrap();
    }
    let mut last = None;
    for _ in 0..200 {
      if let Some(frame) = backend.poll().unwrap() {
        last = Some(frame);
      }
      std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let frame = last.expect("a frame");
    assert!(frame.pixels[0].abs_diff((original[0] as f32 * 0.5) as u8) <= 2);
  }

  #[test]
  fn a_chain_with_a_cpu_effect_is_ready_at_the_next_poll() {
    let Some(mut backend) = backend() else { return };
    let original = backend.render_blocking(&[]).unwrap();
    backend.submit(vec![Arc::new(Invert)]).unwrap();
    let frame = backend.poll().unwrap().expect("a frame");
    assert_eq!(frame.pixels[0], 255 - original[0]);
  }

  #[test]
  fn a_masked_shader_effect_matches_its_cpu_result() {
    let Some(mut backend) = backend() else { return };
    let weights: Vec<f32> = (0..13 * 7).map(|i| if i % 13 < 4 { 1.0 } else if i % 13 < 8 { 0.5 } else { 0.0 }).collect();
    let masked = crate::masked::MaskedEffect::new(Arc::new(Bright(0.5)), 13, 7, &weights);
    assert!(masked.has_gpu());

    let out = backend.render_blocking(&[&masked]).unwrap();
    let mut expected = Image::new_from_pixels(13, 7, backend.render_blocking(&[]).unwrap(), abra_core::Channels::RGBA);
    masked.apply_cpu(&mut expected);
    close(&out, expected.rgba());
  }

  #[test]
  fn masking_an_effect_keeps_it_working_after_another_effect() {
    let Some(mut backend) = backend() else { return };
    let weights = vec![0.0f32; 13 * 7];
    let none = crate::masked::MaskedEffect::new(Arc::new(Bright(0.5)), 13, 7, &weights);
    let original = backend.render_blocking(&[]).unwrap();
    // Weight 0 everywhere: the masked effect must hand back exactly what it was given, which is the first effect's result.
    let out = backend.render_blocking(&[&Bright(0.5), &none]).unwrap();
    let plain = backend.render_blocking(&[&Bright(0.5)]).unwrap();
    close(&out, &plain);
    assert_ne!(out, original);
  }
}
