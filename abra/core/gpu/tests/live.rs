use abra_core::image::gpu::GpuOp;
use anyhow::Result;
use gpu::{GpuContext, LiveRenderer};

const BRIGHTNESS: &str = include_str!("../../adjustments/src/levels/brightness.wgsl");
const CONTRAST: &str = include_str!("../../adjustments/src/levels/contrast.wgsl");

fn brightness(p_factor: f32) -> GpuOp {
  GpuOp::new(BRIGHTNESS, p_factor.to_le_bytes())
}

/// A width that is not a multiple of the workgroup size or the row alignment, to exercise edge handling.
fn gradient_pixels(p_width: u32, p_height: u32) -> Vec<u8> {
  let mut pixels = Vec::new();
  for y in 0..p_height {
    for x in 0..p_width {
      pixels.extend_from_slice(&[(x * 10) as u8, (y * 20) as u8, 100, 255]);
    }
  }
  pixels
}

fn renderer() -> Result<LiveRenderer> {
  Ok(LiveRenderer::new(GpuContext::new_default_blocking()?))
}

#[test]
fn no_effects_returns_the_source_unchanged() -> Result<()> {
  let mut renderer = renderer()?;
  let pixels = gradient_pixels(13, 7);
  let out = renderer.process(&[], 13, 7, &pixels)?;
  assert_eq!(out, pixels);
  Ok(())
}

#[test]
fn changing_a_parameter_reuses_the_pipeline_and_the_uploaded_source() -> Result<()> {
  let mut renderer = renderer()?;
  let pixels = gradient_pixels(13, 7);
  renderer.set_source(13, 7, &pixels)?;

  renderer.render(&[&brightness(0.5)])?;
  let dark = renderer.read_pixels()?;
  renderer.render(&[&brightness(1.0)])?;
  let same = renderer.read_pixels()?;

  assert_eq!(renderer.cached_pipelines(), 1);
  assert_ne!(dark, same);
  // Factor 1.0 leaves the image as it was, so the source was not consumed by the first render.
  for (a, b) in same.iter().zip(&pixels) {
    assert!(a.abs_diff(*b) <= 1);
  }
  // Factor 0.5 halves the color channels and leaves alpha alone.
  for (out, src) in dark.chunks_exact(4).zip(pixels.chunks_exact(4)) {
    assert!(out[0].abs_diff(src[0] / 2) <= 1);
    assert_eq!(out[3], 255);
  }
  Ok(())
}

#[test]
fn effects_chain_in_order_without_leaving_the_gpu() -> Result<()> {
  let mut renderer = renderer()?;
  let pixels = gradient_pixels(13, 7);
  let contrast = GpuOp::new(CONTRAST, 50.0f32.to_le_bytes());

  let chained = renderer.process(&[&brightness(0.5), &contrast], 13, 7, &pixels)?;

  let step_one = renderer.process(&[&brightness(0.5)], 13, 7, &pixels)?;
  let step_two = renderer.process(&[&contrast], 13, 7, &step_one)?;
  for (a, b) in chained.iter().zip(&step_two) {
    assert!(a.abs_diff(*b) <= 1, "chained {a} vs stepwise {b}");
  }
  assert_eq!(renderer.cached_pipelines(), 2);
  Ok(())
}

#[test]
fn a_different_image_size_is_handled_without_stale_textures() -> Result<()> {
  let mut renderer = renderer()?;
  let small = gradient_pixels(5, 3);
  let large = gradient_pixels(70, 33);

  let out_small = renderer.process(&[&brightness(1.0)], 5, 3, &small)?;
  let out_large = renderer.process(&[&brightness(1.0)], 70, 33, &large)?;
  let out_small_again = renderer.process(&[&brightness(1.0)], 5, 3, &small)?;

  assert_eq!(out_small.len(), small.len());
  assert_eq!(out_large.len(), large.len());
  assert_eq!(out_small, out_small_again);
  Ok(())
}

#[test]
fn rendering_before_a_source_is_an_error() -> Result<()> {
  let mut renderer = renderer()?;
  assert!(renderer.render(&[&brightness(1.0)]).is_err());
  assert!(renderer.set_source(2, 2, &[0; 3]).is_err());
  Ok(())
}

#[test]
fn frames_are_read_back_without_blocking_the_render_loop() -> Result<()> {
  let mut renderer = renderer()?;
  let pixels = gradient_pixels(13, 7);
  renderer.set_source(13, 7, &pixels)?;

  renderer.render(&[&brightness(0.5)])?;
  let id = renderer.request_frame()?.expect("a free staging buffer");
  let frame = renderer.wait_frame()?.expect("the requested frame");

  assert_eq!(frame.id, id);
  assert_eq!((frame.width, frame.height), (13, 7));
  assert!(frame.pixels[0].abs_diff(pixels[0] / 2) <= 1);
  assert!(renderer.poll_frame()?.is_none());
  Ok(())
}

#[test]
fn a_slow_reader_only_sees_the_newest_frame() -> Result<()> {
  let mut renderer = renderer()?;
  let pixels = vec![200u8; 13 * 7 * 4];
  renderer.set_source(13, 7, &pixels)?;

  // Three drag steps queued before the reader gets to look.
  let mut last = 0;
  for factor in [0.25f32, 0.5, 1.0] {
    renderer.render(&[&brightness(factor)])?;
    last = renderer.request_frame()?.expect("a free staging buffer");
  }
  // The pool is full, so a fourth request is refused until a frame is collected.
  renderer.render(&[&brightness(1.0)])?;
  assert!(renderer.request_frame()?.is_none());

  let frame = renderer.wait_frame()?.expect("the newest frame");
  assert_eq!(frame.id, last);
  assert_eq!(frame.pixels[0], 200);
  assert!(renderer.wait_frame()?.is_none(), "older frames were discarded, not queued");
  assert!(renderer.request_frame()?.is_some(), "collecting frees the staging buffers");
  Ok(())
}

#[test]
fn readback_follows_a_size_change() -> Result<()> {
  let mut renderer = renderer()?;
  renderer.set_source(13, 7, &gradient_pixels(13, 7))?;
  renderer.render(&[])?;
  renderer.request_frame()?;
  renderer.set_source(20, 9, &gradient_pixels(20, 9))?;
  renderer.render(&[])?;
  renderer.request_frame()?;
  let frame = renderer.wait_frame()?.expect("a frame");
  assert_eq!((frame.width, frame.height), (20, 9));
  assert_eq!(frame.pixels, gradient_pixels(20, 9));
  Ok(())
}

#[test]
fn a_new_renderer_on_the_same_context_starts_with_compiled_pipelines() -> Result<()> {
  let ctx = GpuContext::new_default_blocking()?;
  let pixels = gradient_pixels(13, 7);
  let mut first = LiveRenderer::new(ctx.clone());
  first.process(&[&brightness(0.5)], 13, 7, &pixels)?;
  assert_eq!(first.cached_pipelines(), 1);

  let second = LiveRenderer::new(ctx);
  assert_eq!(second.cached_pipelines(), 1);
  Ok(())
}
