use abra::drawing::prelude::*;
use abra::prelude::*;
use std::time::Instant;

const FILE: &str = "assets/aletta-ocean.jpg";
/// How many touch events arrive each frame, and how many frames the drag lasts.
const POINTS_PER_FRAME: usize = 3;
const FRAMES: usize = 120;

/// Simulates a finger drawing a loop around the middle of the photo, one frame at a time. Each frame the line gains a few points
/// and the photo with the line over it has to be made again. It times that two ways: the shader (the line is a picture made by
/// the GPU when its pixels are read) and the CPU (the same picture made by the processor).
pub fn main() {
  let photo = Image::read(FILE).expect("Failed to load image");
  let (width, height): (u32, u32) = photo.dimensions();
  let (cx, cy) = (width as f32 / 2.0, height as f32 / 2.0);
  let radius = cx.min(cy) * 0.5;
  let touches: Vec<(f32, f32)> = (0..=FRAMES * POINTS_PER_FRAME)
    .map(|i| {
      let t = i as f32 / (FRAMES * POINTS_PER_FRAME) as f32 * std::f32::consts::TAU * 1.03;
      let wobble = 1.0 + 0.05 * (t * 3.0).sin();
      (cx + radius * wobble * t.cos(), cy + radius * wobble * t.sin())
    })
    .collect();
  let red = Color::red();
  let brush = Brush::new().with_size((width.min(height) / 60).max(6)).with_color(&red).with_hardness(0.6).with_opacity(0.9);
  println!("photo {width}x{height}, {FRAMES} frames, {POINTS_PER_FRAME} new points each");

  // How live drawing would really run: the photo stays on the GPU and each frame sends only the line's new points. Nothing is
  // copied back until the end, apart from one pixel each frame to wait for the GPU to finish, so the time is the drawing itself.
  {
    use abra::abra_core::image::recipe::{blended, gpu_run};
    let context = gpu::GpuContext::new_default_blocking().expect("no GPU");
    let mut renderer = gpu::LiveRenderer::new(context);
    let mut times = Vec::new();
    for frame in 1..=FRAMES {
      let points = &touches[..frame * POINTS_PER_FRAME];
      let started = Instant::now();
      let overlay = stroke_overlay_deferred(width, height, points, &brush);
      let (root, passes) = gpu_run(&blended(&photo, &overlay, BlendMode::Normal, 1.0)).expect("the stroke runs on the GPU");
      if frame == 1 {
        renderer.set_source(width, height, root.rgba()).expect("upload");
      }
      renderer.render(&passes).expect("render");
      renderer.wait_frame().ok();
      times.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let rest = &times[1..];
    let mean = rest.iter().sum::<f64>() / rest.len() as f64;
    let worst = rest.iter().cloned().fold(0.0, f64::max);
    println!("GPU resident (live): first frame {:.1} ms, then {mean:.1} ms on average (worst {worst:.1} ms)", times[0]);
    println!("  with a short line (frames 2-6): {:.1} ms, with the whole loop (last 10): {:.1} ms", rest[..5].iter().sum::<f64>() / 5.0, rest[rest.len() - 10..].iter().sum::<f64>() / 10.0);
  }

  for (name, on_gpu) in [("GPU one-shot", true), ("CPU", false)] {
    let mut times = Vec::new();
    for frame in 1..=FRAMES {
      let points = &touches[..frame * POINTS_PER_FRAME];
      let started = Instant::now();
      let overlay = if on_gpu {
        stroke_overlay_deferred(width, height, points, &brush)
      } else {
        stroke_overlay(width, height, points, &brush)
      };
      let shown = abra::abra_core::image::recipe::blended(&photo, &overlay, BlendMode::Normal, 1.0);
      let pixels = shown.rgba().len();
      times.push(started.elapsed().as_secs_f64() * 1000.0);
      if frame == FRAMES {
        shown.write(format!("assets/out/live-stroke-{}.png", if on_gpu { "gpu" } else { "cpu" }), None).expect("Failed to save image");
      }
      assert!(pixels > 0);
    }
    // The first frame includes starting the GPU, so it is reported apart.
    let first = times[0];
    let rest = &times[1..];
    let mean = rest.iter().sum::<f64>() / rest.len() as f64;
    let worst = rest.iter().cloned().fold(0.0, f64::max);
    println!("{name}: first frame {first:.1} ms, then {mean:.1} ms on average (worst {worst:.1} ms)");
  }
}
