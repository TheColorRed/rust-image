//! A canvas with several layers, masks and an adjustment layer is composed on the GPU as one chain and gives the pixels the CPU does.

use abra_core::blend::blend;
use abra_core::image::gpu::{Hardware, LiveEffect, ready_gpu_provider};
use abra_core::{BlendMode, Channels, Image};
use adjustments::levels::brightness;
use canvas::{AdjustmentLayerType, Canvas, NewLayerOptions};
use mask::Mask;
use std::sync::Arc;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;

/// A picture whose colors change in both directions, with a band that is partly see-through.
fn picture(p_seed: u32) -> Image {
  let mut pixels = Vec::new();
  for y in 0..HEIGHT {
    for x in 0..WIDTH {
      let alpha = if (HEIGHT / 3..2 * HEIGHT / 3).contains(&y) { 90 + (x * 7 % 120) as u8 } else { 255 };
      pixels.extend_from_slice(&[(x * 13 + y * 3 + p_seed) as u8, (y * 29 + p_seed * 5) as u8, (x * y * 5 + p_seed * 11) as u8, alpha]);
    }
  }
  Image::new_from_pixels(WIDTH, HEIGHT, pixels, Channels::RGBA)
}

/// A mask with a different value at every pixel.
fn mask(p_step: u32) -> Mask {
  Mask::from_values(WIDTH, HEIGHT, (0..WIDTH * HEIGHT).map(|at| (at * p_step % 256) as u8).collect())
}

fn options(p_mode: BlendMode, p_opacity: f32) -> NewLayerOptions {
  NewLayerOptions { blend_mode: Some(p_mode), opacity: Some(p_opacity), ..NewLayerOptions::default() }
}

#[test]
fn layers_masks_and_an_adjustment_are_one_gpu_chain_that_matches_the_cpu() {
  gpu::register();
  assert!(ready_gpu_provider(Hardware::Gpu).is_some(), "this test needs a GPU");

  let canvas = Canvas::new_blank("stack", WIDTH, HEIGHT);
  canvas.add_layer_from_image("base", picture(1), None);
  let multiplied = canvas.add_layer_from_image("multiply", picture(2), options(BlendMode::Multiply, 0.8));
  multiplied.set_mask(Some(mask(37)));
  canvas.add_layer_from_image("screen", picture(3), options(BlendMode::Screen, 0.6));
  let lighten = canvas.add_adjustment_layer("lighten", AdjustmentLayerType::BrightnessContrast);
  lighten.set_live_effects(vec![Arc::new(brightness(30))]);
  lighten.set_opacity(0.7);
  lighten.set_mask(Some(mask(11)));
  canvas.add_layer_from_image("overlay", picture(4), options(BlendMode::Overlay, 1.0));
  canvas.set_mask(Some(mask(5)));

  // The same stack made by hand on the CPU.
  let mut expected = Image::new(WIDTH, HEIGHT);
  blend(&picture(1)).with_mode(BlendMode::Normal).apply(&mut expected);
  let mut masked = picture(2);
  mask(37).multiply_alpha(&mut masked);
  blend(&masked).with_mode(BlendMode::Multiply).with_opacity(0.8).apply(&mut expected);
  blend(&picture(3)).with_mode(BlendMode::Screen).with_opacity(0.6).apply(&mut expected);
  let mut lightened = expected.clone();
  brightness(30).apply_cpu(&mut lightened);
  let (before, after) = (expected.to_rgba_vec(), lightened.to_rgba_vec());
  let adjustment_mask = mask(11);
  let mut mixed = before.clone();
  for (at, value) in adjustment_mask.values().iter().enumerate() {
    let amount = *value as f32 / 255.0 * 0.7;
    for channel in 0..4 {
      let (from, to) = (before[at * 4 + channel] as f32, after[at * 4 + channel] as f32);
      mixed[at * 4 + channel] = (from + (to - from) * amount).round() as u8;
    }
  }
  let mut expected = Image::new_from_pixels(WIDTH, HEIGHT, mixed, Channels::RGBA);
  blend(&picture(4)).with_mode(BlendMode::Overlay).apply(&mut expected);
  mask(5).multiply_alpha(&mut expected);

  let actual = canvas.as_image();
  let differing = actual.rgba().iter().zip(expected.rgba()).filter(|(a, e)| a.abs_diff(**e) > 3).count();
  let largest = actual.rgba().iter().zip(expected.rgba()).map(|(a, e)| a.abs_diff(*e)).max().unwrap_or(0);
  assert!(differing == 0, "{differing} values differ by more than 3 (largest {largest})");
}
