//! A layer with a drop shadow and a stroke, and a canvas with a shadow, are made as GPU chains and give the pixels the CPU does. This file has one
//! test because it switches the GPU off to make the CPU's picture, and the switch is for the whole program.

use abra_core::image::gpu::{Hardware, clear_gpu_provider, ready_gpu_provider};
use abra_core::{BlendMode, Channels, Color, Image};
use canvas::{Canvas, DropShadow, LayerEffects, Stroke};

fn picture() -> Image {
  let mut pixels = Vec::new();
  for y in 0..40u32 {
    for x in 0..50u32 {
      // A round shape with a soft edge, so the shadow has an outline to follow.
      let (dx, dy) = (x as f32 - 25.0, y as f32 - 20.0);
      let alpha = (255.0 - ((dx * dx + dy * dy).sqrt() - 14.0) * 40.0).clamp(0.0, 255.0) as u8;
      pixels.extend_from_slice(&[(x * 5) as u8, (y * 6) as u8, 120, alpha]);
    }
  }
  Image::new_from_pixels(50, 40, pixels, Channels::RGBA)
}

fn composed() -> Vec<u8> {
  let canvas = Canvas::new_blank("effects", 90, 80);
  canvas.add_layer_from_image("back", Image::new_from_color(90, 80, Color::from_rgba(230, 230, 200, 255)), None);
  let layer = canvas.add_layer_from_image("shape", picture(), None);
  layer.set_effects(
    LayerEffects::new()
      .with_drop_shadow(DropShadow::new().with_distance(6).with_angle(40).with_size(5).with_opacity(0.6).with_blend_mode(BlendMode::Multiply))
      .with_stroke(Stroke::new().with_size(2).with_fill(abra_core::Fill::Solid(Color::from_rgba(200, 0, 0, 255)))),
  );
  canvas.as_image().rgba().to_vec()
}

#[test]
fn a_shadow_and_a_stroke_on_a_layer_match_the_cpu() {
  gpu::register();
  assert!(ready_gpu_provider(Hardware::Gpu).is_some(), "this test needs a GPU");
  let on_gpu = composed();
  clear_gpu_provider();
  let on_cpu = composed();
  assert_eq!(on_gpu.len(), on_cpu.len());
  let differing = on_gpu.iter().zip(&on_cpu).filter(|(a, b)| a.abs_diff(**b) > 4).count();
  let largest = on_gpu.iter().zip(&on_cpu).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
  assert!(differing == 0, "{differing} values differ by more than 4 (largest {largest})");
}
