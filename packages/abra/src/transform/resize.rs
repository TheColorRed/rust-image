use core::fmt::Display;
use std::fmt::Debug;
use std::time::Instant;

use crate::image::Image;
use crate::utils::debug::DebugTransform;
use rayon::prelude::*;

/// Trait for resizing functionality.
pub trait Resize {
  /// Resize the image to the given dimensions.
  /// This does not maintain the aspect ratio unless the given dimensions match the original aspect ratio.
  /// - `p_width`: The target width.
  /// - `p_height`: The target height.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize(&mut self, p_width: u32, p_height: u32, p_algorithm: Option<ResizeAlgorithm>) -> &mut Self;
  /// Resize the image to a percentage of its original size.
  /// 0 to 1.0 represents 0% to 100%, values greater than 1.0 represent percentages over 100%.
  /// - `p_percentage`: The percentage to resize the image by.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize_percentage(&mut self, p_percentage: f32, p_algorithm: Option<ResizeAlgorithm>) -> &mut Self;
  /// Resize the image to the given width keeping the aspect ratio.
  /// - `p_width`: The target width.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize_width(&mut self, p_width: u32, p_algorithm: Option<ResizeAlgorithm>) -> &mut Self;
  /// Resize the image to the given height keeping the aspect ratio.
  /// - `p_height`: The target height.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize_height(&mut self, p_height: u32, p_algorithm: Option<ResizeAlgorithm>) -> &mut Self;
  /// Increase or decrease the image width by the given amount while keeping the aspect ratio.
  /// - `p_width`: The amount to change the width by. Positive values increase the width, negative values decrease it.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize_width_relative(&mut self, p_width: i32, p_algorithm: Option<ResizeAlgorithm>) -> &mut Self;
  /// Increase or decrease the image height by the given amount while keeping the aspect ratio.
  /// - `p_height`: The amount to change the height by. Positive values increase the height, negative values decrease it.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn resize_height_relative(&mut self, p_height: i32, p_algorithm: Option<ResizeAlgorithm>) -> &mut Self;
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Algorithms available for resizing images.
/// Each algorithm offers a different balance between performance and quality.
pub enum ResizeAlgorithm {
  /// Nearest neighbor interpolation. Fast but low quality.
  NearestNeighbor,
  /// Blends 4 neighboring pixels. Good balance between quality and performance.
  Bilinear,
  /// Uses a cubic kernel over 16 pixels (4x4 neighborhood). Better quality than bilinear, noticeable improvement for downscaling.
  Bicubic,
  /// Uses Lanczos-3 kernel over 36 pixels (6x6 neighborhood). Highest quality, best edge preservation, but most computationally expensive.
  Lanczos,
  /// Automatically selects the best algorithm based on the image and target size.
  Auto,
}

/// Displays the name of the resize algorithm that is being used.
impl Display for ResizeAlgorithm {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ResizeAlgorithm::NearestNeighbor => write!(f, "NearestNeighbor"),
      ResizeAlgorithm::Bilinear => write!(f, "Bilinear"),
      ResizeAlgorithm::Bicubic => write!(f, "Bicubic"),
      ResizeAlgorithm::Lanczos => write!(f, "Lanczos"),
      ResizeAlgorithm::Auto => write!(f, "Auto"),
    }
  }
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

    // Bilinear interpolation for each channel
    for c in 0..4 {
      let i00 = p00[c] as f32;
      let i10 = p10[c] as f32;
      let i01 = p01[c] as f32;
      let i11 = p11[c] as f32;

      let i0 = i00 * (1.0 - fx) + i10 * fx;
      let i1 = i01 * (1.0 - fx) + i11 * fx;
      let i = i0 * (1.0 - fy) + i1 * fy;

      result[c] = i.round() as u8;
    }

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

    // Sample 4x4 neighborhood
    for c in 0..4 {
      let mut value = 0.0;
      let mut weight_sum = 0.0;

      for dy in -1..=2 {
        for dx in -1..=2 {
          let px = x0 + dx;
          let py = y0 + dy;
          let pixel = get_pixel(px, py);
          let sample = pixel[c] as f32;

          let weight = cubic_kernel(dx as f32 - fx) * cubic_kernel(dy as f32 - fy);
          value += sample * weight;
          weight_sum += weight;
        }
      }

      if weight_sum > 0.0 {
        value /= weight_sum;
      }

      result[c] = (value.max(0.0).min(255.0)).round() as u8;
    }

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

    // Sample neighborhood with Lanczos kernel
    for c in 0..4 {
      let mut value = 0.0;
      let mut weight_sum = 0.0;

      for dy in -LANCZOS_SIZE + 1..=LANCZOS_SIZE {
        for dx in -LANCZOS_SIZE + 1..=LANCZOS_SIZE {
          let px = x0 + dx;
          let py = y0 + dy;
          let pixel = get_pixel(px, py);
          let sample = pixel[c] as f32;

          let weight = lanczos_kernel(dx as f32 - fx) * lanczos_kernel(dy as f32 - fy);
          value += sample * weight;
          weight_sum += weight;
        }
      }

      if weight_sum > 0.0 {
        value /= weight_sum;
      }

      result[c] = (value.max(0.0).min(255.0)).round() as u8;
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
fn resize_impl(p_image: &mut Image, p_width: u32, p_height: u32, p_algorithm: ResizeAlgorithm) {
  match p_algorithm {
    ResizeAlgorithm::NearestNeighbor => resize_nearest_neighbor(p_image, p_width, p_height),
    ResizeAlgorithm::Bilinear => resize_bilinear(p_image, p_width, p_height),
    ResizeAlgorithm::Bicubic => resize_bicubic(p_image, p_width, p_height),
    ResizeAlgorithm::Lanczos => resize_lanczos(p_image, p_width, p_height),
    ResizeAlgorithm::Auto => {
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
  p_algorithm: Option<ResizeAlgorithm>,
  p_old_width: u32,
  p_old_height: u32,
  p_width: u32,
  p_height: u32,
) -> ResizeAlgorithm {
  match p_algorithm {
    Some(ResizeAlgorithm::Auto) | None => {
      // Uses the Lanczos algorithm when downscaling more than half for best quality.
      if p_width < p_old_width / 2 || p_height < p_old_height / 2 {
        ResizeAlgorithm::Lanczos
      }
      // Uses Bicubic when upscaling for better quality.
      else if p_width > p_old_width || p_height > p_old_height {
        ResizeAlgorithm::Bicubic
      }
      // Uses Bilinear for moderate downscaling.
      else if p_width < p_old_width || p_height < p_old_height {
        ResizeAlgorithm::Bilinear
      } else {
        ResizeAlgorithm::Bicubic
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
pub fn resize(p_image: &mut Image, p_width: u32, p_height: u32, p_algorithm: Option<ResizeAlgorithm>) {
  let start = Instant::now();
  let (old_width, old_height) = p_image.dimensions::<u32>();

  let resolved_algo = get_resize_algorithm(p_algorithm, old_width, old_height, p_width, p_height);

  // Only perform resize if dimensions have changed.
  if p_width != old_width || p_height != old_height {
    resize_impl(p_image, p_width, p_height, resolved_algo);
  }

  DebugTransform::Resize(resolved_algo, old_width, old_height, p_width, p_height, start.elapsed()).log();
}

/// Resize the image to the given width keeping the aspect ratio.
/// - `p_image`: The image to resize.
/// - `p_width`: The target width.
/// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
pub fn width(p_image: &mut Image, p_width: u32, p_algorithm: Option<ResizeAlgorithm>) {
  let (old_width, old_height) = p_image.dimensions::<u32>();
  let new_height = ((old_height as f32 / old_width as f32 * p_width as f32) as u32).max(1);
  resize(p_image, p_width, new_height, p_algorithm);
}

/// Resize the image to the given height keeping the aspect ratio.
/// - `p_image`: The image to resize.
/// - `p_height`: The target height.
/// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
pub fn height(p_image: &mut Image, p_height: u32, p_algorithm: Option<ResizeAlgorithm>) {
  let (old_width, old_height) = p_image.dimensions::<u32>();
  let new_width = (old_width as f32 / old_height as f32 * p_height as f32) as u32;
  resize(p_image, new_width, p_height, p_algorithm);
}

/// Increase or decrease the image width by the given amount while keeping the aspect ratio.
/// - `p_image`: The image to resize.
/// - `p_width`: The amount to change the width by (positive to increase, negative to decrease).
/// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
pub fn width_relative(p_image: &mut Image, p_width: i32, p_algorithm: Option<ResizeAlgorithm>) {
  let (image_width, _) = p_image.dimensions::<u32>();
  let new_width = (image_width as i32 + p_width).max(1) as u32;
  width(p_image, new_width, p_algorithm);
}

/// Increase or decrease the image height by the given amount while keeping the aspect ratio.
/// - `p_image`: The image to resize.
/// - `p_height`: The amount to change the height by (positive to increase, negative to decrease).
/// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
pub fn height_relative(p_image: &mut Image, p_height: i32, p_algorithm: Option<ResizeAlgorithm>) {
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
pub fn resize_percentage(p_image: &mut Image, p_percentage: f32, p_algorithm: Option<ResizeAlgorithm>) {
  if p_percentage <= 0.0 {
    return; // Invalid scale factor, do nothing
  }

  let (old_width, old_height) = p_image.dimensions::<u32>();
  let new_width = ((old_width as f32 * p_percentage).max(1.0)) as u32;
  let new_height = ((old_height as f32 * p_percentage).max(1.0)) as u32;
  resize(p_image, new_width, new_height, p_algorithm);
}
