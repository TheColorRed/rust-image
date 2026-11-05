//! A measurement, not a check: whether memory keeps growing when the particle maker's preview is drawn frame after frame the way the phone draws
//! it (a layer whose live effect is replaced, the canvas composed, the GPU run planned and drawn through a tiled photo). Run it with
//! `cargo test -p particle-system --release --test leak -- --ignored --nocapture`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use abra::abra_core::image::gpu::LiveEffect;
use abra::abra_core::image::recipe::gpu_run;
use abra::canvas::prelude::Canvas;
use abra::prelude::*;
use abra_particle_system::prelude::*;
use gpu::{TileOptions, TiledRenderer, Window};

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
  unsafe fn alloc(&self, p_layout: Layout) -> *mut u8 {
    LIVE.fetch_add(p_layout.size(), Ordering::Relaxed);
    unsafe { System.alloc(p_layout) }
  }
  unsafe fn dealloc(&self, p_pointer: *mut u8, p_layout: Layout) {
    LIVE.fetch_sub(p_layout.size(), Ordering::Relaxed);
    unsafe { System.dealloc(p_pointer, p_layout) }
  }
  unsafe fn realloc(&self, p_pointer: *mut u8, p_layout: Layout, p_size: usize) -> *mut u8 {
    LIVE.fetch_add(p_size, Ordering::Relaxed);
    LIVE.fetch_sub(p_layout.size(), Ordering::Relaxed);
    unsafe { System.realloc(p_pointer, p_layout, p_size) }
  }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 768;
const FRAMES: usize = 600;

fn fire() -> ParticleSystem {
  let shorter = HEIGHT as f32;
  particle_system()
    .with_seed(1)
    .with_emitter(Emitter::Circle { center: (0.54 * WIDTH as f32, 0.77 * HEIGHT as f32), radius: 0.04 * shorter })
    .with_count(949)
    .with_size((0.5 * shorter / 100.0, 2.4 * shorter / 100.0))
    .with_opacity((0.5, 0.9))
    .with_blend(Blend::Additive)
    .with_start_speed((-0.29 * shorter, 0.2 * shorter), (-0.7 * shorter, -0.32 * shorter))
    .with_gravity(-0.15 * shorter)
    .with_looping(true)
    .with_trail(0.09)
    .with_trail_fade(0.2)
}

fn megabytes() -> f32 {
  LIVE.load(Ordering::Relaxed) as f32 / 1_048_576.0
}

#[test]
#[ignore = "a measurement, not a check"]
fn memory_while_the_preview_is_drawn_frame_after_frame() {
  gpu::register();
  let context = gpu::context().expect("a GPU to measure with").clone();
  let photo = Image::new_from_color(WIDTH, HEIGHT, Color::from_rgba(10, 12, 28, 255));
  let (swarm, textures) = fire().swarm(WIDTH, HEIGHT, Some(&photo));
  for stage in 0..3 {
    let canvas = Canvas::new("preview");
    let layer = canvas.add_layer_from_image("backdrop", photo.clone(), None);
    let mut held: Vec<(u64, TiledRenderer)> = Vec::new();
    let mut readings = Vec::new();
    for frame in 0..FRAMES {
      let progress = (frame as f32 * 0.016) % 1.0;
      let batch = swarm.sprites(progress, Blend::Additive, &textures);
      let effect: Arc<dyn LiveEffect> = Arc::new(batch);
      layer.set_live_effects(vec![effect]);
      let image = canvas.as_image();
      if stage >= 1 {
        let (root, passes) = gpu_run(&image).unwrap_or_else(|| (image.clone(), Vec::new()));
        if stage >= 2 {
          let (width, height) = root.dimensions::<u32>();
          let key = root.version();
          if !held.iter().any(|(held_key, _)| *held_key == key) {
            let regions: gpu::Regions = {
              let root = root.clone();
              Arc::new(move |level, left, top, width, height| root.region(level, left, top, width, height))
            };
            let renderer = TiledRenderer::from_regions(context.clone(), width, height, regions, TileOptions::default()).expect("a tiled photo");
            held.push((key, renderer));
            if held.len() > 2 {
              held.remove(0);
            }
          }
          let window = Window { x: 0.0, y: 0.0, width: width as f32, height: height as f32, out_width: 512, out_height: 384, pad: 0, angle: 0.0 };
          let renderer = &mut held.last_mut().unwrap().1;
          renderer.render(&window, &passes).expect("the GPU draws it");
          let _texture = renderer.snapshot_texture().expect("a frame is kept");
        }
      }
      if frame % 100 == 99 {
        readings.push(megabytes());
      }
    }
    println!("stage {stage} (0 compose, 1 + plan, 2 + draw): live MB every 100 frames = {readings:.1?}");
  }
}
