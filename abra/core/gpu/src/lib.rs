//! GPU runtime helpers for the workspace
//!
//! This crate provides a small wrapper around `wgpu` to create a headless
//! GPU context and helper functions to upload/download textures.
#![deny(missing_docs)]

pub mod context;
pub mod present;
pub mod renderer;

pub use context::GpuContext;
pub use present::Presenter;
pub use renderer::{Frame, LiveRenderer};

use abra_core::Settings;
use abra_core::image::gpu::{GpuEffect, GpuProvider, GpuSession, register_gpu_provider};
use std::sync::{Mutex, OnceLock};

/// Register this crate as `core`'s GPU provider.
///
/// When GPU is enabled in settings, the GPU context is created on a background thread so that
/// startup cost (~200ms) overlaps with other work. A GPU operation that runs before it is ready
/// waits for it to finish.
pub fn register() {
  register_gpu_provider(GpuProvider {
    wait_until_ready: || context().is_some(),
    process,
    new_session,
  });
  if Settings::gpu_enabled() {
    std::thread::spawn(context);
  }
}

/// The shared GPU context, created once (waiting for it if it is still starting). `None` when no adapter is available.
pub fn context() -> Option<&'static GpuContext> {
  static CONTEXT: OnceLock<Option<GpuContext>> = OnceLock::new();
  CONTEXT.get_or_init(|| GpuContext::new_default_blocking().ok()).as_ref()
}

/// Most idle renderers kept for one-shot calls. Each holds textures the size of the last image it processed.
const MAX_IDLE_RENDERERS: usize = 4;

/// Idle renderers behind [`process`]. Compiled pipelines are shared through the context, so a renderer only holds
/// textures, and one-shot calls on different threads do not wait for each other.
fn idle_renderers() -> &'static Mutex<Vec<LiveRenderer>> {
  static IDLE: OnceLock<Mutex<Vec<LiveRenderer>>> = OnceLock::new();
  IDLE.get_or_init(|| Mutex::new(Vec::new()))
}

/// Takes the idle renderer whose textures already fit `p_width` x `p_height`, else any idle one, else a new one.
fn take_renderer(p_ctx: &GpuContext, p_width: u32, p_height: u32) -> LiveRenderer {
  let mut idle = idle_renderers().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let index = idle.iter().position(|renderer| renderer.size() == Some((p_width, p_height)));
  match index {
    Some(index) => idle.swap_remove(index),
    None => idle.pop().unwrap_or_else(|| LiveRenderer::new(p_ctx.clone())),
  }
}

fn process(p_effect: &dyn GpuEffect, p_width: u32, p_height: u32, p_pixels: &[u8]) -> Result<Vec<u8>, String> {
  let ctx = context().ok_or("no GPU adapter available")?;
  let mut renderer = take_renderer(ctx, p_width, p_height);
  let result = renderer.process(&[p_effect], p_width, p_height, p_pixels).map_err(|e| e.to_string());
  let mut idle = idle_renderers().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  if idle.len() < MAX_IDLE_RENDERERS {
    idle.push(renderer);
  }
  result
}

fn new_session() -> Result<Box<dyn GpuSession>, String> {
  let ctx = context().ok_or("no GPU adapter available")?;
  Ok(Box::new(LiveRenderer::new(ctx.clone())))
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::image::gpu::GpuOp;

  #[test]
  fn process_brightness_through_shared_renderer() -> anyhow::Result<()> {
    // 2x2 image: white, middle gray, red, green
    let pixels: Vec<u8> = vec![255, 255, 255, 255, 128, 128, 128, 255, 255, 0, 0, 255, 0, 255, 0, 255];
    let op = GpuOp::new(include_str!("../../adjustments/src/levels/brightness.wgsl"), 0.5f32.to_le_bytes());
    let result = process(&op, 2, 2, &pixels).map_err(anyhow::Error::msg)?;
    assert_eq!(result.len(), pixels.len());
    // 128 * 0.5 = 64, and alpha is untouched.
    assert!(result[4].abs_diff(64) <= 1);
    assert_eq!(result[7], 255);
    Ok(())
  }

  #[test]
  fn concurrent_one_shot_calls_do_not_share_state() -> anyhow::Result<()> {
    let op = GpuOp::new(include_str!("../../adjustments/src/levels/brightness.wgsl"), 0.5f32.to_le_bytes());
    let small: Vec<u8> = [200, 200, 200, 255].repeat(4 * 4);
    let large: Vec<u8> = [100, 100, 100, 255].repeat(9 * 5);
    std::thread::scope(|scope| {
      let handles: Vec<_> = (0..8)
        .map(|i| {
          let (width, height, pixels, expected) =
            if i % 2 == 0 { (4, 4, &small, 100) } else { (9, 5, &large, 50) };
          let op = &op;
          scope.spawn(move || {
            let out = process(op, width, height, pixels).unwrap();
            assert_eq!(out.len(), pixels.len());
            assert!(out[0].abs_diff(expected) <= 1);
          })
        })
        .collect();
      for handle in handles {
        handle.join().unwrap();
      }
    });
    Ok(())
  }
}
