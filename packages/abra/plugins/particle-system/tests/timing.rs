//! A measurement, not a check: where the time goes when a swarm with trails is drawn frame after frame on the GPU, the way the mobile particle maker
//! draws it. Run it with `cargo test -p particle-system --release --test timing -- --ignored --nocapture`.

use std::time::{Duration, Instant};

use abra::abra_core::image::gpu::GpuProcessor;
use abra::prelude::*;
use abra_particle_system::prelude::*;
use gpu::{GpuContext, LiveRenderer};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 768;
const FRAMES: usize = 60;

/// The "Fire" system of the particle maker as it is saved on the phone, in the pixels `toSystem` turns its shares of the shorter side into.
fn fire(p_trail: f32) -> ParticleSystem {
  let shorter = HEIGHT as f32;
  particle_system()
    .with_seed(1)
    .with_emitter(Emitter::Circle { center: (0.5408 * WIDTH as f32, 0.7721 * HEIGHT as f32), radius: 0.04 * shorter })
    .with_count(949)
    .with_size((0.5 * shorter / 100.0, 2.4 * shorter / 100.0))
    .with_hardness(0.0)
    .with_opacity((0.5, 0.9))
    .with_blend(Blend::Additive)
    .with_start_color(Gradient::from_to(Color::from_rgb(65, 88, 231), Color::from_rgb(117, 22, 202)))
    .with_color_over_lifetime(Gradient::from_to(Color::from_rgba(255, 255, 255, 128), Color::from_rgba(255, 214, 10, 0)))
    .with_start_speed((-0.29 * shorter, 0.2 * shorter), (-0.7 * shorter, -0.32 * shorter))
    .with_launch(Impulse::new(0.0, 30.0, (0.0, 0.45 * shorter)))
    .with_gravity(-0.15 * shorter)
    .with_radial_velocity((-0.1 * shorter, 0.02 * shorter))
    .with_noise((0.03 * shorter, 0.15 * shorter), shorter / 10.0)
    .with_noise_scroll(2.0 * shorter, 0.0)
    .with_age_spread(0.7)
    .with_looping(true)
    .with_trail(p_trail)
    .with_trail_fade(0.2)
    .with_trail_hardness(0.1)
}

fn median(p_times: &mut [Duration]) -> Duration {
  p_times.sort();
  p_times[p_times.len() / 2]
}

#[test]
#[ignore = "a measurement, not a check"]
fn where_the_time_goes_drawing_the_fire_swarm_frame_after_frame() {
  let photo = Image::new_from_color(WIDTH, HEIGHT, Color::from_rgba(10, 12, 28, 255));
  let rgba = photo.to_rgba_vec();
  let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking().expect("a GPU to measure with"));
  renderer.set_source(WIDTH, HEIGHT, &rgba).expect("the backdrop is the source");
  for trail in [0.09f32, 0.0] {
    let (swarm, textures) = fire(trail).swarm(WIDTH, HEIGHT, Some(&photo));
    let (mut sprite_times, mut pass_times, mut gpu_times, mut again_times) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut words = 0usize;
    for frame in 0..FRAMES + 5 {
      let progress = (frame as f32 * 0.016) % 1.0;
      let started = Instant::now();
      let batch = swarm.sprites(progress, Blend::Additive, &textures);
      let sprites = started.elapsed();
      let started = Instant::now();
      let passes = batch.passes(WIDTH, HEIGHT);
      let made = started.elapsed();
      if let Some(aux) = passes.first().and_then(|pass| pass.aux.as_ref()) {
        words = aux.rgba.len() / 4;
        if frame == 30 {
          // What every pixel of a tile walks: the trails (four words each) and sprites (one word) listed for it.
          let word = |at: usize| u32::from_le_bytes(aux.rgba[at * 4..at * 4 + 4].try_into().unwrap());
          let uniform = |at: usize| u32::from_le_bytes(passes[0].uniforms[at * 4..at * 4 + 4].try_into().unwrap()) as usize;
          let (tiles_at, list_at, tiles_x) = (uniform(4), uniform(5), uniform(7));
          let tiles = tiles_x * (HEIGHT as usize).div_ceil(16);
          let (mut trail_entries, mut pieces, mut sprite_entries): (Vec<usize>, usize, Vec<usize>) = (Vec::new(), 0, Vec::new());
          for tile in 0..tiles {
            let (mut k, end) = (word(tiles_at + tile) as usize, word(tiles_at + tile + 1) as usize);
            let (mut trails_here, mut sprites_here) = (0, 0);
            while k < end {
              if word(list_at + k) & 0x8000_0000 != 0 {
                trails_here += 1;
                let (first, last, mask) = (word(list_at + k + 1), word(list_at + k + 2), word(list_at + k + 3));
                // A range the mask holds is tested piece by piece as it says; a longer one is tested whole.
                pieces += if last - first < 32 { mask.count_ones() } else { last - first + 1 } as usize;
                k += 4;
              } else {
                sprites_here += 1;
                k += 1;
              }
            }
            trail_entries.push(trails_here);
            sprite_entries.push(sprites_here);
          }
          let total: usize = trail_entries.iter().sum();
          let mut sorted = trail_entries.clone();
          sorted.sort();
          println!(
            "  lists of {tiles} tiles: trails per tile mean {:.1}, median {}, p99 {}, max {} | pieces tested per trail entry {:.1} | sprites per tile mean {:.1}, max {}",
            total as f32 / tiles as f32,
            sorted[tiles / 2],
            sorted[tiles * 99 / 100],
            sorted[tiles - 1],
            pieces as f32 / total.max(1) as f32,
            sprite_entries.iter().sum::<usize>() as f32 / tiles as f32,
            sprite_entries.iter().max().unwrap()
          );
        }
      }
      let started = Instant::now();
      let again = passes.clone();
      renderer.render(&[passes]).expect("the GPU draws it");
      renderer.read_pixels().expect("the picture comes back");
      let drawn = started.elapsed();
      // The same passes again: the aux texture is the same, so nothing is uploaded and this is the shader and the read back alone.
      let started = Instant::now();
      renderer.render(&[again]).expect("the GPU draws it again");
      renderer.read_pixels().expect("the picture comes back again");
      let repeated = started.elapsed();
      if frame >= 5 {
        sprite_times.push(sprites);
        pass_times.push(made);
        gpu_times.push(drawn);
        again_times.push(repeated);
      }
    }
    println!(
      "trail {trail}: {words} words ({:.2} MB) uploaded | swarm.sprites {:?} | passes() {:?} | render + read back {:?} | same passes again (no upload) {:?}",
      words as f32 * 4.0 / 1_048_576.0,
      median(&mut sprite_times),
      median(&mut pass_times),
      median(&mut gpu_times),
      median(&mut again_times)
    );
  }
}
