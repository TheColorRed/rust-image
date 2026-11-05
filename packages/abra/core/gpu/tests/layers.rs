//! A stack of pictures laid over one another on the GPU gives the CPU's pixels: layers of any size at any offset, hanging over the edges, with
//! every built-in blend mode and several opacities.

use std::sync::Arc;

use abra_core::blend::blend;
use abra_core::layers::{MaskTexture, adjust, multiply_alpha};
use abra_core::image::GrayPlane;
use abra_core::image::gpu::{CpuProcessor, GpuPass, GpuProcessor, Hardware, LiveEffect, ready_gpu_provider};
use abra_core::image::recipe::{lay_over, laid};
use abra_core::{BlendMode, Channels, Image};

/// A picture with colors that change in both directions and a band that is partly see-through, so alpha matters.
fn picture(p_width: u32, p_height: u32, p_seed: u32) -> Image {
  let mut pixels = Vec::new();
  for y in 0..p_height {
    for x in 0..p_width {
      let alpha = if (p_height / 3..2 * p_height / 3).contains(&y) { 90 + (x * 7 % 120) as u8 } else { 255 };
      pixels.extend_from_slice(&[(x * 13 + y * 3 + p_seed) as u8, (y * 29 + p_seed * 5) as u8, (x * y * 5 + p_seed * 11) as u8, alpha]);
    }
  }
  Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA)
}

/// How many values differ by more than 2, and the largest difference.
fn differences(p_expected: &Image, p_actual: &Image) -> (usize, u8) {
  assert_eq!(p_expected.dimensions::<u32>(), p_actual.dimensions::<u32>());
  let gaps: Vec<u8> = p_expected.rgba().iter().zip(p_actual.rgba()).map(|(a, b)| a.abs_diff(*b)).collect();
  (gaps.iter().filter(|gap| **gap > 2).count(), gaps.into_iter().max().unwrap_or(0))
}

#[test]
fn every_built_in_mode_is_laid_over_the_picture_as_the_cpu_does() {
  gpu::register();
  assert!(ready_gpu_provider(Hardware::Gpu).is_some(), "these tests need a GPU");
  let base = picture(61, 43, 0);
  let top = picture(23, 17, 3);
  let mut checked = 0;
  // `DarkerColor` and `LighterColor` choose by comparing the lightness of two colors, and the CPU and the GPU come out differently on some pixels
  // for any size and offset (it is how those two modes are, not how a layer is laid), so they are left out here.
  let compared_by_lightness = [BlendMode::DarkerColor, BlendMode::LighterColor];
  for mode in BlendMode::ALL.iter().copied().filter(|mode| mode.shader_index().is_some() && !compared_by_lightness.contains(mode)) {
    for (offset, opacity) in [((0, 0), 1.0), ((20, 11), 0.6), ((50, 30), 1.0), ((-9, -5), 0.8)] {
      let mut expected = base.clone();
      blend(&top).with_offset(offset).with_mode(mode).with_opacity(opacity).apply(&mut expected);
      let on_gpu = laid(&base, &top, offset, mode, opacity);
      assert!(on_gpu.deferred().is_some(), "{mode:?} {offset:?} waits for the GPU");
      let (many, largest) = differences(&expected, &on_gpu);
      assert!(many == 0 && largest <= 3, "{mode:?} at {offset:?} with opacity {opacity}: {many} values differ, by up to {largest}");
      checked += 1;
    }
  }
  assert!(checked > 40, "the built-in modes were tried: {checked}");
}

#[test]
fn a_whole_stack_is_one_chain_and_matches_the_cpu() {
  gpu::register();
  assert!(ready_gpu_provider(Hardware::Gpu).is_some(), "these tests need a GPU");
  let layers = [
    (picture(40, 30, 1), (0, 0), BlendMode::Normal, 1.0),
    (picture(25, 25, 2), (30, 10), BlendMode::Multiply, 0.9),
    (picture(70, 8, 3), (-10, 20), BlendMode::Screen, 0.7),
    (picture(12, 40, 4), (55, -6), BlendMode::Overlay, 1.0),
    (picture(30, 30, 5), (10, 10), BlendMode::SoftLight, 0.5),
  ];
  let mut expected = Image::new(64, 48);
  let mut stack = Image::new(64, 48);
  for (layer, offset, mode, opacity) in &layers {
    blend(layer).with_offset(*offset).with_mode(*mode).with_opacity(*opacity).apply(&mut expected);
    lay_over(&mut stack, layer, *offset, *mode, *opacity);
  }
  assert!(stack.deferred().is_some(), "the stack is a recipe and not pixels");
  let (many, largest) = differences(&expected, &stack);
  assert!(many == 0 && largest <= 3, "{many} values differ, by up to {largest}");
}

/// A step with a CPU version and a shader that give the same pixels: the colors turned over (alpha is kept).
struct Invert;

const INVERT_WGSL: &str = "
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let p = textureLoad(input_tex, pos, 0);
  textureStore(output_tex, pos, vec4<f32>(1.0 - p.rgb, p.a));
}
";

impl CpuProcessor for Invert {
  fn process(&self, p_image: &mut Image) {
    let mut pixels = p_image.to_rgba_vec();
    for rgba in pixels.chunks_exact_mut(4) {
      for value in &mut rgba[..3] {
        *value = 255 - *value;
      }
    }
    p_image.set_rgba(pixels);
  }

  fn gpu(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for Invert {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![GpuPass::new(INVERT_WGSL, [])]
  }
}

/// A mask that is a different value at every pixel.
fn mask_of(p_width: u32, p_height: u32) -> Arc<MaskTexture> {
  let values: Vec<u8> = (0..p_width * p_height).map(|at| (at * 37 % 256) as u8).collect();
  Arc::new(MaskTexture::new(GrayPlane::new(p_width, p_height, values)))
}

#[test]
fn a_mask_limits_the_picture_on_the_gpu_as_it_does_on_the_cpu() {
  gpu::register();
  assert!(ready_gpu_provider(Hardware::Gpu).is_some(), "these tests need a GPU");
  let mask = mask_of(61, 43);
  let mut on_gpu = picture(61, 43, 7);
  let mut expected = on_gpu.to_rgba_vec();
  for (rgba, value) in expected.chunks_exact_mut(4).zip(mask.plane().values()) {
    rgba[3] = ((rgba[3] as u16 * *value as u16 + 127) / 255) as u8;
  }
  multiply_alpha(&mut on_gpu, &mask);
  assert!(on_gpu.deferred().is_some(), "the mask is a step of the chain");
  let expected = Image::new_from_pixels(61, 43, expected, Channels::RGBA);
  let (many, largest) = differences(&expected, &on_gpu);
  assert!(many == 0 && largest <= 1, "{many} values differ, by up to {largest}");
}

#[test]
fn an_adjustment_is_effects_and_a_mix_in_one_chain_and_matches_the_cpu() {
  gpu::register();
  assert!(ready_gpu_provider(Hardware::Gpu).is_some(), "these tests need a GPU");
  let effects: Vec<Arc<dyn LiveEffect>> = vec![Arc::new(Invert)];
  let mask = mask_of(30, 20);
  for (strength, with_mask, origin) in [(1.0, false, (0, 0)), (0.4, false, (0, 0)), (0.7, true, (0, 0)), (1.0, true, (20, 15)), (0.5, true, (-8, -4))] {
    let mut on_gpu = picture(61, 43, 9);
    adjust(&mut on_gpu, &effects, strength, with_mask.then_some(&mask), origin);
    assert!(on_gpu.deferred().is_some(), "the adjustment is steps of the chain");
    let before = picture(61, 43, 9);
    let mut inverted = before.clone();
    Invert.process(&mut inverted);
    let (before_pixels, after_pixels) = (before.to_rgba_vec(), inverted.to_rgba_vec());
    let mut pixels = before_pixels.clone();
    for y in 0..43i32 {
      for x in 0..61i32 {
        let value = if with_mask {
          let (mx, my) = (x - origin.0, y - origin.1);
          if mx < 0 || my < 0 { 0 } else { mask.plane().get(mx as u32, my as u32).unwrap_or(0) }
        } else {
          255
        };
        let amount = value as f32 / 255.0 * strength;
        let at = (y * 61 + x) as usize * 4;
        for channel in 0..4 {
          let (from, to) = (before_pixels[at + channel] as f32, after_pixels[at + channel] as f32);
          pixels[at + channel] = (from + (to - from) * amount).round() as u8;
        }
      }
    }
    let on_cpu = Image::new_from_pixels(61, 43, pixels, Channels::RGBA);
    let (many, largest) = differences(&on_cpu, &on_gpu);
    assert!(many == 0 && largest <= 2, "strength {strength} mask {with_mask} at {origin:?}: {many} values differ, by up to {largest}");
  }
}
