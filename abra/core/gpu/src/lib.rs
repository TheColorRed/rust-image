//! GPU runtime helpers for the workspace
//!
//! This crate provides a small wrapper around `wgpu` to create a headless
//! GPU context and helper functions to upload/download textures.
#![deny(missing_docs)]

pub mod context;
pub mod image;

pub use context::GpuContext;
pub use image::GpuImage;
use wgpu::TextureFormat::Rgba8Unorm;

use abra_core::{
  Channels, Image,
  image::{
    apply_area::PreparedAreaMeta,
    gpu_op::{GpuOp::*, get_gpu_op, get_gpu_shader},
    gpu_registry::{GpuCallback, register_gpu_provider},
  },
};
use std::sync::Arc;

/// Register a GPU context with the core image processing registry.
///
/// This is a minimal adapter that registers a `GpuCallback` into `core`'s
/// registry. The callback here simply forwards the pixels back unchanged — a
/// real implementation would dispatch compute shaders, but for initial
/// integration this keeps API demoable and safe.
pub fn register_gpu_context(ctx: Arc<GpuContext>) {
  let ctx_clone = ctx.clone();
  let should_process_cb = Arc::new(move |_meta: &PreparedAreaMeta| -> bool {
    // Only process when a GPU operation is set.
    match get_gpu_op() {
      None => false,
      _ => true,
    }
  });
  let process_cb = Arc::new(move |meta: &PreparedAreaMeta, pixels: &[u8]| -> Result<Vec<u8>, String> {
    // Check operation: only handle Brightness for now.
    let bytes = match get_gpu_op() {
      Brightness(amount) => (amount).to_le_bytes(),
      Contrast(amount) => (amount).to_le_bytes(),
      _ => return Err("unsupported gpu operation".to_string()),
    };
    let shader_code = get_gpu_shader().ok_or("missing gpu shader code")?;
    let w = meta.rect_w as u32;
    let h = meta.rect_h as u32;
    let img = Image::new_from_pixels(w, h, pixels.to_vec(), Channels::RGBA);
    let out_bytes = (&*ctx_clone)
      .run_compute_with_image_io(
        &shader_code,
        Some("brightness"),
        "main",
        &img.rgba(),
        w,
        h,
        (8, 8),
        Some(&bytes),
        Rgba8Unorm,
        Rgba8Unorm,
      )
      .map_err(|e| e.to_string())?;
    Ok(out_bytes)
  });
  let cb = Arc::new(GpuCallback {
    should_process: should_process_cb,
    process: process_cb,
  });
  register_gpu_provider(cb);
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Arc;

  #[test]
  fn compute_brightness_via_helper() -> anyhow::Result<()> {
    let ctx = Arc::new(GpuContext::new_default_blocking()?);

    // 2x2 image: two pixels white, two pixels middle gray
    let pixels: Vec<u8> = vec![255, 255, 255, 255, 128, 128, 128, 255, 255, 0, 0, 255, 0, 255, 0, 255];
    let result = (&*ctx).run_compute_with_image_io(
      include_str!("../../adjustments/src/levels/brightness.wgsl"),
      Some("brightness_test"),
      "main",
      &pixels,
      2,
      2,
      (8, 8),
      Some(&1.5f32.to_le_bytes()),
      wgpu::TextureFormat::Rgba8Unorm,
      wgpu::TextureFormat::Rgba8Unorm,
    )?;
    assert_eq!(result.len(), pixels.len());
    Ok(())
  }
}
