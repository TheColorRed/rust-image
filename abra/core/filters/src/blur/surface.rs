use crate::common::*;

use abra_core::Size;
use abra_core::image::gpu::{GpuOp, GpuPass, GpuProcessor};
use abra_core::transform::{ResizeTarget, Transform, TransformAlgorithm};
use abra_core::{Channel, Histogram};

/// Past this radius the CPU blurs a half-size copy of the image and scales it back up, for speed.
const CPU_SUBSAMPLE_RADIUS: u32 = 50;

fn apply_surface_blur(p_image: &mut Image, p_radius: u32, p_threshold: u8, p_step: usize) {
  if p_radius == 0 {
    return;
  }

  // For large radii, use sub-sampling approximation for speed if dimensions allow
  if p_radius > CPU_SUBSAMPLE_RADIUS {
    let (w, h) = p_image.dimensions::<u32>();
    let factor = 2;
    if w >= factor && h >= factor {
      let new_w = w / factor;
      let new_h = h / factor;

      // Downsample using bilinear for speed
      let mut downsampled = p_image.clone();
      downsampled.resize(ResizeTarget::Exact(Size::new(new_w, new_h)), TransformAlgorithm::Bilinear);

      // Apply blur at reduced resolution
      apply_surface_blur(&mut downsampled, p_radius / factor, p_threshold, p_step);

      // Upsample using bicubic for better quality
      downsampled.resize(ResizeTarget::Exact(Size::new(w, h)), TransformAlgorithm::Bicubic);
      *p_image = downsampled;
      return;
    }
    // Fall back to direct computation if dimensions too small
  }

  let (width, height) = p_image.dimensions::<u32>();
  if width == 0 || height == 0 {
    return;
  }

  let src = p_image.rgba();
  let (w, h) = (width as usize, height as usize);
  let mut out = vec![0u8; w * h * 4];

  let r = p_radius as i32;
  let step = p_step.max(1) as i32;

  // Process rows in parallel for better cache locality
  out.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
    // Allocate histogram once per row - reuse across pixels
    let mut hist = Histogram::new();

    for x in 0..w {
      let center_idx = (y * w + x) * 4;
      let cr = src[center_idx];
      let cg = src[center_idx + 1];
      let cb = src[center_idx + 2];
      let ca = src[center_idx + 3];

      // Clear and get mutable access to histogram arrays
      hist.clear();

      // Build histogram of neighbors
      for dy in -r..=r {
        let ny = ((y as i32 + dy * step).max(0).min(h as i32 - 1)) as usize;
        let ny_offset = ny * w * 4;

        for dx in -r..=r {
          let nx = ((x as i32 + dx * step).max(0).min(w as i32 - 1)) as usize;
          let n_idx = ny_offset + nx * 4;

          // Use unsafe for faster access - bounds already validated
          let (nr, ng, nb) =
            unsafe { (*src.get_unchecked(n_idx), *src.get_unchecked(n_idx + 1), *src.get_unchecked(n_idx + 2)) };

          hist.add_rgb(nr, ng, nb);
        }
      }

      // Compute weighted average within threshold using histogram
      let out_r = hist.channel(Channel::R).weighted_average(cr, p_threshold);
      let out_g = hist.channel(Channel::G).weighted_average(cg, p_threshold);
      let out_b = hist.channel(Channel::B).weighted_average(cb, p_threshold);

      let dst = &mut row[x * 4..(x + 1) * 4];
      dst[0] = out_r;
      dst[1] = out_g;
      dst[2] = out_b;
      dst[3] = ca;
    }
  });

  p_image.set_rgba(out);
}
/// Applies a surface blur to an image.
/// - `p_image`: The image to be blurred.
/// - `p_radius`: The radius of the surface blur.
/// - `p_threshold`: The threshold for the surface blur.
/// - `p_apply_options`: Additional options for applying the blur.
#[derive(Clone)]
pub struct SurfaceBlur {
  radius: u32,
  threshold: u8,
  step: usize,
  options: Options,
}

impl SurfaceBlur {
  /// Spaces the pixels the blur averages over `p_step` pixels apart, so the same number of pixels reaches `p_step`
  /// times as far. The cost of the blur depends on how many pixels it averages, so this blurs a large photo as widely as
  /// a small one would be, for the same cost, and with a plain blur's smooth result on flat areas. Defaults to `1`, which
  /// averages every pixel in the window.
  pub fn with_step(mut self, p_step: usize) -> Self {
    self.step = p_step.max(1);
    self
  }
}

impl Effect for SurfaceBlur {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    (self.radius as usize * self.step) as i32
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_surface_blur(p_image, self.radius, self.threshold, self.step);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    // Past `CPU_SUBSAMPLE_RADIUS` the CPU blurs a half-size copy and scales it back up, for speed, which the shader does
    // not do: it would look at (2 * radius + 1) squared pixels for each pixel. So a big radius stays on the CPU.
    (self.radius <= CPU_SUBSAMPLE_RADIUS).then_some(self)
  }
}

impl GpuProcessor for SurfaceBlur {
  /// One pass of `surface.wgsl`. Its uniform is three `u32`s: the radius, the threshold, then the step.
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    let mut uniforms = Vec::with_capacity(12);
    uniforms.extend_from_slice(&self.radius.to_le_bytes());
    uniforms.extend_from_slice(&(self.threshold as u32).to_le_bytes());
    uniforms.extend_from_slice(&(self.step as u32).to_le_bytes());
    vec![GpuPass::new(include_str!("./surface.wgsl"), uniforms)]
  }
}

pub fn surface_blur(p_radius: u32, p_threshold: u8) -> SurfaceBlur {
  SurfaceBlur {
    radius: p_radius,
    threshold: p_threshold,
    step: 1,
    options: None,
  }
}
