use crate::Brush;
use abra_core::image::gpu::GpuPass;
use abra_core::image::recipe::{Generator, generated};
use abra_core::{Channels, Fill, Image};
use rayon::prelude::*;
use std::sync::Arc;

/// Most points one stroke is drawn from. A line with more is thinned evenly (keeping both ends) before it is drawn, so a long
/// stroke still fits in the shader's uniform buffer and costs a bounded amount per pixel.
pub const MAX_STROKE_POINTS: usize = 256;

/// What a brush looks like as a line, in the terms a shader draws it: how far from the line it reaches, how wide its fade is and its
/// color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeLook {
  /// Half the brush's size, in pixels.
  pub radius: f32,
  /// How wide the fade at the edge is, in pixels: the part of the radius that the brush's softness covers, and at least one so the
  /// edge is never jagged.
  pub edge: f32,
  /// The color, straight (not multiplied by its alpha), each from 0 to 1, the last being the color's alpha times the brush's
  /// opacity. A brush that is not a solid color is black.
  pub color: [f32; 4],
}

impl StrokeLook {
  /// How `p_brush` draws a line.
  pub fn of(p_brush: &Brush) -> StrokeLook {
    let radius = p_brush.size() as f32 / 2.0;
    let color = match p_brush.color() {
      Fill::Solid(color) => color.rgba(),
      _ => (0, 0, 0, 255),
    };
    StrokeLook {
      radius,
      edge: ((1.0 - p_brush.hardness()) * radius).max(1.0),
      color: [color.0 as f32 / 255.0, color.1 as f32 / 255.0, color.2 as f32 / 255.0, color.3 as f32 / 255.0 * p_brush.opacity()],
    }
  }
}

/// A line through points, drawn with a brush's size, softness, opacity and color. Made when it is needed: on the CPU by
/// [`render`](Generator::render), or by a shader.
#[derive(Debug)]
struct StrokeOverlay {
  points: Vec<(f32, f32)>,
  radius: f32,
  edge: f32,
  /// The brush color, straight, each from 0 to 1, the last being its alpha times the brush's opacity.
  color: [f32; 4],
}

impl StrokeOverlay {
  fn new(p_points: &[(f32, f32)], p_brush: &Brush) -> StrokeOverlay {
    let points = if p_points.len() <= MAX_STROKE_POINTS {
      p_points.to_vec()
    } else {
      (0..MAX_STROKE_POINTS).map(|i| p_points[i * (p_points.len() - 1) / (MAX_STROKE_POINTS - 1)]).collect()
    };
    let look = StrokeLook::of(p_brush);
    StrokeOverlay { points, radius: look.radius, edge: look.edge, color: look.color }
  }

  /// How far `(x, y)` is from the line.
  fn distance(&self, p_x: f32, p_y: f32) -> f32 {
    let at = |point: &(f32, f32)| ((p_x - point.0).powi(2) + (p_y - point.1).powi(2)).sqrt();
    let mut nearest = at(&self.points[0]);
    for pair in self.points.windows(2) {
      let (a, b) = (pair[0], pair[1]);
      let (abx, aby) = (b.0 - a.0, b.1 - a.1);
      let length_squared = abx * abx + aby * aby;
      let along = if length_squared > 0.0 { (((p_x - a.0) * abx + (p_y - a.1) * aby) / length_squared).clamp(0.0, 1.0) } else { 0.0 };
      nearest = nearest.min(((p_x - (a.0 + abx * along)).powi(2) + (p_y - (a.1 + aby * along)).powi(2)).sqrt());
    }
    nearest
  }
}

impl Generator for StrokeOverlay {
  fn render(&self, p_width: u32, p_height: u32) -> Image {
    let width = p_width as usize;
    let mut pixels = vec![0u8; width * p_height as usize * 4];
    if self.points.is_empty() || pixels.is_empty() {
      return Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA);
    }
    // Only the box around the line, as wide as the brush and its fade, can be touched; every pixel outside stays transparent.
    let reach = self.radius + self.edge;
    let left = self.points.iter().map(|point| point.0).fold(f32::MAX, f32::min) - reach;
    let right = self.points.iter().map(|point| point.0).fold(f32::MIN, f32::max) + reach;
    let top = self.points.iter().map(|point| point.1).fold(f32::MAX, f32::min) - reach;
    let bottom = self.points.iter().map(|point| point.1).fold(f32::MIN, f32::max) + reach;
    let [red, green, blue, _] = self.color.map(|channel| (channel * 255.0 + 0.5) as u8);
    pixels.par_chunks_exact_mut(width * 4).enumerate().for_each(|(y, row)| {
      let center_y = y as f32 + 0.5;
      if center_y < top || center_y > bottom {
        return;
      }
      for (x, pixel) in row.chunks_exact_mut(4).enumerate() {
        let center_x = x as f32 + 0.5;
        if center_x < left || center_x > right {
          continue;
        }
        let coverage = ((self.radius - self.distance(center_x, center_y)) / self.edge + 0.5).clamp(0.0, 1.0) * self.color[3];
        if coverage > 0.0 {
          pixel.copy_from_slice(&[red, green, blue, (coverage * 255.0 + 0.5) as u8]);
        }
      }
    });
    Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA)
  }

  fn passes(&self, _p_width: u32, _p_height: u32) -> Option<Vec<GpuPass>> {
    let mut uniforms: Vec<u8> = Vec::with_capacity(32 + MAX_STROKE_POINTS * 16);
    uniforms.extend(self.color.into_iter().flat_map(f32::to_le_bytes));
    uniforms.extend((self.points.len() as u32).to_le_bytes());
    uniforms.extend([self.radius, self.edge, 0.0].into_iter().flat_map(f32::to_le_bytes));
    for index in 0..MAX_STROKE_POINTS {
      let (x, y) = self.points.get(index).copied().unwrap_or((0.0, 0.0));
      uniforms.extend([x, y, 0.0, 0.0].into_iter().flat_map(f32::to_le_bytes));
    }
    Some(vec![GpuPass::new(include_str!("./live_stroke.wgsl"), uniforms).with_window()])
  }
}

/// An image of `p_width` x `p_height` pixels holding a line drawn through `p_points` (in pixels of the image) with `p_brush`'s
/// size, hardness, opacity and solid color. Pixels away from the line are transparent, so the image is made to be blended over a
/// photo; a brush that is not a solid color draws in black. A line of more than [`MAX_STROKE_POINTS`] points is thinned evenly.
///
/// The pixels are made on the CPU now. [`stroke_overlay_deferred`] leaves them to the GPU when there is one.
pub fn stroke_overlay(p_width: u32, p_height: u32, p_points: &[(f32, f32)], p_brush: &Brush) -> Image {
  StrokeOverlay::new(p_points, p_brush).render(p_width, p_height)
}

/// [`stroke_overlay`] as an image that is made by the GPU when its pixels are first read (see `abra_core::image::recipe`). A
/// line that gains a point is a new image that costs one shader run, so it can follow a finger frame after frame. Without a GPU
/// it is made now, with the same pixels either way.
pub fn stroke_overlay_deferred(p_width: u32, p_height: u32, p_points: &[(f32, f32)], p_brush: &Brush) -> Image {
  generated(p_width, p_height, Arc::new(StrokeOverlay::new(p_points, p_brush)))
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Color;

  fn alpha_at(p_image: &Image, p_x: usize, p_y: usize) -> u8 {
    p_image.rgba()[(p_y * p_image.dimensions::<u32>().0 as usize + p_x) * 4 + 3]
  }

  #[test]
  fn the_line_is_solid_on_it_soft_at_its_edge_and_clear_away_from_it() {
    let red = Color::red();
    let brush = Brush::new().with_size(9).with_color(&red).with_hardness(1.0);
    let image = stroke_overlay(40, 20, &[(5.0, 10.0), (35.0, 10.0)], &brush);
    assert_eq!(alpha_at(&image, 20, 10), 255);
    assert_eq!(alpha_at(&image, 20, 2), 0);
    // The brush is 4.5 pixels from the line to its edge, which is exactly where the middle of the pixel in row 14 is.
    let edge = alpha_at(&image, 20, 14);
    assert!(edge > 0 && edge < 255, "{edge}");
    assert_eq!(alpha_at(&stroke_overlay(8, 8, &[], &brush), 4, 4), 0);
  }

  #[test]
  fn a_long_line_is_thinned_to_the_most_points_the_shader_takes() {
    let points: Vec<(f32, f32)> = (0..1000).map(|i| (i as f32 / 10.0, 5.0)).collect();
    let overlay = StrokeOverlay::new(&points, &Brush::new());
    assert_eq!(overlay.points.len(), MAX_STROKE_POINTS);
    assert_eq!(overlay.points[0], points[0]);
    assert_eq!(overlay.points[MAX_STROKE_POINTS - 1], points[999]);
  }

  /// The shader on a real GPU gives the CPU's pixels, for a hard and a soft brush. The real provider is wrapped in one that counts,
  /// so the test fails if the work quietly fell back to the CPU (which would match perfectly).
  #[test]
  fn the_gpu_gives_the_cpu_s_pixels() -> anyhow::Result<()> {
    use abra_core::BlendMode;
    use abra_core::image::gpu::{GPU_PROVIDER_LOCK, GpuProvider, get_gpu_provider, register_gpu_provider};
    use abra_core::image::recipe::blended;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static REAL: OnceLock<GpuProvider> = OnceLock::new();
    static RUNS: AtomicUsize = AtomicUsize::new(0);

    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    gpu::GpuContext::new_default_blocking()?;
    gpu::register();
    let real = *REAL.get_or_init(|| get_gpu_provider().expect("the gpu crate registers a provider"));
    register_gpu_provider(GpuProvider {
      process: |effects, width, height, pixels| {
        RUNS.fetch_add(1, Ordering::SeqCst);
        (REAL.get().unwrap().process)(effects, width, height, pixels)
      },
      ..real
    });
    RUNS.store(0, Ordering::SeqCst);

    let (width, height) = (97, 61);
    let photo = Image::new_from_pixels(width, height, [90u8, 120, 150, 255].repeat((width * height) as usize), Channels::RGBA);
    let white = Color::white();
    let points = [(8.0, 50.0), (30.0, 12.0), (60.5, 40.0), (90.0, 8.0)];
    for (size, hardness) in [(9, 1.0), (22, 0.0), (14, 0.5)] {
      let brush = Brush::new().with_size(size).with_color(&white).with_hardness(hardness).with_opacity(0.8);
      let expected = blended(&photo, &stroke_overlay(width, height, &points, &brush), BlendMode::Normal, 1.0);
      let made = blended(&photo, &stroke_overlay_deferred(width, height, &points, &brush), BlendMode::Normal, 1.0);
      assert!(made.deferred().is_some(), "with a GPU the result is not made until it is read");
      let worst = made.rgba().iter().zip(expected.rgba()).map(|(a, e)| a.abs_diff(*e)).max().unwrap_or(0);
      assert!(worst <= 2, "size {size} hardness {hardness}: the GPU differs from the CPU by {worst}");
    }
    assert!(RUNS.load(Ordering::SeqCst) >= 3, "the GPU did not run the shader");
    abra_core::image::gpu::clear_gpu_provider();
    Ok(())
  }
}
