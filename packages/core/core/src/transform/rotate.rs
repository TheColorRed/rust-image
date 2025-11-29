use std::time::{Duration, Instant};

use crate::Image;
use primitives::Image as PrimitiveImage;

use rayon::prelude::*;

use super::{TransformAlgorithm, resize::get_resize_algorithm};

/// Trait for rotating images.
pub trait Rotate {
  /// Rotates the image by the specified number of degrees.
  /// Positive values rotate clockwise, negative values rotate counter-clockwise.
  /// Accepts any numeric type that can losslessly or approximately convert into `f64` (e.g. `i32`, `u32`, `f32`, `f64`).
  /// Internally coerces to `f32` for computation.
  fn rotate(&mut self, p_degrees: impl Into<f64>, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self;
  /// Flips the image horizontally.
  fn flip_horizontal(&mut self) -> &mut Self;

  /// Flips the image vertically.
  fn flip_vertical(&mut self) -> &mut Self;
}

/// Calculate the new size of the image after rotation.
/// This is to resize the new image to fit the rotated image without cropping.
/// * `width` - The source image width.
/// * `height` - The source image height.
/// * `degrees` - The degrees to rotate the image.
fn calc_image_new_size(p_width: u32, p_height: u32, p_degrees: f32) -> (u32, u32) {
  let (mut width, mut height) = (p_width, p_height);
  let mut degrees = p_degrees % 180.0;
  if degrees < 0.0 {
    degrees += 180.0
  }
  if degrees >= 90.0 {
    std::mem::swap(&mut width, &mut height);
    degrees -= 90.0;
  }

  if degrees == 0.0 {
    return (width.max(1), height.max(1));
  }

  let radians = degrees.to_radians();
  let new_width = (width as f32 * radians.cos() + height as f32 * radians.sin()).abs() as u32;
  let new_height = (width as f32 * radians.sin() + height as f32 * radians.cos()).abs() as u32;

  (new_width.max(1), new_height.max(1))
}

fn fetch_pixel(p_pixels: &[u8], p_width: usize, p_height: usize, p_x: i32, p_y: i32) -> [u8; 4] {
  if p_x < 0 || p_y < 0 || p_x >= p_width as i32 || p_y >= p_height as i32 {
    // Return fully transparent pixel instead of opaque black to prevent dark edges during rotation
    return [0, 0, 0, 0];
  }

  let index = (p_y as usize * p_width + p_x as usize) * 4;
  if index + 3 >= p_pixels.len() {
    // Return fully transparent pixel instead of opaque black to prevent dark edges during rotation
    return [0, 0, 0, 0];
  }

  [
    p_pixels[index],
    p_pixels[index + 1],
    p_pixels[index + 2],
    p_pixels[index + 3],
  ]
}

fn sample_nearest_neighbor(p_pixels: &[u8], p_width: usize, p_height: usize, p_x: f32, p_y: f32) -> [u8; 4] {
  let src_x = p_x.floor() as i32;
  let src_y = p_y.floor() as i32;
  fetch_pixel(p_pixels, p_width, p_height, src_x, src_y)
}

fn sample_bilinear(p_pixels: &[u8], p_width: usize, p_height: usize, p_x: f32, p_y: f32) -> [u8; 4] {
  let x0 = p_x.floor() as i32;
  let y0 = p_y.floor() as i32;
  let x1 = x0 + 1;
  let y1 = y0 + 1;

  let fx = p_x - x0 as f32;
  let fy = p_y - y0 as f32;

  let p00 = fetch_pixel(p_pixels, p_width, p_height, x0, y0);
  let p10 = fetch_pixel(p_pixels, p_width, p_height, x1, y0);
  let p01 = fetch_pixel(p_pixels, p_width, p_height, x0, y1);
  let p11 = fetch_pixel(p_pixels, p_width, p_height, x1, y1);

  // Premultiply RGB by alpha for correct interpolation at transparent edges
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

  // Interpolate premultiplied RGB and alpha
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

  let mut out = [0u8; 4];
  if a > 0.0 {
    out[0] = (rp / a).clamp(0.0, 255.0).round() as u8;
    out[1] = (gp / a).clamp(0.0, 255.0).round() as u8;
    out[2] = (bp / a).clamp(0.0, 255.0).round() as u8;
  } else {
    out[0] = 0;
    out[1] = 0;
    out[2] = 0;
  }
  out[3] = (a * 255.0).clamp(0.0, 255.0).round() as u8;
  out
}

fn sample_bicubic(p_pixels: &[u8], p_width: usize, p_height: usize, p_x: f32, p_y: f32) -> [u8; 4] {
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

  // Accumulate premultiplied RGB and alpha
  let mut acc_r = 0.0;
  let mut acc_g = 0.0;
  let mut acc_b = 0.0;
  let mut acc_a = 0.0;
  let mut weight_sum = 0.0;

  for dy in -1..=2 {
    for dx in -1..=2 {
      let p = fetch_pixel(p_pixels, p_width, p_height, x0 + dx, y0 + dy);
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

  let mut out = [0u8; 4];
  if acc_a > 0.0 {
    out[0] = (acc_r / acc_a).clamp(0.0, 255.0).round() as u8;
    out[1] = (acc_g / acc_a).clamp(0.0, 255.0).round() as u8;
    out[2] = (acc_b / acc_a).clamp(0.0, 255.0).round() as u8;
  } else {
    out[0] = 0;
    out[1] = 0;
    out[2] = 0;
  }
  out[3] = (acc_a * 255.0).clamp(0.0, 255.0).round() as u8;
  out
}

fn sample_lanczos(p_pixels: &[u8], p_width: usize, p_height: usize, p_x: f32, p_y: f32) -> [u8; 4] {
  const LANCZOS_SIZE: i32 = 3;

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

  // Accumulate premultiplied RGB and alpha
  let mut acc_r = 0.0;
  let mut acc_g = 0.0;
  let mut acc_b = 0.0;
  let mut acc_a = 0.0;
  let mut weight_sum = 0.0;

  for dy in -LANCZOS_SIZE + 1..=LANCZOS_SIZE {
    for dx in -LANCZOS_SIZE + 1..=LANCZOS_SIZE {
      let p = fetch_pixel(p_pixels, p_width, p_height, x0 + dx, y0 + dy);
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

  let mut out = [0u8; 4];
  if acc_a > 0.0 {
    out[0] = (acc_r / acc_a).clamp(0.0, 255.0).round() as u8;
    out[1] = (acc_g / acc_a).clamp(0.0, 255.0).round() as u8;
    out[2] = (acc_b / acc_a).clamp(0.0, 255.0).round() as u8;
  } else {
    out[0] = 0;
    out[1] = 0;
    out[2] = 0;
  }
  out[3] = (acc_a * 255.0).clamp(0.0, 255.0).round() as u8;
  out
}

// Implement Rotate trait for primitives::Image so the methods are available on re-exported abra_core::Image.
impl Rotate for PrimitiveImage {
  fn rotate(&mut self, p_degrees: impl Into<f64>, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    crate::transform::rotate(self, p_degrees, p_algorithm);
    self
  }

  fn flip_horizontal(&mut self) -> &mut Self {
    crate::transform::horizontal(self);
    self
  }

  fn flip_vertical(&mut self) -> &mut Self {
    crate::transform::vertical(self);
    self
  }
}

fn sample_edge_direct_nedi(p_pixels: &[u8], p_width: usize, p_height: usize, p_x: f32, p_y: f32) -> [u8; 4] {
  let get_pixel = |px: i32, py: i32| -> [f32; 4] {
    let p = fetch_pixel(p_pixels, p_width, p_height, px, py);
    let a = p[3] as f32 / 255.0;
    [p[0] as f32 * a, p[1] as f32 * a, p[2] as f32 * a, a]
  };

  let x0 = p_x.floor() as i32;
  let y0 = p_y.floor() as i32;
  let fx = p_x - x0 as f32;
  let fy = p_y - y0 as f32;

  // Compute local covariance matrix
  let mut cov = [[0.0f32; 2]; 2];
  let mut mean_x = 0.0f32;
  let mut mean_y = 0.0f32;
  let mut count = 0;
  let mut gradients = Vec::new();

  let window_size = 2;
  for dy in -window_size..=window_size {
    for dx in -window_size..=window_size {
      let px = x0 + dx;
      let py = y0 + dy;

      let p_left = get_pixel(px - 1, py);
      let p_right = get_pixel(px + 1, py);
      let p_top = get_pixel(px, py - 1);
      let p_bottom = get_pixel(px, py + 1);

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

  // Compute eigenvector for edge direction
  let a = cov[0][0];
  let b = cov[0][1];
  let c = cov[1][1];
  let trace = a + c;
  let det = a * c - b * b;
  let discriminant = (trace * trace * 0.25 - det).max(0.0).sqrt();
  let lambda1 = trace * 0.5 + discriminant;
  let lambda2 = trace * 0.5 - discriminant;
  let use_lambda = if lambda1.abs() > lambda2.abs() {
    lambda1
  } else {
    lambda2
  };

  let (edge_x, edge_y) = if b.abs() > 1e-6 {
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
  };

  let edge_strength = (cov[0][0] + cov[1][1]).sqrt();

  if edge_strength > 5.0 {
    // Use edge-directed interpolation
    let t = fx * edge_x + fy * edge_y;
    let step_size = 1.0;

    let sample_x1 = x0 as f32 - edge_x * step_size;
    let sample_y1 = y0 as f32 - edge_y * step_size;
    let sample_x2 = x0 as f32 + edge_x * step_size;
    let sample_y2 = y0 as f32 + edge_y * step_size;

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

    let interp_t = (t + 1.0) * 0.5;
    let acc_r = s1[0] * (1.0 - interp_t) + s2[0] * interp_t;
    let acc_g = s1[1] * (1.0 - interp_t) + s2[1] * interp_t;
    let acc_b = s1[2] * (1.0 - interp_t) + s2[2] * interp_t;
    let acc_a = s1[3] * (1.0 - interp_t) + s2[3] * interp_t;

    let mut out = [0u8; 4];
    if acc_a > 0.0 {
      out[0] = (acc_r / acc_a).clamp(0.0, 255.0).round() as u8;
      out[1] = (acc_g / acc_a).clamp(0.0, 255.0).round() as u8;
      out[2] = (acc_b / acc_a).clamp(0.0, 255.0).round() as u8;
    }
    out[3] = (acc_a * 255.0).clamp(0.0, 255.0).round() as u8;
    out
  } else {
    // Fall back to bicubic
    sample_bicubic(p_pixels, p_width, p_height, p_x, p_y)
  }
}

fn sample_edge_direct_edi(p_pixels: &[u8], p_width: usize, p_height: usize, p_x: f32, p_y: f32) -> [u8; 4] {
  let get_pixel = |px: i32, py: i32| -> [f32; 4] {
    let p = fetch_pixel(p_pixels, p_width, p_height, px, py);
    let a = p[3] as f32 / 255.0;
    [p[0] as f32 * a, p[1] as f32 * a, p[2] as f32 * a, a]
  };

  let x0 = p_x.floor() as i32;
  let y0 = p_y.floor() as i32;
  let fx = p_x - x0 as f32;
  let fy = p_y - y0 as f32;

  // Compute gradient using Sobel operator
  let p00 = get_pixel(x0 - 1, y0 - 1);
  let p01 = get_pixel(x0, y0 - 1);
  let p02 = get_pixel(x0 + 1, y0 - 1);
  let p10 = get_pixel(x0 - 1, y0);
  let p12 = get_pixel(x0 + 1, y0);
  let p20 = get_pixel(x0 - 1, y0 + 1);
  let p21 = get_pixel(x0, y0 + 1);
  let p22 = get_pixel(x0 + 1, y0 + 1);

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

  if magnitude > 10.0 {
    // Strong edge - use directional interpolation
    let norm_angle = if angle < 0.0 {
      angle + std::f32::consts::PI
    } else {
      angle
    };
    let direction = ((norm_angle / std::f32::consts::PI * 4.0).round() as i32) % 4;

    let (p0, p1) = match direction {
      0 => {
        // Horizontal
        let p0 = get_pixel(x0, y0);
        let p1 = get_pixel(x0 + 1, y0);
        (p0, p1)
      }
      1 => {
        // Diagonal (top-left to bottom-right)
        let p0 = get_pixel(x0, y0);
        let p1 = get_pixel(x0 + 1, y0 + 1);
        (p0, p1)
      }
      2 => {
        // Vertical
        let p0 = get_pixel(x0, y0);
        let p1 = get_pixel(x0, y0 + 1);
        (p0, p1)
      }
      _ => {
        // Diagonal (top-right to bottom-left)
        let p0 = get_pixel(x0 + 1, y0);
        let p1 = get_pixel(x0, y0 + 1);
        (p0, p1)
      }
    };

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

    let mut out = [0u8; 4];
    if acc_a > 0.0 {
      out[0] = (acc_r / acc_a).clamp(0.0, 255.0).round() as u8;
      out[1] = (acc_g / acc_a).clamp(0.0, 255.0).round() as u8;
      out[2] = (acc_b / acc_a).clamp(0.0, 255.0).round() as u8;
    }
    out[3] = (acc_a * 255.0).clamp(0.0, 255.0).round() as u8;
    out
  } else {
    // Weak edge - use bilinear
    sample_bilinear(p_pixels, p_width, p_height, p_x, p_y)
  }
}

fn sample_pixel(
  p_pixels: &[u8], p_width: usize, p_height: usize, p_x: f32, p_y: f32, p_algorithm: TransformAlgorithm,
) -> [u8; 4] {
  match p_algorithm {
    TransformAlgorithm::NearestNeighbor => sample_nearest_neighbor(p_pixels, p_width, p_height, p_x, p_y),
    TransformAlgorithm::Bilinear => sample_bilinear(p_pixels, p_width, p_height, p_x, p_y),
    TransformAlgorithm::Bicubic => sample_bicubic(p_pixels, p_width, p_height, p_x, p_y),
    TransformAlgorithm::Lanczos => sample_lanczos(p_pixels, p_width, p_height, p_x, p_y),
    TransformAlgorithm::EdgeDirectNEDI => sample_edge_direct_nedi(p_pixels, p_width, p_height, p_x, p_y),
    TransformAlgorithm::EdgeDirectEDI => sample_edge_direct_edi(p_pixels, p_width, p_height, p_x, p_y),
    TransformAlgorithm::Auto => sample_bicubic(p_pixels, p_width, p_height, p_x, p_y),
  }
}

/// Applies the rotation to the image by copying the pixels from the source image to the destination image
/// at the proper rotated position.
/// * `image` - The image to rotate.
/// * `degrees` - The degrees to rotate the image.
/// * `width` - The new width of the image after rotation.
/// * `height` - The new height of the image after rotation.
/// * `algorithm` - The interpolation algorithm to use while sampling source pixels.
fn apply_rotation(p_image: &mut Image, p_degrees: f32, p_width: u32, p_height: u32, p_algorithm: TransformAlgorithm) {
  let (src_width, src_height) = p_image.dimensions::<usize>();
  let radians = p_degrees.to_radians();

  let src_center_x = src_width as f32 / 2.0;
  let src_center_y = src_height as f32 / 2.0;
  let dest_center_x = p_width as f32 / 2.0;
  let dest_center_y = p_height as f32 / 2.0;

  let src_pixels = p_image.rgba();
  let mut pixels = vec![0; p_width as usize * p_height as usize * 4];

  pixels.par_chunks_mut(4).enumerate().for_each(|(index, pixel)| {
    let x = index as u32 % p_width;
    let y = index as u32 / p_width;

    let src_x = (x as f32 - dest_center_x) * radians.cos() + (y as f32 - dest_center_y) * radians.sin() + src_center_x;
    let src_y = -(x as f32 - dest_center_x) * radians.sin() + (y as f32 - dest_center_y) * radians.cos() + src_center_y;

    let sample = sample_pixel(&src_pixels, src_width, src_height, src_x, src_y, p_algorithm);
    pixel.copy_from_slice(&sample);
  });

  p_image.set_new_pixels(&pixels, p_width, p_height);
}

fn rotate_internal(
  p_image: &mut Image, p_degrees: impl Into<f64>, p_algorithm: impl Into<Option<TransformAlgorithm>>,
) -> (TransformAlgorithm, u32, u32, u32, u32, Duration) {
  let start = Instant::now();
  let degrees = p_degrees.into() as f32;
  let (old_width, old_height) = p_image.dimensions::<u32>();
  let (target_width, target_height) = calc_image_new_size(old_width, old_height, degrees);
  let resolved_algorithm = get_resize_algorithm(p_algorithm, old_width, old_height, target_width, target_height);

  apply_rotation(p_image, degrees, target_width, target_height, resolved_algorithm);

  let (new_width, new_height) = p_image.dimensions::<u32>();
  (resolved_algorithm, old_width, old_height, new_width, new_height, start.elapsed())
}

/// Rotates the image by the specified number of degrees.\
/// The image will be resized to fit the rotated image without cropping.\
/// * `image` - The image to rotate.
/// * `degrees` - The number of degrees to rotate the image. Positive values rotate clockwise, negative values rotate counter-clockwise.
/// * `algorithm` - The interpolation algorithm to use. When `None`, an appropriate algorithm is selected automatically.
pub fn rotate(p_image: &mut Image, p_degrees: impl Into<f64>, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
  let degrees = p_degrees.into() as f32;
  let (_resolved_algorithm, _old_width, _old_height, _new_width, _new_height, _duration) =
    rotate_internal(p_image, degrees, p_algorithm);
  // DebugTransform::Rotate(resolved_algorithm, degrees, old_width, old_height, new_width, new_height, duration).log();
}

/// Rotates the image 90 degrees clockwise.
/// * `image` - The image to rotate.
/// * `algorithm` - The interpolation algorithm to use. When `None`, an appropriate algorithm is selected automatically.
pub fn rotate_90(p_image: &mut Image, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
  let (_resolved_algorithm, _old_width, _old_height, _new_width, _new_height, _duration) =
    rotate_internal(p_image, 90.0, p_algorithm);
  // DebugTransform::Rotate(resolved_algorithm, 90.0, old_width, old_height, new_width, new_height, duration).log();
}

/// Rotates the image 90 degrees counter-clockwise.
/// * `image` - The image to rotate.
/// * `algorithm` - The interpolation algorithm to use. When `None`, an appropriate algorithm is selected automatically.
pub fn rotate_90_ccw(p_image: &mut Image, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
  let (_resolved_algorithm, _old_width, _old_height, _new_width, _new_height, _duration) =
    rotate_internal(p_image, -90.0, p_algorithm);
  // DebugTransform::Rotate(resolved_algorithm, -90.0, old_width, old_height, new_width, new_height, duration).log();
}

/// Rotates the image 180 degrees.
/// * `image` - The image to rotate.
/// * `algorithm` - The interpolation algorithm to use. When `None`, an appropriate algorithm is selected automatically.
pub fn rotate_180(p_image: &mut Image, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
  let (_resolved_algorithm, _old_width, _old_height, _new_width, _new_height, _duration) =
    rotate_internal(p_image, 180.0, p_algorithm);
  // DebugTransform::Rotate(resolved_algorithm, 180.0, old_width, old_height, new_width, new_height, duration).log();
}
