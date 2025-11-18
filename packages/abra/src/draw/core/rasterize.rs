//! Unified rasterization engine.

use rayon::prelude::*;

use crate::Image;

use super::{Compositor, CoverageMask, SampleGrid, Shader};

/// Unified rasterizer for area filling with configurable shader and coverage.
pub(crate) struct Rasterizer<'a> {
  coverage: &'a dyn CoverageMask,
  shader: &'a dyn Shader,
  compositor: &'a dyn Compositor,
  sample_grid: SampleGrid,
}

impl<'a> Rasterizer<'a> {
  /// Creates a new rasterizer with the specified components.
  pub fn new(
    p_coverage: &'a dyn CoverageMask, p_shader: &'a dyn Shader, p_compositor: &'a dyn Compositor,
    p_sample_grid: SampleGrid,
  ) -> Self {
    Rasterizer {
      coverage: p_coverage,
      shader: p_shader,
      compositor: p_compositor,
      sample_grid: p_sample_grid,
    }
  }

  /// Rasterizes to the provided image.
  pub fn rasterize(&self, p_image: &mut Image) {
    let mut pixels = p_image.rgba();
    let (width, height) = p_image.dimensions::<u32>();
    let total_samples = self.sample_grid.total_samples() as f32;
    // Determine bounds from coverage (if available) so we only iterate pixels that may be affected.
    let (min_x_f, min_y_f, max_x_f, max_y_f) = match self.coverage.bounds() {
      Some((min_x, min_y, max_x, max_y)) => (min_x, min_y, max_x, max_y),
      None => (0.0, 0.0, width as f32, height as f32),
    };
    let min_x = min_x_f.floor().max(0.0) as u32;
    let min_y = min_y_f.floor().max(0.0) as u32;
    let max_x = max_x_f.ceil().min(width as f32 - 1.0) as u32;
    let max_y = max_y_f.ceil().min(height as f32 - 1.0) as u32;

    let width_usize = width as usize;
    // Iterate by row in parallel. Each row is a slice of width_usize*4 bytes.
    pixels
      .par_chunks_mut(width_usize * 4)
      .enumerate()
      .for_each(|(row_idx, row)| {
        let y = row_idx as u32;
        if y < min_y || y > max_y {
          return;
        }

        for x in min_x..=max_x {
          let px_idx = x as usize * 4;
          let pixel = &mut row[px_idx..px_idx + 4];

          // Accumulate coverage and color samples
          let mut coverage_count = 0.0;
          let mut r_sum = 0.0;
          let mut g_sum = 0.0;
          let mut b_sum = 0.0;
          let mut a_sum = 0.0;

          for (sub_x, sub_y) in self.sample_grid.samples(x, y) {
            if self.coverage.contains(sub_x, sub_y) {
              coverage_count += 1.0;
              let (r, g, b, a) = self.shader.shade(sub_x, sub_y);
              r_sum += r as f32;
              g_sum += g as f32;
              b_sum += b as f32;
              a_sum += a as f32;
            }
          }

          if coverage_count > 0.0 {
            // Average the sampled colors
            let avg_r = (r_sum / coverage_count) as u8;
            let avg_g = (g_sum / coverage_count) as u8;
            let avg_b = (b_sum / coverage_count) as u8;
            let avg_a = (a_sum / coverage_count) as u8;

            let coverage = coverage_count / total_samples;

            // Composite with existing pixel
            let (out_r, out_g, out_b, out_a) = self
              .compositor
              .composite(avg_r, avg_g, avg_b, avg_a, coverage, pixel[0], pixel[1], pixel[2], pixel[3]);

            pixel[0] = out_r;
            pixel[1] = out_g;
            pixel[2] = out_b;
            pixel[3] = out_a;
          }
        }
      });

    p_image.set_rgba(pixels);
  }
}
