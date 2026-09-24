use crate::transform::TransformAlgorithm;
use crate::transform::interpolation;
use crate::{Image, Size};
// use crate::utils::debug::DebugTransform;
use primitives::Image as PrimitiveImage;
use rayon::prelude::*;

/// Describes how an image should be resized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResizeTarget {
  /// Resize to exact dimensions without preserving aspect ratio.
  Exact(Size),
  /// Resize to a width while preserving aspect ratio.
  FitWidth(u32),
  /// Resize to a height while preserving aspect ratio.
  FitHeight(u32),
  /// Change the width by a number of pixels while preserving aspect ratio.
  RelativeWidth(i32),
  /// Change the height by a number of pixels while preserving aspect ratio.
  RelativeHeight(i32),
  /// Resize both dimensions by a positive scale factor.
  Scale(f32),
}

/// Trait for resizing functionality.
pub trait Resize {
  /// Resize the image according to the supplied target.
  /// - `p_target`: The desired dimensions or aspect-preserving strategy.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize(&mut self, p_target: ResizeTarget, p_algorithm: impl Into<Option<TransformAlgorithm>>);
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
        let luma =
          |p: [f32; 4]| -> f32 { if p[3] > 0.0 { (0.299 * p[0] + 0.587 * p[1] + 0.114 * p[2]) / p[3] } else { 0.0 } };

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
    let use_lambda = if lambda1.abs() > lambda2.abs() { lambda1 } else { lambda2 };

    // Eigenvector calculation
    if b.abs() > 1e-6 {
      let v_x = use_lambda - c;
      let v_y = b;
      let norm = (v_x * v_x + v_y * v_y).sqrt();
      if norm > 0.0 { (v_x / norm, v_y / norm) } else { (1.0, 0.0) }
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

  p_image.set_new_pixels(&new_pixels, p_width, p_height);
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
    let luma =
      |p: [f32; 4]| -> f32 { if p[3] > 0.0 { (0.299 * p[0] + 0.587 * p[1] + 0.114 * p[2]) / p[3] } else { 0.0 } };

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
      let norm_angle = if angle < 0.0 { angle + std::f32::consts::PI } else { angle };

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

  p_image.set_new_pixels(&new_pixels, p_width, p_height);
}

/// Internal function to perform the actual resizing based on the selected algorithm.
/// This function dispatches to the appropriate resizing algorithm implementation.
/// - `p_image`: The image to resize.
/// - `p_width`: The target width.
/// - `p_height`: The target height.
/// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
fn resize_impl(p_image: &mut Image, p_width: u32, p_height: u32, p_algorithm: TransformAlgorithm) {
  let new_pixels = match p_algorithm {
    TransformAlgorithm::NearestNeighbor
    | TransformAlgorithm::Bilinear
    | TransformAlgorithm::Bicubic
    | TransformAlgorithm::Lanczos => interpolation::resample(p_image, p_width, p_height, p_algorithm.interpolation()),
    TransformAlgorithm::EdgeDirectNEDI => {
      resize_edge_direct_nedi(p_image, p_width, p_height);
      return;
    }
    TransformAlgorithm::EdgeDirectEDI => {
      resize_edge_direct_edi(p_image, p_width, p_height);
      return;
    }
    TransformAlgorithm::Auto => {
      let (old_width, old_height) = p_image.dimensions::<u32>();
      let resolved_algo = get_resize_algorithm(None, old_width, old_height, p_width, p_height);
      resize_impl(p_image, p_width, p_height, resolved_algo);
      return;
    }
  };
  p_image.set_new_pixels(&new_pixels, p_width, p_height);
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

/// Resolves an image resize target to exact pixel dimensions.
fn target_dimensions(p_image: &Image, p_target: ResizeTarget) -> Option<(u32, u32)> {
  let (old_width, old_height) = p_image.dimensions::<u32>();
  match p_target {
    ResizeTarget::Exact(size) => Some((size.width.max(0.0) as u32, size.height.max(0.0) as u32)),
    ResizeTarget::FitWidth(width) => {
      let height = ((old_height as f32 / old_width as f32 * width as f32) as u32).max(1);
      Some((width, height))
    }
    ResizeTarget::FitHeight(height) => {
      let width = (old_width as f32 / old_height as f32 * height as f32) as u32;
      Some((width, height))
    }
    ResizeTarget::RelativeWidth(amount) => {
      let width = (old_width as i32 + amount).max(1) as u32;
      target_dimensions(p_image, ResizeTarget::FitWidth(width))
    }
    ResizeTarget::RelativeHeight(amount) => {
      let height = (old_height as i32 + amount).max(1) as u32;
      target_dimensions(p_image, ResizeTarget::FitHeight(height))
    }
    ResizeTarget::Scale(scale) if scale > 0.0 => {
      Some(((old_width as f32 * scale).max(1.0) as u32, (old_height as f32 * scale).max(1.0) as u32))
    }
    ResizeTarget::Scale(_) => None,
  }
}

/// A resize that has been described but not yet run. Create one with [`resize`], optionally set the algorithm
/// with [`ResizeImage::with_algorithm`], then run it with [`ResizeImage::apply`].
pub struct ResizeImage {
  pub target: ResizeTarget,
  pub algorithm: Option<TransformAlgorithm>,
}

impl ResizeImage {
  /// Sets the resizing algorithm. When `None` (the default), the best algorithm is selected automatically.
  pub fn with_algorithm(mut self, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> Self {
    self.algorithm = p_algorithm.into();
    self
  }

  /// Resizes the image. Nothing happens when the resolved dimensions are the same as the image's.
  pub fn apply(&self, p_image: &mut Image) {
    let (old_width, old_height) = p_image.dimensions::<u32>();
    let Some((width, height)) = target_dimensions(p_image, self.target) else {
      return;
    };
    if width == old_width && height == old_height {
      return;
    }

    let algorithm = get_resize_algorithm(self.algorithm, old_width, old_height, width, height);
    resize_impl(p_image, width, height, algorithm);
  }
}

/// Resizes an image according to an exact or aspect-preserving target.
/// # Arguments
/// - `p_target`: The desired dimensions or resizing strategy.
///
/// The resizing algorithm is chosen automatically unless set with [`ResizeImage::with_algorithm`].
pub fn resize(p_target: ResizeTarget) -> ResizeImage {
  ResizeImage {
    target: p_target,
    algorithm: None,
  }
}

// Implement Resize trait for primitives::Image so external code that expects
// `abra_core::Image` (a re-export of primitives::Image) can call `.resize(...)`.
impl Resize for PrimitiveImage {
  fn resize(&mut self, p_target: ResizeTarget, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
    crate::transform::resize(p_target).with_algorithm(p_algorithm).apply(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn image(p_width: u32, p_height: u32) -> Image {
    Image::from_rgba_bytes(p_width, p_height, &[10, 20, 30, 255].repeat((p_width * p_height) as usize))
  }

  #[test]
  fn builder_resizes_to_the_target() {
    let mut img = image(20, 10);
    resize(ResizeTarget::Exact(Size::new(8, 4))).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (8, 4));

    resize(ResizeTarget::FitWidth(16)).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (16, 8));
  }

  #[test]
  fn builder_accepts_an_algorithm() {
    for algorithm in [TransformAlgorithm::NearestNeighbor, TransformAlgorithm::Lanczos] {
      let mut img = image(20, 10);
      resize(ResizeTarget::Scale(2.0)).with_algorithm(algorithm).apply(&mut img);
      assert_eq!(img.dimensions::<u32>(), (40, 20));
    }
  }

  #[test]
  fn builder_leaves_an_image_alone_when_the_size_is_unchanged() {
    let mut img = image(6, 3);
    let before = img.rgba().to_vec();
    resize(ResizeTarget::Exact(Size::new(6, 3))).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (6, 3));
    assert_eq!(img.rgba(), before.as_slice());
  }

  #[test]
  fn trait_method_still_works() {
    let mut img = image(10, 10);
    Resize::resize(&mut img, ResizeTarget::Exact(Size::new(5, 5)), None);
    assert_eq!(img.dimensions::<u32>(), (5, 5));
  }
}
