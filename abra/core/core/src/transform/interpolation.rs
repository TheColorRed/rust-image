//! Interpolation algorithms for image sampling.
//!
//! This module provides various interpolation methods used for sampling pixels
//! at non-integer coordinates. These are fundamental building blocks for image
//! transformations like resizing, distortions, and warping.

use primitives::Image;
use rayon::prelude::*;

/// Sample a pixel using bilinear interpolation with premultiplied alpha.
///
/// This function samples the image at a fractional (x, y) coordinate by performing
/// a weighted average of the four nearest pixels. Premultiplied alpha is used to
/// avoid dark fringes around transparent edges.
///
/// - `p_image`: The image to sample from.
/// - `p_x`: The x-coordinate (can be fractional).
/// - `p_y`: The y-coordinate (can be fractional).
///
/// Returns `[r, g, b, a]` as u8 values, or `[0, 0, 0, 0]` if out of bounds.
pub fn sample_bilinear(p_image: &Image, p_x: f32, p_y: f32) -> [u8; 4] {
  let (width, height) = p_image.dimensions::<u32>();
  let pixels = p_image.rgba();

  let x0 = p_x.floor() as i32;
  let y0 = p_y.floor() as i32;
  let x1 = x0 + 1;
  let y1 = y0 + 1;

  let fx = p_x - x0 as f32;
  let fy = p_y - y0 as f32;

  // Helper to safely get pixel
  let get_pixel = |px: i32, py: i32| -> [u8; 4] {
    if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
      [0, 0, 0, 0]
    } else {
      let idx = (py as u32 * width + px as u32) as usize;
      if idx * 4 + 3 < pixels.len() {
        [
          pixels[idx * 4],
          pixels[idx * 4 + 1],
          pixels[idx * 4 + 2],
          pixels[idx * 4 + 3],
        ]
      } else {
        [0, 0, 0, 0]
      }
    }
  };

  let p00 = get_pixel(x0, y0);
  let p10 = get_pixel(x1, y0);
  let p01 = get_pixel(x0, y1);
  let p11 = get_pixel(x1, y1);

  // Bilinear interpolation with premultiplied alpha
  let a00 = p00[3] as f32 / 255.0;
  let a10 = p10[3] as f32 / 255.0;
  let a01 = p01[3] as f32 / 255.0;
  let a11 = p11[3] as f32 / 255.0;

  let r00 = p00[0] as f32 * a00;
  let g00 = p00[1] as f32 * a00;
  let b00 = p00[2] as f32 * a00;
  let r10 = p10[0] as f32 * a10;
  let g10 = p10[1] as f32 * a10;
  let b10 = p10[2] as f32 * a10;
  let r01 = p01[0] as f32 * a01;
  let g01 = p01[1] as f32 * a01;
  let b01 = p01[2] as f32 * a01;
  let r11 = p11[0] as f32 * a11;
  let g11 = p11[1] as f32 * a11;
  let b11 = p11[2] as f32 * a11;

  let a0 = a00 * (1.0 - fx) + a10 * fx;
  let a1 = a01 * (1.0 - fx) + a11 * fx;
  let a = a0 * (1.0 - fy) + a1 * fy;

  let r0 = r00 * (1.0 - fx) + r10 * fx;
  let r1 = r01 * (1.0 - fx) + r11 * fx;
  let rp = r0 * (1.0 - fy) + r1 * fy;

  let g0 = g00 * (1.0 - fx) + g10 * fx;
  let g1 = g01 * (1.0 - fx) + g11 * fx;
  let gp = g0 * (1.0 - fy) + g1 * fy;

  let b0 = b00 * (1.0 - fx) + b10 * fx;
  let b1 = b01 * (1.0 - fx) + b11 * fx;
  let bp = b0 * (1.0 - fy) + b1 * fy;

  let result = if a > 0.0 {
    [
      (rp / a).clamp(0.0, 255.0).round() as u8,
      (gp / a).clamp(0.0, 255.0).round() as u8,
      (bp / a).clamp(0.0, 255.0).round() as u8,
      (a * 255.0).clamp(0.0, 255.0).round() as u8,
    ]
  } else {
    [0, 0, 0, 0]
  };

  result
}

/// Sample a pixel using bicubic interpolation with premultiplied alpha.
///
/// This function samples the image at a fractional (x, y) coordinate by performing
/// a weighted average of the sixteen nearest pixels using a cubic kernel. This
/// provides higher quality than bilinear interpolation.
///
/// - `p_image`: The image to sample from.
/// - `p_x`: The x-coordinate (can be fractional).
/// - `p_y`: The y-coordinate (can be fractional).
///
/// Returns `[r, g, b, a]` as u8 values, or `[0, 0, 0, 0]` if out of bounds.
pub fn sample_bicubic(p_image: &Image, p_x: f32, p_y: f32) -> [u8; 4] {
  let (width, height) = p_image.dimensions::<u32>();
  let pixels = p_image.rgba();

  // Cubic interpolation kernel
  let cubic_kernel = |t: f32| -> f32 {
    let t = t.abs();
    if t < 1.0 {
      1.0 - 2.0 * t * t + t * t * t
    } else if t < 2.0 {
      -4.0 + 8.0 * t - 5.0 * t * t + t * t * t
    } else {
      0.0
    }
  };

  let x0 = p_x.floor() as i32;
  let y0 = p_y.floor() as i32;

  let fx = p_x - x0 as f32;
  let fy = p_y - y0 as f32;

  // Helper to safely get pixel
  let get_pixel = |px: i32, py: i32| -> [u8; 4] {
    if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
      [0, 0, 0, 0]
    } else {
      let idx = (py as u32 * width + px as u32) as usize;
      if idx * 4 + 3 < pixels.len() {
        [
          pixels[idx * 4],
          pixels[idx * 4 + 1],
          pixels[idx * 4 + 2],
          pixels[idx * 4 + 3],
        ]
      } else {
        [0, 0, 0, 0]
      }
    }
  };

  // Sample 4x4 neighborhood using premultiplied alpha
  let mut acc_r = 0.0;
  let mut acc_g = 0.0;
  let mut acc_b = 0.0;
  let mut acc_a = 0.0;
  let mut weight_sum = 0.0;

  for dy in -1..=2 {
    for dx in -1..=2 {
      let px = x0 + dx;
      let py = y0 + dy;
      let p = get_pixel(px, py);
      let a = p[3] as f32 / 255.0;
      let w = cubic_kernel(dx as f32 - fx) * cubic_kernel(dy as f32 - fy);
      acc_r += (p[0] as f32 * a) * w;
      acc_g += (p[1] as f32 * a) * w;
      acc_b += (p[2] as f32 * a) * w;
      acc_a += a * w;
      weight_sum += w;
    }
  }

  if weight_sum > 0.0 {
    acc_r /= weight_sum;
    acc_g /= weight_sum;
    acc_b /= weight_sum;
    acc_a /= weight_sum;
  }

  let result = if acc_a > 0.0 {
    [
      (acc_r / acc_a).clamp(0.0, 255.0).round() as u8,
      (acc_g / acc_a).clamp(0.0, 255.0).round() as u8,
      (acc_b / acc_a).clamp(0.0, 255.0).round() as u8,
      (acc_a * 255.0).clamp(0.0, 255.0).round() as u8,
    ]
  } else {
    [0, 0, 0, 0]
  };

  result
}

/// Sample a pixel using Lanczos resampling with premultiplied alpha.
///
/// This function samples the image at a fractional (x, y) coordinate using the
/// Lanczos kernel (a=3). This provides the highest quality interpolation but
/// is more computationally expensive.
///
/// - `p_image`: The image to sample from.
/// - `p_x`: The x-coordinate (can be fractional).
/// - `p_y`: The y-coordinate (can be fractional).
///
/// Returns `[r, g, b, a]` as u8 values, or `[0, 0, 0, 0]` if out of bounds.
pub fn sample_lanczos(p_image: &Image, p_x: f32, p_y: f32) -> [u8; 4] {
  let (width, height) = p_image.dimensions::<u32>();
  let pixels = p_image.rgba();

  const LANCZOS_SIZE: i32 = 3;

  // Lanczos kernel with a=3
  let lanczos_kernel = |t: f32| -> f32 {
    let t = t.abs();
    if t == 0.0 {
      1.0
    } else if t < LANCZOS_SIZE as f32 {
      let pi_t = std::f32::consts::PI * t;
      let pi_t_a = std::f32::consts::PI * t / LANCZOS_SIZE as f32;
      (pi_t.sin() / pi_t) * (pi_t_a.sin() / pi_t_a)
    } else {
      0.0
    }
  };

  let x0 = p_x.floor() as i32;
  let y0 = p_y.floor() as i32;

  let fx = p_x - x0 as f32;
  let fy = p_y - y0 as f32;

  // Helper to safely get pixel
  let get_pixel = |px: i32, py: i32| -> [u8; 4] {
    if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
      [0, 0, 0, 0]
    } else {
      let idx = (py as u32 * width + px as u32) as usize;
      if idx * 4 + 3 < pixels.len() {
        [
          pixels[idx * 4],
          pixels[idx * 4 + 1],
          pixels[idx * 4 + 2],
          pixels[idx * 4 + 3],
        ]
      } else {
        [0, 0, 0, 0]
      }
    }
  };

  // Sample neighborhood with Lanczos kernel using premultiplied alpha
  let mut acc_r = 0.0;
  let mut acc_g = 0.0;
  let mut acc_b = 0.0;
  let mut acc_a = 0.0;
  let mut weight_sum = 0.0;

  for dy in -LANCZOS_SIZE + 1..=LANCZOS_SIZE {
    for dx in -LANCZOS_SIZE + 1..=LANCZOS_SIZE {
      let px = x0 + dx;
      let py = y0 + dy;
      let p = get_pixel(px, py);
      let a = p[3] as f32 / 255.0;
      let w = lanczos_kernel(dx as f32 - fx) * lanczos_kernel(dy as f32 - fy);
      acc_r += (p[0] as f32 * a) * w;
      acc_g += (p[1] as f32 * a) * w;
      acc_b += (p[2] as f32 * a) * w;
      acc_a += a * w;
      weight_sum += w;
    }
  }

  if weight_sum > 0.0 {
    acc_r /= weight_sum;
    acc_g /= weight_sum;
    acc_b /= weight_sum;
    acc_a /= weight_sum;
  }

  let result = if acc_a > 0.0 {
    [
      (acc_r / acc_a).clamp(0.0, 255.0).round() as u8,
      (acc_g / acc_a).clamp(0.0, 255.0).round() as u8,
      (acc_b / acc_a).clamp(0.0, 255.0).round() as u8,
      (acc_a * 255.0).clamp(0.0, 255.0).round() as u8,
    ]
  } else {
    [0, 0, 0, 0]
  };

  result
}

/// Sample a pixel using nearest neighbor (no interpolation).
///
/// This function returns the pixel at the nearest integer coordinate.
/// It's the fastest but lowest quality sampling method.
///
/// - `p_image`: The image to sample from.
/// - `p_x`: The x-coordinate (will be rounded).
/// - `p_y`: The y-coordinate (will be rounded).
///
/// Returns `[r, g, b, a]` as u8 values, or `[0, 0, 0, 0]` if out of bounds.
pub fn sample_nearest(p_image: &Image, p_x: f32, p_y: f32) -> [u8; 4] {
  let (width, height) = p_image.dimensions::<u32>();
  let pixels = p_image.rgba();

  let x = p_x.round() as i32;
  let y = p_y.round() as i32;

  if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
    [0, 0, 0, 0]
  } else {
    let idx = (y as u32 * width + x as u32) as usize;
    if idx * 4 + 3 < pixels.len() {
      [
        pixels[idx * 4],
        pixels[idx * 4 + 1],
        pixels[idx * 4 + 2],
        pixels[idx * 4 + 3],
      ]
    } else {
      [0, 0, 0, 0]
    }
  }
}

/// Resample an entire image using bilinear interpolation.
///
/// This is a batch operation that samples all pixels for a new image size using
/// bilinear interpolation. More efficient than calling `sample_bilinear` per pixel.
///
/// - `p_source`: The source image to sample from.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
///
/// Returns a vector of RGBA pixel data for the new image.
pub fn resample_bilinear(p_source: &Image, p_width: u32, p_height: u32) -> Vec<u8> {
  let (old_width, old_height) = p_source.dimensions::<u32>();
  let buffer_size = (p_width as u64)
    .checked_mul(p_height as u64)
    .and_then(|size| size.checked_mul(4))
    .expect("Image dimensions too large") as usize;
  let mut new_pixels = vec![0; buffer_size];

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % p_width;
    let y = i as u32 / p_width;

    let src_x = (x as f32 + 0.5) * (old_width as f32 / p_width as f32) - 0.5;
    let src_y = (y as f32 + 0.5) * (old_height as f32 / p_height as f32) - 0.5;

    let pixel = sample_bilinear(p_source, src_x, src_y);
    chunk.copy_from_slice(&pixel);
  });

  new_pixels
}

/// Resample an entire image using bicubic interpolation.
///
/// This is a batch operation that samples all pixels for a new image size using
/// bicubic interpolation. More efficient than calling `sample_bicubic` per pixel.
///
/// - `p_source`: The source image to sample from.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
///
/// Returns a vector of RGBA pixel data for the new image.
pub fn resample_bicubic(p_source: &Image, p_width: u32, p_height: u32) -> Vec<u8> {
  let (old_width, old_height) = p_source.dimensions::<u32>();
  let buffer_size = (p_width as u64)
    .checked_mul(p_height as u64)
    .and_then(|size| size.checked_mul(4))
    .expect("Image dimensions too large") as usize;
  let mut new_pixels = vec![0; buffer_size];

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % p_width;
    let y = i as u32 / p_width;

    let src_x = (x as f32 + 0.5) * (old_width as f32 / p_width as f32) - 0.5;
    let src_y = (y as f32 + 0.5) * (old_height as f32 / p_height as f32) - 0.5;

    let pixel = sample_bicubic(p_source, src_x, src_y);
    chunk.copy_from_slice(&pixel);
  });

  new_pixels
}

/// Resample an entire image using Lanczos resampling.
///
/// This is a batch operation that samples all pixels for a new image size using
/// Lanczos resampling. More efficient than calling `sample_lanczos` per pixel.
///
/// - `p_source`: The source image to sample from.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
///
/// Returns a vector of RGBA pixel data for the new image.
pub fn resample_lanczos(p_source: &Image, p_width: u32, p_height: u32) -> Vec<u8> {
  let (old_width, old_height) = p_source.dimensions::<u32>();
  let buffer_size = (p_width as u64)
    .checked_mul(p_height as u64)
    .and_then(|size| size.checked_mul(4))
    .expect("Image dimensions too large") as usize;
  let mut new_pixels = vec![0; buffer_size];

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % p_width;
    let y = i as u32 / p_width;

    let src_x = (x as f32 + 0.5) * (old_width as f32 / p_width as f32) - 0.5;
    let src_y = (y as f32 + 0.5) * (old_height as f32 / p_height as f32) - 0.5;

    let pixel = sample_lanczos(p_source, src_x, src_y);
    chunk.copy_from_slice(&pixel);
  });

  new_pixels
}

/// Resample an entire image using nearest neighbor (no interpolation).
///
/// This is a batch operation that samples all pixels for a new image size using
/// nearest neighbor. This is the fastest but lowest quality resampling method.
///
/// - `p_source`: The source image to sample from.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
///
/// Returns a vector of RGBA pixel data for the new image.
pub fn resample_nearest(p_source: &Image, p_width: u32, p_height: u32) -> Vec<u8> {
  let (old_width, old_height) = p_source.dimensions::<u32>();
  let old_pixels = p_source.rgba();
  let buffer_size = (p_width as u64)
    .checked_mul(p_height as u64)
    .and_then(|size| size.checked_mul(4))
    .expect("Image dimensions too large") as usize;
  let mut new_pixels = vec![0; buffer_size];

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % p_width;
    let y = i as u32 / p_width;

    let old_x = ((x as f32 / p_width as f32) * (old_width as f32 - 1.0))
      .max(0.0)
      .min(old_width as f32 - 1.0) as u32;
    let old_y = ((y as f32 / p_height as f32) * (old_height as f32 - 1.0))
      .max(0.0)
      .min(old_height as f32 - 1.0) as u32;
    let old_index = (old_y * old_width + old_x) as usize;

    if old_index * 4 + 3 < old_pixels.len() {
      chunk.copy_from_slice(&old_pixels[old_index * 4..old_index * 4 + 4]);
    }
  });

  new_pixels
}
