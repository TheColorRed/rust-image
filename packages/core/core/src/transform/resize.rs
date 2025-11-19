use std::time::Instant;

use crate::image::Image;
use crate::transform::TransformAlgorithm;
// use crate::utils::debug::DebugTransform;
use rayon::prelude::*;

/// Trait for resizing functionality.
pub trait Resize {
  /// Resize the image to the given dimensions.
  /// This does not maintain the aspect ratio unless the given dimensions match the original aspect ratio.
  /// - `p_width`: The target width.
  /// - `p_height`: The target height.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize(&mut self, p_width: u32, p_height: u32, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self;
  /// Resize the image to a percentage of its original size.
  /// 0 to 1.0 represents 0% to 100%, values greater than 1.0 represent percentages over 100%.
  /// - `p_percentage`: The percentage to resize the image by.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize_percentage(&mut self, p_percentage: f32, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self;
  /// Resize the image to the given width keeping the aspect ratio.
  /// - `p_width`: The target width.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize_width(&mut self, p_width: u32, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self;
  /// Resize the image to the given height keeping the aspect ratio.
  /// - `p_height`: The target height.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize_height(&mut self, p_height: u32, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self;
  /// Increase or decrease the image width by the given amount while keeping the aspect ratio.
  /// - `p_width`: The amount to change the width by. Positive values increase the width, negative values decrease it.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize_width_relative(&mut self, p_width: i32, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self;
  /// Increase or decrease the image height by the given amount while keeping the aspect ratio.
  /// - `p_height`: The amount to change the height by. Positive values increase the height, negative values decrease it.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize_height_relative(&mut self, p_height: i32, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self;
}

/// Resize using bilinear interpolation.
/// This function resizes the image to the specified width and height using bilinear interpolation.
/// It calculates the color of each pixel in the new image by performing a weighted average of the four nearest pixels in the original image.
/// - `p_image`: The image to resize.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
fn resize_bilinear(p_image: &mut Image, p_width: u32, p_height: u32) {
  let old_pixels = p_image.rgba();
  let buffer_size = (p_width as u64)
    .checked_mul(p_height as u64)
    .and_then(|size| size.checked_mul(4))
    .expect("Image dimensions too large") as usize;
  let mut new_pixels = vec![0; buffer_size];
  let (old_width, old_height) = p_image.dimensions::<u32>();

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % p_width;
    let y = i as u32 / p_width;

    let src_x = (x as f32 + 0.5) * (old_width as f32 / p_width as f32) - 0.5;
    let src_y = (y as f32 + 0.5) * (old_height as f32 / p_height as f32) - 0.5;

    let x0 = src_x.floor() as i32;
    let y0 = src_y.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    let fx = src_x - x0 as f32;
    let fy = src_y - y0 as f32;

    let mut result = [0u8; 4];

    // Helper to safely get pixel
    let get_pixel = |px: i32, py: i32| -> [u8; 4] {
      if px < 0 || py < 0 || px >= old_width as i32 || py >= old_height as i32 {
        [0, 0, 0, 0]
      } else {
        let idx = (py as u32 * old_width + px as u32) as usize;
        if idx * 4 + 3 < old_pixels.len() {
          [
            old_pixels[idx * 4],
            old_pixels[idx * 4 + 1],
            old_pixels[idx * 4 + 2],
            old_pixels[idx * 4 + 3],
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

    // Bilinear interpolation with premultiplied alpha to avoid dark fringes
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

    if a > 0.0 {
      result[0] = (rp / a).clamp(0.0, 255.0).round() as u8;
      result[1] = (gp / a).clamp(0.0, 255.0).round() as u8;
      result[2] = (bp / a).clamp(0.0, 255.0).round() as u8;
    } else {
      result[0] = 0;
      result[1] = 0;
      result[2] = 0;
    }
    result[3] = (a * 255.0).clamp(0.0, 255.0).round() as u8;

    chunk.copy_from_slice(&result);
  });

  p_image.set_new_pixels(new_pixels, p_width, p_height);
}

/// Resize using bicubic interpolation.
/// This function resizes the image to the specified width and height using bicubic interpolation.
/// It calculates the color of each pixel in the new image by performing a weighted average of the sixteen nearest pixels in the original image.
/// - `p_image`: The image to resize.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
fn resize_bicubic(p_image: &mut Image, p_width: u32, p_height: u32) {
  let old_pixels = p_image.rgba();
  let buffer_size = (p_width as u64)
    .checked_mul(p_height as u64)
    .and_then(|size| size.checked_mul(4))
    .expect("Image dimensions too large") as usize;
  let mut new_pixels = vec![0; buffer_size];
  let (old_width, old_height) = p_image.dimensions::<u32>();

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

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % p_width;
    let y = i as u32 / p_width;

    let src_x = (x as f32 + 0.5) * (old_width as f32 / p_width as f32) - 0.5;
    let src_y = (y as f32 + 0.5) * (old_height as f32 / p_height as f32) - 0.5;

    let x0 = src_x.floor() as i32;
    let y0 = src_y.floor() as i32;

    let fx = src_x - x0 as f32;
    let fy = src_y - y0 as f32;

    let mut result = [0u8; 4];

    // Helper to safely get pixel
    let get_pixel = |px: i32, py: i32| -> [u8; 4] {
      if px < 0 || py < 0 || px >= old_width as i32 || py >= old_height as i32 {
        [0, 0, 0, 0]
      } else {
        let idx = (py as u32 * old_width + px as u32) as usize;
        if idx * 4 + 3 < old_pixels.len() {
          [
            old_pixels[idx * 4],
            old_pixels[idx * 4 + 1],
            old_pixels[idx * 4 + 2],
            old_pixels[idx * 4 + 3],
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

    if acc_a > 0.0 {
      result[0] = (acc_r / acc_a).clamp(0.0, 255.0).round() as u8;
      result[1] = (acc_g / acc_a).clamp(0.0, 255.0).round() as u8;
      result[2] = (acc_b / acc_a).clamp(0.0, 255.0).round() as u8;
    } else {
      result[0] = 0;
      result[1] = 0;
      result[2] = 0;
    }
    result[3] = (acc_a * 255.0).clamp(0.0, 255.0).round() as u8;

    chunk.copy_from_slice(&result);
  });

  p_image.set_new_pixels(new_pixels, p_width, p_height);
}

/// Resize using Lanczos resampling.
/// This function resizes the image to the specified width and height using Lanczos resampling.
/// It calculates the color of each pixel in the new image by performing a weighted average of the surrounding pixels in the original image using the Lanczos kernel.
/// - `p_image`: The image to resize.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
fn resize_lanczos(p_image: &mut Image, p_width: u32, p_height: u32) {
  let old_pixels = p_image.rgba();
  let buffer_size = (p_width as u64)
    .checked_mul(p_height as u64)
    .and_then(|size| size.checked_mul(4))
    .expect("Image dimensions too large") as usize;
  let mut new_pixels = vec![0; buffer_size];
  let (old_width, old_height) = p_image.dimensions::<u32>();

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

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % p_width;
    let y = i as u32 / p_width;

    let src_x = (x as f32 + 0.5) * (old_width as f32 / p_width as f32) - 0.5;
    let src_y = (y as f32 + 0.5) * (old_height as f32 / p_height as f32) - 0.5;

    let x0 = src_x.floor() as i32;
    let y0 = src_y.floor() as i32;

    let fx = src_x - x0 as f32;
    let fy = src_y - y0 as f32;

    let mut result = [0u8; 4];

    // Helper to safely get pixel
    let get_pixel = |px: i32, py: i32| -> [u8; 4] {
      if px < 0 || py < 0 || px >= old_width as i32 || py >= old_height as i32 {
        [0, 0, 0, 0]
      } else {
        let idx = (py as u32 * old_width + px as u32) as usize;
        if idx * 4 + 3 < old_pixels.len() {
          [
            old_pixels[idx * 4],
            old_pixels[idx * 4 + 1],
            old_pixels[idx * 4 + 2],
            old_pixels[idx * 4 + 3],
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

    if acc_a > 0.0 {
      result[0] = (acc_r / acc_a).clamp(0.0, 255.0).round() as u8;
      result[1] = (acc_g / acc_a).clamp(0.0, 255.0).round() as u8;
      result[2] = (acc_b / acc_a).clamp(0.0, 255.0).round() as u8;
    } else {
      result[0] = 0;
      result[1] = 0;
      result[2] = 0;
    }
    result[3] = (acc_a * 255.0).clamp(0.0, 255.0).round() as u8;

    chunk.copy_from_slice(&result);
  });

  p_image.set_new_pixels(new_pixels, p_width, p_height);
}

/// Resize using Edge Direct NEDI algorithm.
/// This function resizes the image to the specified width and height using the Edge Direct NEDI algorithm.
/// It is designed to preserve edges and details in the image during the resizing process. This is the higher quality of the two Edge Direct algorithms.
/// - `p_image`: The image to resize.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
fn resize_edge_direct_nedi(p_image: &mut Image, p_width: u32, p_height: u32) {
  let old_pixels = p_image.rgba();
  let buffer_size = (p_width as u64)
    .checked_mul(p_height as u64)
    .and_then(|size| size.checked_mul(4))
    .expect("Image dimensions too large") as usize;
  let mut new_pixels = vec![0; buffer_size];
  let (old_width, old_height) = p_image.dimensions::<u32>();

  // Helper function to safely get pixel with premultiplied alpha
  let get_pixel = |px: i32, py: i32| -> [f32; 4] {
    if px < 0 || py < 0 || px >= old_width as i32 || py >= old_height as i32 {
      [0.0, 0.0, 0.0, 0.0]
    } else {
      let idx = (py as u32 * old_width + px as u32) as usize;
      if idx * 4 + 3 < old_pixels.len() {
        let a = old_pixels[idx * 4 + 3] as f32 / 255.0;
        [
          old_pixels[idx * 4] as f32 * a,
          old_pixels[idx * 4 + 1] as f32 * a,
          old_pixels[idx * 4 + 2] as f32 * a,
          a,
        ]
      } else {
        [0.0, 0.0, 0.0, 0.0]
      }
    }
  };

  // Helper to compute local covariance matrix for NEDI
  let compute_covariance = |x: i32, y: i32, window_size: i32| -> [[f32; 2]; 2] {
    let mut cov = [[0.0f32; 2]; 2];
    let mut mean_x = 0.0f32;
    let mut mean_y = 0.0f32;
    let mut count = 0;

    // Collect gradient samples in the local window
    let mut gradients: Vec<(f32, f32)> = Vec::new();

    for dy in -window_size..=window_size {
      for dx in -window_size..=window_size {
        let px = x + dx;
        let py = y + dy;

        // Compute gradient at this point
        let p_left = get_pixel(px - 1, py);
        let p_right = get_pixel(px + 1, py);
        let p_top = get_pixel(px, py - 1);
        let p_bottom = get_pixel(px, py + 1);

        // Compute luminance for gradient calculation
        let luma = |p: [f32; 4]| -> f32 {
          if p[3] > 0.0 {
            (0.299 * p[0] + 0.587 * p[1] + 0.114 * p[2]) / p[3]
          } else {
            0.0
          }
        };

        let gx = (luma(p_right) - luma(p_left)) * 0.5;
        let gy = (luma(p_bottom) - luma(p_top)) * 0.5;

        gradients.push((gx, gy));
        mean_x += gx;
        mean_y += gy;
        count += 1;
      }
    }

    if count > 0 {
      mean_x /= count as f32;
      mean_y /= count as f32;

      // Compute covariance matrix
      for (gx, gy) in gradients {
        let dx = gx - mean_x;
        let dy = gy - mean_y;
        cov[0][0] += dx * dx;
        cov[0][1] += dx * dy;
        cov[1][0] += dx * dy;
        cov[1][1] += dy * dy;
      }

      let scale = 1.0 / count as f32;
      cov[0][0] *= scale;
      cov[0][1] *= scale;
      cov[1][0] *= scale;
      cov[1][1] *= scale;
    }

    cov
  };

  // Helper to compute eigenvector of 2x2 symmetric matrix (edge direction)
  let compute_eigenvector = |cov: [[f32; 2]; 2]| -> (f32, f32) {
    let a = cov[0][0];
    let b = cov[0][1];
    let c = cov[1][1];

    // Compute eigenvalues using characteristic equation
    let trace = a + c;
    let det = a * c - b * b;
    let discriminant = (trace * trace * 0.25 - det).max(0.0).sqrt();

    let lambda1 = trace * 0.5 + discriminant;
    let lambda2 = trace * 0.5 - discriminant;

    // Use the eigenvector corresponding to the larger eigenvalue (primary edge direction)
    let use_lambda = if lambda1.abs() > lambda2.abs() {
      lambda1
    } else {
      lambda2
    };

    // Eigenvector calculation
    if b.abs() > 1e-6 {
      let v_x = use_lambda - c;
      let v_y = b;
      let norm = (v_x * v_x + v_y * v_y).sqrt();
      if norm > 0.0 {
        (v_x / norm, v_y / norm)
      } else {
        (1.0, 0.0)
      }
    } else if (a - c).abs() > 1e-6 {
      if a > c { (1.0, 0.0) } else { (0.0, 1.0) }
    } else {
      (1.0, 0.0)
    }
  };

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % p_width;
    let y = i as u32 / p_width;

    let src_x = (x as f32 + 0.5) * (old_width as f32 / p_width as f32) - 0.5;
    let src_y = (y as f32 + 0.5) * (old_height as f32 / p_height as f32) - 0.5;

    let x0 = src_x.floor() as i32;
    let y0 = src_y.floor() as i32;

    let fx = src_x - x0 as f32;
    let fy = src_y - y0 as f32;

    // Compute local covariance matrix (window size of 2 for performance)
    let cov = compute_covariance(x0, y0, 2);

    // Compute principal edge direction from covariance
    let (edge_x, edge_y) = compute_eigenvector(cov);

    // Measure edge strength from covariance eigenvalues
    let edge_strength = (cov[0][0] + cov[1][1]).sqrt();

    let mut result = [0u8; 4];

    // Threshold for using covariance-based interpolation
    const NEDI_THRESHOLD: f32 = 5.0;

    if edge_strength > NEDI_THRESHOLD {
      // Strong edge - use covariance-directed interpolation
      // Sample along the edge direction
      let step_size = 1.0;

      // Project the fractional offset onto the edge direction
      let t = fx * edge_x + fy * edge_y;

      // Sample along edge direction
      let sample_x1 = x0 as f32 - edge_x * step_size;
      let sample_y1 = y0 as f32 - edge_y * step_size;
      let sample_x2 = x0 as f32 + edge_x * step_size;
      let sample_y2 = y0 as f32 + edge_y * step_size;

      // Bilinear samples at edge-directed positions
      let get_interpolated = |sx: f32, sy: f32| -> [f32; 4] {
        let ix = sx.floor() as i32;
        let iy = sy.floor() as i32;
        let fx_local = sx - ix as f32;
        let fy_local = sy - iy as f32;

        let p00 = get_pixel(ix, iy);
        let p10 = get_pixel(ix + 1, iy);
        let p01 = get_pixel(ix, iy + 1);
        let p11 = get_pixel(ix + 1, iy + 1);

        let r0 = p00[0] * (1.0 - fx_local) + p10[0] * fx_local;
        let r1 = p01[0] * (1.0 - fx_local) + p11[0] * fx_local;
        let r = r0 * (1.0 - fy_local) + r1 * fy_local;

        let g0 = p00[1] * (1.0 - fx_local) + p10[1] * fx_local;
        let g1 = p01[1] * (1.0 - fx_local) + p11[1] * fx_local;
        let g = g0 * (1.0 - fy_local) + g1 * fy_local;

        let b0 = p00[2] * (1.0 - fx_local) + p10[2] * fx_local;
        let b1 = p01[2] * (1.0 - fx_local) + p11[2] * fx_local;
        let b = b0 * (1.0 - fy_local) + b1 * fy_local;

        let a0 = p00[3] * (1.0 - fx_local) + p10[3] * fx_local;
        let a1 = p01[3] * (1.0 - fx_local) + p11[3] * fx_local;
        let a = a0 * (1.0 - fy_local) + a1 * fy_local;

        [r, g, b, a]
      };

      let s1 = get_interpolated(sample_x1, sample_y1);
      let s2 = get_interpolated(sample_x2, sample_y2);

      // Interpolate between the two edge-directed samples
      let interp_t = (t + 1.0) * 0.5; // Normalize to [0, 1]
      let acc_r = s1[0] * (1.0 - interp_t) + s2[0] * interp_t;
      let acc_g = s1[1] * (1.0 - interp_t) + s2[1] * interp_t;
      let acc_b = s1[2] * (1.0 - interp_t) + s2[2] * interp_t;
      let acc_a = s1[3] * (1.0 - interp_t) + s2[3] * interp_t;

      if acc_a > 0.0 {
        result[0] = (acc_r / acc_a).clamp(0.0, 255.0).round() as u8;
        result[1] = (acc_g / acc_a).clamp(0.0, 255.0).round() as u8;
        result[2] = (acc_b / acc_a).clamp(0.0, 255.0).round() as u8;
      }
      result[3] = (acc_a * 255.0).clamp(0.0, 255.0).round() as u8;
    } else {
      // Weak or no edge - use bicubic interpolation for better quality
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
          let w = cubic_kernel(dx as f32 - fx) * cubic_kernel(dy as f32 - fy);
          acc_r += p[0] * w;
          acc_g += p[1] * w;
          acc_b += p[2] * w;
          acc_a += p[3] * w;
          weight_sum += w;
        }
      }

      if weight_sum > 0.0 {
        acc_r /= weight_sum;
        acc_g /= weight_sum;
        acc_b /= weight_sum;
        acc_a /= weight_sum;
      }

      if acc_a > 0.0 {
        result[0] = (acc_r / acc_a).clamp(0.0, 255.0).round() as u8;
        result[1] = (acc_g / acc_a).clamp(0.0, 255.0).round() as u8;
        result[2] = (acc_b / acc_a).clamp(0.0, 255.0).round() as u8;
      }
      result[3] = (acc_a * 255.0).clamp(0.0, 255.0).round() as u8;
    }

    chunk.copy_from_slice(&result);
  });

  p_image.set_new_pixels(new_pixels, p_width, p_height);
}

/// Resize using Edge Direct EDI algorithm.
/// This function resizes the image to the specified width and height using the Edge Direct EDI algorithm.
/// It is designed to preserve edges and details in the image during the resizing process. This is the faster of the two Edge Direct algorithms.
/// - `p_image`: The image to resize.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
fn resize_edge_direct_edi(p_image: &mut Image, p_width: u32, p_height: u32) {
  let old_pixels = p_image.rgba();
  let buffer_size = (p_width as u64)
    .checked_mul(p_height as u64)
    .and_then(|size| size.checked_mul(4))
    .expect("Image dimensions too large") as usize;
  let mut new_pixels = vec![0; buffer_size];
  let (old_width, old_height) = p_image.dimensions::<u32>();

  // Helper function to safely get pixel with premultiplied alpha
  let get_pixel = |px: i32, py: i32| -> [f32; 4] {
    if px < 0 || py < 0 || px >= old_width as i32 || py >= old_height as i32 {
      [0.0, 0.0, 0.0, 0.0]
    } else {
      let idx = (py as u32 * old_width + px as u32) as usize;
      if idx * 4 + 3 < old_pixels.len() {
        let a = old_pixels[idx * 4 + 3] as f32 / 255.0;
        [
          old_pixels[idx * 4] as f32 * a,
          old_pixels[idx * 4 + 1] as f32 * a,
          old_pixels[idx * 4 + 2] as f32 * a,
          a,
        ]
      } else {
        [0.0, 0.0, 0.0, 0.0]
      }
    }
  };

  // Helper to compute gradient magnitude and direction at a point
  let compute_gradient = |x: i32, y: i32| -> (f32, f32) {
    // Sobel operator for gradient detection
    let p00 = get_pixel(x - 1, y - 1);
    let p01 = get_pixel(x, y - 1);
    let p02 = get_pixel(x + 1, y - 1);
    let p10 = get_pixel(x - 1, y);
    let p12 = get_pixel(x + 1, y);
    let p20 = get_pixel(x - 1, y + 1);
    let p21 = get_pixel(x, y + 1);
    let p22 = get_pixel(x + 1, y + 1);

    // Compute gradient for luminance (weighted RGB)
    let luma = |p: [f32; 4]| -> f32 {
      if p[3] > 0.0 {
        (0.299 * p[0] + 0.587 * p[1] + 0.114 * p[2]) / p[3]
      } else {
        0.0
      }
    };

    let gx = -luma(p00) - 2.0 * luma(p10) - luma(p20) + luma(p02) + 2.0 * luma(p12) + luma(p22);
    let gy = -luma(p00) - 2.0 * luma(p01) - luma(p02) + luma(p20) + 2.0 * luma(p21) + luma(p22);

    let magnitude = (gx * gx + gy * gy).sqrt();
    let angle = gy.atan2(gx);

    (magnitude, angle)
  };

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % p_width;
    let y = i as u32 / p_width;

    let src_x = (x as f32 + 0.5) * (old_width as f32 / p_width as f32) - 0.5;
    let src_y = (y as f32 + 0.5) * (old_height as f32 / p_height as f32) - 0.5;

    let x0 = src_x.floor() as i32;
    let y0 = src_y.floor() as i32;

    let fx = src_x - x0 as f32;
    let fy = src_y - y0 as f32;

    // Compute gradient at the interpolation point
    let (magnitude, angle) = compute_gradient(x0, y0);

    // Threshold for edge detection
    const EDGE_THRESHOLD: f32 = 10.0;

    let mut result = [0u8; 4];

    if magnitude > EDGE_THRESHOLD {
      // Strong edge detected - use directional interpolation
      // Normalize angle to [0, π]
      let norm_angle = if angle < 0.0 {
        angle + std::f32::consts::PI
      } else {
        angle
      };

      // Determine primary interpolation direction (quantized to 8 directions)
      let direction = ((norm_angle / std::f32::consts::PI * 4.0).round() as i32) % 4;

      // Interpolate along the edge direction
      let (p0, p1) = match direction {
        0 => {
          // Horizontal edge (interpolate horizontally)
          let p0 = get_pixel(x0, y0);
          let p1 = get_pixel(x0 + 1, y0);
          (p0, p1)
        }
        1 => {
          // Diagonal edge (top-left to bottom-right)
          let p0 = get_pixel(x0, y0);
          let p1 = get_pixel(x0 + 1, y0 + 1);
          (p0, p1)
        }
        2 => {
          // Vertical edge (interpolate vertically)
          let p0 = get_pixel(x0, y0);
          let p1 = get_pixel(x0, y0 + 1);
          (p0, p1)
        }
        _ => {
          // Diagonal edge (top-right to bottom-left)
          let p0 = get_pixel(x0 + 1, y0);
          let p1 = get_pixel(x0, y0 + 1);
          (p0, p1)
        }
      };

      // Linear interpolation along the edge
      let t = if direction == 0 {
        fx
      } else if direction == 2 {
        fy
      } else {
        (fx + fy) * 0.5
      };

      let acc_r = p0[0] * (1.0 - t) + p1[0] * t;
      let acc_g = p0[1] * (1.0 - t) + p1[1] * t;
      let acc_b = p0[2] * (1.0 - t) + p1[2] * t;
      let acc_a = p0[3] * (1.0 - t) + p1[3] * t;

      if acc_a > 0.0 {
        result[0] = (acc_r / acc_a).clamp(0.0, 255.0).round() as u8;
        result[1] = (acc_g / acc_a).clamp(0.0, 255.0).round() as u8;
        result[2] = (acc_b / acc_a).clamp(0.0, 255.0).round() as u8;
      }
      result[3] = (acc_a * 255.0).clamp(0.0, 255.0).round() as u8;
    } else {
      // Weak or no edge - use standard bilinear interpolation
      let p00 = get_pixel(x0, y0);
      let p10 = get_pixel(x0 + 1, y0);
      let p01 = get_pixel(x0, y0 + 1);
      let p11 = get_pixel(x0 + 1, y0 + 1);

      let r0 = p00[0] * (1.0 - fx) + p10[0] * fx;
      let r1 = p01[0] * (1.0 - fx) + p11[0] * fx;
      let acc_r = r0 * (1.0 - fy) + r1 * fy;

      let g0 = p00[1] * (1.0 - fx) + p10[1] * fx;
      let g1 = p01[1] * (1.0 - fx) + p11[1] * fx;
      let acc_g = g0 * (1.0 - fy) + g1 * fy;

      let b0 = p00[2] * (1.0 - fx) + p10[2] * fx;
      let b1 = p01[2] * (1.0 - fx) + p11[2] * fx;
      let acc_b = b0 * (1.0 - fy) + b1 * fy;

      let a0 = p00[3] * (1.0 - fx) + p10[3] * fx;
      let a1 = p01[3] * (1.0 - fx) + p11[3] * fx;
      let acc_a = a0 * (1.0 - fy) + a1 * fy;

      if acc_a > 0.0 {
        result[0] = (acc_r / acc_a).clamp(0.0, 255.0).round() as u8;
        result[1] = (acc_g / acc_a).clamp(0.0, 255.0).round() as u8;
        result[2] = (acc_b / acc_a).clamp(0.0, 255.0).round() as u8;
      }
      result[3] = (acc_a * 255.0).clamp(0.0, 255.0).round() as u8;
    }

    chunk.copy_from_slice(&result);
  });

  p_image.set_new_pixels(new_pixels, p_width, p_height);
}

/// Resize using nearest neighbor interpolation.
/// This function resizes the image to the specified width and height using nearest neighbor interpolation.
/// It assigns each pixel in the new image the color of the nearest pixel in the original image.
/// - `p_image`: The image to resize.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
fn resize_nearest_neighbor(p_image: &mut Image, p_width: u32, p_height: u32) {
  let old_pixels = p_image.rgba();
  // Use checked multiplication to avoid overflow when multiplying large dimensions
  let buffer_size = (p_width as u64)
    .checked_mul(p_height as u64)
    .and_then(|size| size.checked_mul(4))
    .expect("Image dimensions too large") as usize;
  let mut new_pixels = vec![0; buffer_size];
  let (old_width, old_height) = p_image.dimensions::<u32>();

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

  p_image.set_new_pixels(new_pixels, p_width, p_height);
}

/// Internal function to perform the actual resizing based on the selected algorithm.
/// This function dispatches to the appropriate resizing algorithm implementation.
/// - `p_image`: The image to resize.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
/// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
fn resize_impl(p_image: &mut Image, p_width: u32, p_height: u32, p_algorithm: TransformAlgorithm) {
  match p_algorithm {
    TransformAlgorithm::NearestNeighbor => resize_nearest_neighbor(p_image, p_width, p_height),
    TransformAlgorithm::Bilinear => resize_bilinear(p_image, p_width, p_height),
    TransformAlgorithm::Bicubic => resize_bicubic(p_image, p_width, p_height),
    TransformAlgorithm::Lanczos => resize_lanczos(p_image, p_width, p_height),
    TransformAlgorithm::EdgeDirectNEDI => resize_edge_direct_nedi(p_image, p_width, p_height),
    TransformAlgorithm::EdgeDirectEDI => resize_edge_direct_edi(p_image, p_width, p_height),
    TransformAlgorithm::Auto => {
      let (old_width, old_height) = p_image.dimensions::<u32>();
      let resolved_algo = get_resize_algorithm(None, old_width, old_height, p_width, p_height);
      resize_impl(p_image, p_width, p_height, resolved_algo);
    }
  }
}

/// Determine the best resize algorithm based on the original and target dimensions.
/// If no algorithm is specified, this function selects an appropriate algorithm:
/// - If the target size is less than half the original size, Lanczos is chosen for high quality downscaling.
/// - If the target size is larger than the original size, Bicubic is chosen for quality upscaling.
/// - If the target size is smaller than the original size but not less than half, Bilinear is chosen for a good balance.
/// - If the target size is the same as the original size, Bilinear is used as a default.
pub(crate) fn get_resize_algorithm(
  p_algorithm: impl Into<Option<TransformAlgorithm>>, p_old_width: u32, p_old_height: u32, p_width: u32, p_height: u32,
) -> TransformAlgorithm {
  match p_algorithm.into() {
    Some(TransformAlgorithm::Auto) | None => {
      // Uses the Lanczos algorithm when downscaling more than half for best quality.
      if p_width < p_old_width / 2 || p_height < p_old_height / 2 {
        TransformAlgorithm::Lanczos
      }
      // Uses Bicubic when upscaling for better quality.
      else if p_width > p_old_width || p_height > p_old_height {
        TransformAlgorithm::Bicubic
      }
      // Uses Bilinear for moderate downscaling.
      else if p_width < p_old_width || p_height < p_old_height {
        TransformAlgorithm::Bilinear
      } else {
        TransformAlgorithm::Bicubic
      }
    }
    Some(algo) => algo,
  }
}

/// Resize the image to the given dimensions.
/// This does not maintain the aspect ratio unless the given dimensions match the original aspect ratio.
/// Only performs the resize if the dimensions have changed.
/// - `p_image`: The image to resize.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
/// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
pub fn resize(p_image: &mut Image, p_width: u32, p_height: u32, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
  let start = Instant::now();
  let (old_width, old_height) = p_image.dimensions::<u32>();

  let resolved_algo = get_resize_algorithm(p_algorithm.into(), old_width, old_height, p_width, p_height);
  // Only perform resize if dimensions have changed.
  if p_width != old_width || p_height != old_height {
    resize_impl(p_image, p_width, p_height, resolved_algo);
  }

  // DebugTransform::Resize(resolved_algo, old_width, old_height, p_width, p_height, start.elapsed()).log();
}

/// Resize the image to the given width keeping the aspect ratio.
/// - `p_image`: The image to resize.
/// - `p_width`: The target width.
/// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
pub fn width(p_image: &mut Image, p_width: u32, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
  let (old_width, old_height) = p_image.dimensions::<u32>();
  let new_height = ((old_height as f32 / old_width as f32 * p_width as f32) as u32).max(1);
  resize(p_image, p_width, new_height, p_algorithm);
}

/// Resize the image to the given height keeping the aspect ratio.
/// - `p_image`: The image to resize.
/// - `p_height`: The target height.
/// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
pub fn height(p_image: &mut Image, p_height: u32, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
  let (old_width, old_height) = p_image.dimensions::<u32>();
  let new_width = (old_width as f32 / old_height as f32 * p_height as f32) as u32;
  resize(p_image, new_width, p_height, p_algorithm);
}

/// Increase or decrease the image width by the given amount while keeping the aspect ratio.
/// - `p_image`: The image to resize.
/// - `p_width`: The amount to change the width by (positive to increase, negative to decrease).
/// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
pub fn width_relative(p_image: &mut Image, p_width: i32, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
  let (image_width, _) = p_image.dimensions::<u32>();
  let new_width = (image_width as i32 + p_width).max(1) as u32;
  width(p_image, new_width, p_algorithm);
}

/// Increase or decrease the image height by the given amount while keeping the aspect ratio.
/// - `p_image`: The image to resize.
/// - `p_height`: The amount to change the height by (positive to increase, negative to decrease).
/// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
pub fn height_relative(p_image: &mut Image, p_height: i32, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
  let (_, image_height) = p_image.dimensions::<u32>();
  let new_height = (image_height as i32 + p_height).max(1) as u32;
  height(p_image, new_height, p_algorithm);
}

/// Resize the image by a scale factor.
///
/// # Arguments
/// * `p_percentage` - Scale factor where:
///   - 0 < p_percentage < 1 decreases the size (e.g., 0.5 = half size)
///   - p_percentage == 1 keeps the original size
///   - p_percentage > 1 increases the size (e.g., 2 = double size, 50 = 50x larger)
///
/// # Examples
/// - `0.5` resizes to 50% of original (half size)
/// - `1.0` keeps original size
/// - `2.0` resizes to 200% of original (double size)
/// - `50.0` resizes to 5000% of original (50x larger)
pub fn resize_percentage(p_image: &mut Image, p_percentage: f32, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
  if p_percentage <= 0.0 {
    return; // Invalid scale factor, do nothing
  }

  let (old_width, old_height) = p_image.dimensions::<u32>();
  let new_width = ((old_width as f32 * p_percentage).max(1.0)) as u32;
  let new_height = ((old_height as f32 * p_percentage).max(1.0)) as u32;
  resize(p_image, new_width, new_height, p_algorithm);
}
