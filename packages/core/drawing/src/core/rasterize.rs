//! Unified, sample-based rasterization engine used for area and stroke fills.
//!
//! The `Rasterizer` implements a simple, thread-parallel rasterization
//! algorithm that walks device pixels within an optional coverage
//! bounding box, samples sub-pixel positions using a `SampleGrid`, asks a
//! `Shader` to compute colors for each covered sample, and composites the
//! averaged color back onto the image using a `Compositor`.
//!
//! Design notes
//! - The rasterizer relies on `CoverageMask` to limit work. Masks can be
//!   arbitrary geometry: polygons, brushes, or full-screen coverage.
//! - `Shader` implementations return RGBA per-sample at non-integer
//!   coordinates—this allows gradients, textures, and brush falloff.
//! - The `Compositor` controls how the averaged source color is blended
//!   with the destination pixel (e.g., source-over, overwrite).
//! - Rows are processed in parallel using `rayon::par_chunks_mut` for
//!   good multi-core performance; therefore, `CoverageMask`, `Shader`,
//!   and `Compositor` types should be `Sync` (they are required to be by
//!   the trait signatures).
//!
//! Example
//! ```ignore
//! use abra::draw::core::{Rasterizer, FullCoverage, SampleGrid, SourceOverCompositor, SolidShader};
//! let grid = SampleGrid::from_aa_level(2);
//! // Coverage and shader are usually created by calling into the public API.
//! let coverage = FullCoverage;
//! let shader = SolidShader::new(abra::color::Color::from_rgba(255, 0, 0, 255));
//! let comp = SourceOverCompositor;
//! let rasterizer = Rasterizer::new(&coverage, &shader, &comp, grid);
//! rasterizer.rasterize(&mut image);
//! ```

use rayon::prelude::*;

use core::Image;

use crate::{Compositor, CoverageMask, SampleGrid, Shader};

/// Unified rasterizer for area filling with configurable shader and coverage.
///
/// The `Rasterizer` drives the sample-based rasterization loop. It uses
/// a `CoverageMask` to test which sub-pixel samples belong to the fill,
/// a `Shader` to compute the sample color for a given sample location,
/// a `Compositor` to blend the sample with destination pixels and a
/// `SampleGrid` to enumerate sub-pixel positions for anti-aliasing.
///
/// The implementation uses `rayon` in the inner loop to parallelize row
/// processing for improved performance on multi-core systems.
///
/// Example
/// ```ignore
/// let rasterizer = Rasterizer::new(&coverage, &shader, &compositor, SampleGrid::from_aa_level(4));
/// rasterizer.rasterize(&mut image);
/// ```
pub struct Rasterizer<'a> {
  coverage: &'a dyn CoverageMask,
  shader: &'a dyn Shader,
  compositor: &'a dyn Compositor,
  sample_grid: SampleGrid,
}

impl<'a> Rasterizer<'a> {
  /// Creates a new rasterizer with the specified components.
  ///
  /// Parameters
  /// - `p_coverage`: implementation of `CoverageMask` that selects samples.
  /// - `p_shader`: shading implementation for computing RGBA at sample points.
  /// - `p_compositor`: compositing rule used to merge source sample with destination.
  /// - `p_sample_grid`: supersampling grid controlling sub-pixel sampling density.
  ///
  /// Example
  /// ```ignore
  /// let r = Rasterizer::new(&coverage, &shader, &compositor, SampleGrid::from_aa_level(2));
  /// ```
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

  /// Rasterizes the configured coverage and writes into `p_image`.
  ///
  /// The rasterizer will only iterate pixels within the coverage bounds
  /// (if provided) and will sample sub-pixel locations according to the
  /// `SampleGrid`. For each sample inside the coverage mask, it will use
  /// the `Shader` to obtain a color and blend the averaged color with the
  /// existing destination pixel using the `Compositor`.
  ///
  /// Parameters
  /// - `p_image`: the target image to write the rasterized result into.
  ///
  /// Example
  /// ```ignore
  /// rasterizer.rasterize(&mut image);
  /// ```
  pub fn rasterize(&self, p_image: &mut Image) {
    let start = std::time::Instant::now();
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
    // DebugDrawing::Rasterization(start.elapsed()).log();
  }
}
