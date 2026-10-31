//! The GPU draws a particle system the way the CPU does. Each test makes a system, draws it on the CPU and on the GPU over the same
//! picture, and compares the two. The particles are made by the CPU either way, so any difference is the drawing. Dots, textures and
//! the blend modes are the same to a level or two; a trail is a line the CPU rasterizes as a brush stroke and the GPU works out for each
//! pixel, so only its edges differ.

use abra::abra_core::image::gpu::GpuProcessor;
use abra::prelude::*;
use abra_particle_system::prelude::*;
use gpu::{GpuContext, LiveRenderer};

const WIDTH: u32 = 200;
const HEIGHT: u32 = 150;

/// A picture with something in every pixel, so a color taken from it and a blend over it both show.
fn picture() -> Image {
  let mut pixels = Vec::new();
  for y in 0..HEIGHT {
    for x in 0..WIDTH {
      let (fx, fy) = (x as f32 / WIDTH as f32, y as f32 / HEIGHT as f32);
      pixels.extend([(fx * 255.0) as u8, (fy * 255.0) as u8, ((fx * fy) * 255.0) as u8, 255]);
    }
  }
  Image::new_from_pixels(WIDTH, HEIGHT, pixels, Channels::RGBA)
}

fn on_cpu(p_system: &ParticleSystem) -> Image {
  let mut image = picture();
  p_system.apply_on_cpu(&mut image);
  image
}

fn on_gpu(p_system: &ParticleSystem) -> Image {
  assert!(p_system.gpu_processor().is_some(), "the system is meant to be one the GPU draws");
  let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking().expect("a GPU to test with"));
  let pixels = renderer.process(&[p_system.passes(WIDTH, HEIGHT)], WIDTH, HEIGHT, &picture().to_rgba_vec()).expect("the GPU draws it");
  Image::new_from_pixels(WIDTH, HEIGHT, pixels, Channels::RGBA)
}

/// How the two pictures differ: the largest difference of any channel, how many pixels differ at all, and the mean difference.
fn compare(p_cpu: &Image, p_gpu: &Image) -> (u8, f32, f32) {
  let (cpu, gpu) = (p_cpu.to_rgba_vec(), p_gpu.to_rgba_vec());
  let (mut largest, mut differing, mut total) = (0u8, 0usize, 0u64);
  for (a, b) in cpu.chunks_exact(4).zip(gpu.chunks_exact(4)) {
    let pixel = a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
    largest = largest.max(pixel);
    differing += (pixel > 0) as usize;
    total += a.iter().zip(b).map(|(a, b)| a.abs_diff(*b) as u64).sum::<u64>();
  }
  let pixels = cpu.len() / 4;
  (largest, differing as f32 / pixels as f32, total as f32 / cpu.len() as f32)
}

/// Soft dots all over the picture, in a color of their own.
fn dots() -> ParticleSystem {
  particle_system()
    .with_seed(3)
    .with_count(260)
    .with_size((3.0, 14.0))
    .with_opacity((0.3, 1.0))
    .with_start_color(Gradient::from_to(Color::from_rgb(255, 90, 20), Color::from_rgb(40, 200, 255)))
}

#[test]
fn dots_are_drawn_to_the_level() {
  let (largest, differing, _) = compare(&on_cpu(&dots()), &on_gpu(&dots()));
  assert!(largest <= 2, "the largest difference is {largest}");
  assert!(differing < 0.02, "{} of the pixels differ", differing * 100.0);
}

#[test]
fn every_blend_is_drawn_to_the_level() {
  for blend in [
    Blend::Additive,
    Blend::from(BlendMode::Normal),
    Blend::from(BlendMode::Multiply),
    Blend::from(BlendMode::Screen),
    Blend::from(BlendMode::Overlay),
    Blend::from(BlendMode::Difference),
    Blend::from(BlendMode::ColorDodge),
    Blend::from(BlendMode::SoftLight),
  ] {
    let system = dots().with_blend(blend);
    let (largest, differing, _) = compare(&on_cpu(&system), &on_gpu(&system));
    assert!(largest <= 2 && differing < 0.02, "{blend:?}: largest {largest}, {} differ", differing * 100.0);
  }
}

#[test]
fn a_color_taken_from_the_picture_is_read_by_the_gpu() {
  // No color is set, so each dot takes the color of the picture where it was born, which only the shader can read.
  let system = particle_system().with_seed(9).with_count(300).with_size((2.0, 9.0)).with_opacity(1.0);
  let (largest, differing, _) = compare(&on_cpu(&system), &on_gpu(&system));
  assert!(largest <= 2 && differing < 0.02, "largest {largest}, {} differ", differing * 100.0);
}

#[test]
fn textures_are_drawn_turned_and_tinted() {
  // A texture that is not the same all over, with a see-through corner, turned by a different amount for each particle.
  let mut pixels = Vec::new();
  for y in 0..16u32 {
    for x in 0..24u32 {
      pixels.extend([(x * 10) as u8, (y * 15) as u8, 200, if x + y < 6 { 0 } else { 255 }]);
    }
  }
  let texture = Image::new_from_pixels(24, 16, pixels, Channels::RGBA);
  let plain = particle_system().with_seed(5).with_count(40).with_size((8.0, 22.0)).with_opacity((0.5, 1.0)).with_rotation((-90.0, 90.0)).with_texture(texture);
  let tinted = plain.clone().with_start_color(Gradient::from_to(Color::from_rgb(255, 200, 120), Color::from_rgb(120, 200, 255)));
  for system in [plain, tinted] {
    let (largest, differing, _) = compare(&on_cpu(&system), &on_gpu(&system));
    assert!(largest <= 2 && differing < 0.02, "largest {largest}, {} differ", differing * 100.0);
  }
}

#[test]
fn children_born_from_a_particle_are_drawn() {
  let sparks = particle_system().with_seed(1).with_count(6).with_size((1.0, 3.0)).with_start_speed((-40.0, 40.0), (-40.0, 40.0)).with_blend(Blend::Additive);
  let system = dots().with_count(60).with_sub_emitter(0.4, sparks);
  let (largest, differing, _) = compare(&on_cpu(&system), &on_gpu(&system));
  assert!(largest <= 2 && differing < 0.02, "largest {largest}, {} differ", differing * 100.0);
}

#[test]
fn trails_are_drawn_to_within_their_edges() {
  let system = particle_system()
    .with_seed(11)
    .with_count(30)
    .with_emitter(Emitter::Line((20.0, 140.0), (180.0, 140.0)))
    .with_size((Curve::from_to(4.0, 1.0), Curve::from_to(6.0, 1.5)))
    .with_opacity(0.9)
    .with_color_over_lifetime(Gradient::from_to(Color::from_rgb(255, 240, 200), Color::from_rgb(255, 90, 20)))
    .with_start_speed((-30.0, 30.0), (-170.0, -110.0))
    .with_gravity(80.0)
    .with_progress(1.0)
    .with_trail(0.9)
    .with_trail_hardness(0.6)
    .with_blend(Blend::Additive);
  let (cpu, gpu) = (on_cpu(&system), on_gpu(&system));
  let (largest, differing, mean) = compare(&cpu, &gpu);
  // The two are the same lines, so the picture as a whole is the same, and they part only at the soft edge of a stroke.
  assert!(mean < 1.5, "the mean difference is {mean}");
  assert!(largest < 160, "the largest difference is {largest}");
  assert!(differing > 0.0, "a stroke is not drawn the same by the two");
  // Where a trail is, the GPU draws it: the picture changed there as it did on the CPU.
  let changed = |image: &Image| picture().rgba().chunks_exact(4).zip(image.rgba().chunks_exact(4)).filter(|(a, b)| a != b).count();
  let (on_the_cpu, on_the_gpu) = (changed(&cpu) as f32, changed(&gpu) as f32);
  assert!((on_the_gpu / on_the_cpu - 1.0).abs() < 0.15, "{on_the_cpu} pixels changed on the CPU and {on_the_gpu} on the GPU");
}

#[test]
fn many_particles_over_many_tiles_are_all_drawn() {
  // Big soft particles each touch many tiles, and small ones touch one; every one of them has to be found by every pixel it covers.
  let system = particle_system()
    .with_seed(21)
    .with_count(2500)
    .with_size((1.0, 40.0))
    .with_opacity((0.05, 0.4))
    .with_start_color(Gradient::from_to(Color::from_rgb(10, 40, 120), Color::from_rgb(230, 240, 255)));
  let (largest, differing, _) = compare(&on_cpu(&system), &on_gpu(&system));
  assert!(largest <= 2 && differing < 0.05, "largest {largest}, {} differ", differing * 100.0);
}

#[test]
fn a_system_the_shader_cannot_draw_is_left_to_the_cpu() {
  // A luminance emitter picks pixels of the picture, which the particles are made without on the GPU.
  assert!(particle_system().with_emitter(Emitter::Luminance(0.5)).gpu_processor().is_none());
  // A color taken from the picture cannot be turned in hue or put in a trail by the shader.
  assert!(particle_system().with_hue_variation(40.0).gpu_processor().is_none());
  assert!(particle_system().with_trail(0.5).gpu_processor().is_none());
  assert!(dots().with_hue_variation(40.0).with_trail(0.5).gpu_processor().is_some());
  // A custom blend mode is a Rust function, and has no shader.
  fn custom(a: (u8, u8, u8, u8), _b: (u8, u8, u8, u8)) -> (u8, u8, u8, u8) {
    a
  }
  assert!(dots().with_blend(BlendMode::Custom(custom)).gpu_processor().is_none());
  // And a child that cannot be drawn by the shader keeps the whole system on the CPU.
  assert!(dots().with_sub_emitter(0.5, particle_system().with_trail(0.4)).gpu_processor().is_none());
  assert!(dots().with_sub_emitter(0.5, dots().with_trail(0.4)).gpu_processor().is_some());
}
