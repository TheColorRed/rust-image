//! The GPU side of a live image: a source uploaded once, re-rendered through effect chains.

use abra::abra_core::image::gpu::{ChainRenderer, GpuEffect, GpuPass, LiveEffect};
use gpu::LiveRenderer;
use std::sync::Arc;

/// A source image uploaded once, plus the state needed to render effect chains over it interactively.
///
/// Effects with a shader run on the GPU; effects without one run on the CPU in between (see [`ChainRenderer`]). A chain
/// is rendered either to a texture that a surface can draw as it is ([`render_texture`](Self::render_texture)), or to
/// pixels the caller waits for ([`render_blocking`](Self::render_blocking)).
pub(crate) struct GpuBackend {
  chain: ChainRenderer<LiveRenderer>,
  context: &'static gpu::GpuContext,
}

impl GpuBackend {
  /// Uploads `p_rgba` (`p_width * p_height` RGBA pixels) as the image effects are applied to.
  pub(crate) fn new(p_width: u32, p_height: u32, p_rgba: &[u8]) -> Result<GpuBackend, String> {
    let context = gpu::context().ok_or("no GPU adapter available")?;
    let chain = ChainRenderer::new(LiveRenderer::new(context.clone()), p_width, p_height, p_rgba)?;
    Ok(GpuBackend { chain, context })
  }

  /// Renders the chain on the GPU and returns the finished picture without reading anything back, for a surface that can
  /// draw a texture itself. Returns `None`, and renders nothing, when the chain has an effect without a shader: that
  /// chain needs the CPU, so it goes through [`render_blocking`](Self::render_blocking).
  pub(crate) fn render_texture(
    &mut self, p_effects: &[Arc<dyn LiveEffect>],
  ) -> Result<Option<gpu_passes::target::TextureFrame>, String> {
    if p_effects.iter().any(|effect| !effect.has_gpu()) {
      return Ok(None);
    }
    self.chain.restore_source()?;
    let run: Vec<&dyn GpuEffect> = p_effects.iter().map(|effect| effect.as_ref() as &dyn GpuEffect).collect();
    gpu_render(&mut self.chain.session, &run)?;
    let texture = self.chain.session.output_texture().ok_or("nothing has been rendered")?;
    Ok(Some(gpu_passes::target::TextureFrame::new(self.context, texture.clone())))
  }

  /// Runs the chain and returns the final pixels, waiting for the GPU.
  pub(crate) fn render_blocking(&mut self, p_effects: &[&dyn LiveEffect]) -> Result<Vec<u8>, String> {
    self.chain.render_blocking(p_effects)
  }
}

/// Describes the effects as passes at the renderer's image size and runs them.
fn gpu_render(p_renderer: &mut LiveRenderer, p_effects: &[&dyn GpuEffect]) -> Result<(), String> {
  let (width, height) = p_renderer.size().ok_or("no source image has been uploaded")?;
  let passes: Vec<Vec<GpuPass>> = p_effects.iter().map(|effect| effect.passes(width, height)).collect();
  p_renderer.render(&passes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra::abra_core::Image;
  use abra::abra_core::image::gpu::{GpuOp, GpuPass};

  const BRIGHTNESS: &str = include_str!("../../../../../abra/core/adjustments/src/levels/brightness.wgsl");

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

    let mut expected = Image::new_from_pixels(13, 7, backend.render_blocking(&[]).unwrap(), abra::abra_core::Channels::RGBA);
    for effect in chain {
      effect.apply_cpu(&mut expected);
    }
    close(&out, expected.rgba());
  }

  #[test]
  fn a_chain_that_starts_with_a_cpu_effect_starts_from_the_original() {
    let Some(mut backend) = backend() else { return };
    let original = backend.render_blocking(&[]).unwrap();
    // A GPU-only chain leaves its result in the session; the next chain must not start from it.
    backend.render_blocking(&[&Bright(0.5)]).unwrap();
    let out = backend.render_blocking(&[&Invert]).unwrap();
    let expected: Vec<u8> =
      original.chunks_exact(4).flat_map(|p| [255 - p[0], 255 - p[1], 255 - p[2], p[3]]).collect();
    close(&out, &expected);
  }

  #[test]
  fn rendering_again_starts_from_the_original_source() {
    let Some(mut backend) = backend() else { return };
    let original = backend.render_blocking(&[]).unwrap();
    backend.render_blocking(&[&Invert]).unwrap();
    assert_eq!(backend.render_blocking(&[]).unwrap(), original);
  }

  #[test]
  fn a_masked_shader_effect_matches_its_cpu_result() {
    let Some(mut backend) = backend() else { return };
    let weights: Vec<f32> = (0..13 * 7).map(|i| if i % 13 < 4 { 1.0 } else if i % 13 < 8 { 0.5 } else { 0.0 }).collect();
    let masked = crate::live_image::masked::MaskedEffect::new(Arc::new(Bright(0.5)), 13, 7, &weights);
    assert!(masked.has_gpu());

    let out = backend.render_blocking(&[&masked]).unwrap();
    let mut expected = Image::new_from_pixels(13, 7, backend.render_blocking(&[]).unwrap(), abra::abra_core::Channels::RGBA);
    masked.apply_cpu(&mut expected);
    close(&out, expected.rgba());
  }

  #[test]
  fn masking_an_effect_keeps_it_working_after_another_effect() {
    let Some(mut backend) = backend() else { return };
    let weights = vec![0.0f32; 13 * 7];
    let none = crate::live_image::masked::MaskedEffect::new(Arc::new(Bright(0.5)), 13, 7, &weights);
    let original = backend.render_blocking(&[]).unwrap();
    // Weight 0 everywhere: the masked effect must hand back exactly what it was given, which is the first effect's result.
    let out = backend.render_blocking(&[&Bright(0.5), &none]).unwrap();
    let plain = backend.render_blocking(&[&Bright(0.5)]).unwrap();
    close(&out, &plain);
    assert_ne!(out, original);
  }
}
