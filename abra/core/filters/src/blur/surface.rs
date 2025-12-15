use crate::common::*;

use abra_core::color::Histogram;
use abra_core::transform::{Resize, TransformAlgorithm};

fn apply_surface_blur(image: &mut Image, radius: u32, threshold: u8) {
  if radius == 0 {
    return;
  }

  // For large radii, use sub-sampling approximation for speed if dimensions allow
  if radius > 50 {
    let (w, h) = image.dimensions::<u32>();
    let factor = 2;
    if w >= factor && h >= factor {
      let new_w = w / factor;
      let new_h = h / factor;

      // Downsample using bilinear for speed
      let mut downsampled = image.clone();
      downsampled.resize(new_w, new_h, TransformAlgorithm::Bilinear);

      // Apply blur at reduced resolution
      apply_surface_blur(&mut downsampled, radius / factor, threshold);

      // Upsample using bicubic for better quality
      downsampled.resize(w, h, TransformAlgorithm::Bicubic);
      *image = downsampled;
      return;
    }
    // Fall back to direct computation if dimensions too small
  }

  let (width, height) = image.dimensions::<u32>();
  if width == 0 || height == 0 {
    return;
  }

  let src = image.rgba();
  let (w, h) = (width as usize, height as usize);
  let mut out = vec![0u8; w * h * 4];

  let r = radius as i32;

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
      let (r_hist, g_hist, b_hist) = hist.rgb_mut();

      // Build histogram of neighbors
      for dy in -r..=r {
        let ny = ((y as i32 + dy).max(0).min(h as i32 - 1)) as usize;
        let ny_offset = ny * w * 4;

        for dx in -r..=r {
          let nx = ((x as i32 + dx).max(0).min(w as i32 - 1)) as usize;
          let n_idx = ny_offset + nx * 4;

          // Use unsafe for faster access - bounds already validated
          let (nr, ng, nb) =
            unsafe { (*src.get_unchecked(n_idx), *src.get_unchecked(n_idx + 1), *src.get_unchecked(n_idx + 2)) };

          r_hist[nr as usize] += 1;
          g_hist[ng as usize] += 1;
          b_hist[nb as usize] += 1;
        }
      }

      // Compute weighted average within threshold using histogram
      let out_r = hist.red_weighted_average(cr, threshold);
      let out_g = hist.green_weighted_average(cg, threshold);
      let out_b = hist.blue_weighted_average(cb, threshold);

      let dst = &mut row[x * 4..(x + 1) * 4];
      dst[0] = out_r;
      dst[1] = out_g;
      dst[2] = out_b;
      dst[3] = ca;
    }
  });

  image.set_rgba_owned(out);
}
/// Applies a surface blur to an image.
/// - `p_image`: The image to be blurred.
/// - `p_radius`: The radius of the surface blur.
/// - `p_threshold`: The threshold for the surface blur.
/// - `p_apply_options`: Additional options for applying the blur.
pub fn surface_blur<'a>(
  p_image: impl Into<ImageRef<'a>>, p_radius: u32, p_threshold: u8, p_apply_options: impl Into<Options>,
) {
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  apply_filter!(apply_surface_blur, image, p_apply_options, p_radius as i32, p_radius, p_threshold);
}
