use std::time::{Duration, Instant};

use crate::{image::Image, utils::debug::DebugTransform};

use rayon::prelude::*;

use super::{ResizeAlgorithm, resize::get_resize_algorithm};

/// Trait for rotating images.
pub trait Rotate {
  /// Rotates the image by the specified number of degrees.
  /// Positive values rotate clockwise, negative values rotate counter-clockwise.
  fn rotate(&mut self, p_degrees: f32, p_algorithm: Option<ResizeAlgorithm>) -> &mut Self;
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
    return [0, 0, 0, 0];
  }

  let index = (p_y as usize * p_width + p_x as usize) * 4;
  if index + 3 >= p_pixels.len() {
    return [0, 0, 0, 0];
  }

  [p_pixels[index], p_pixels[index + 1], p_pixels[index + 2], p_pixels[index + 3]]
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

  let mut result = [0u8; 4];

  for c in 0..4 {
    let i00 = p00[c] as f32;
    let i10 = p10[c] as f32;
    let i01 = p01[c] as f32;
    let i11 = p11[c] as f32;

    let i0 = i00 * (1.0 - fx) + i10 * fx;
    let i1 = i01 * (1.0 - fx) + i11 * fx;
    let i = i0 * (1.0 - fy) + i1 * fy;

    result[c] = i.round().clamp(0.0, 255.0) as u8;
  }

  result
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

  let mut result = [0u8; 4];

  for c in 0..4 {
    let mut value = 0.0;
    let mut weight_sum = 0.0;

    for dy in -1..=2 {
      for dx in -1..=2 {
        let sample = fetch_pixel(p_pixels, p_width, p_height, x0 + dx, y0 + dy)[c] as f32;
        let weight = cubic_kernel(dx as f32 - fx) * cubic_kernel(dy as f32 - fy);
        value += sample * weight;
        weight_sum += weight;
      }
    }

    if weight_sum > 0.0 {
      value /= weight_sum;
    }

    result[c] = value.clamp(0.0, 255.0).round() as u8;
  }

  result
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

  let mut result = [0u8; 4];

  for c in 0..4 {
    let mut value = 0.0;
    let mut weight_sum = 0.0;

    for dy in -LANCZOS_SIZE + 1..=LANCZOS_SIZE {
      for dx in -LANCZOS_SIZE + 1..=LANCZOS_SIZE {
        let sample = fetch_pixel(p_pixels, p_width, p_height, x0 + dx, y0 + dy)[c] as f32;
        let weight = lanczos_kernel(dx as f32 - fx) * lanczos_kernel(dy as f32 - fy);
        value += sample * weight;
        weight_sum += weight;
      }
    }

    if weight_sum > 0.0 {
      value /= weight_sum;
    }

    result[c] = value.clamp(0.0, 255.0).round() as u8;
  }

  result
}

fn sample_pixel(p_pixels: &[u8], p_width: usize, p_height: usize, p_x: f32, p_y: f32, p_algorithm: ResizeAlgorithm) -> [u8; 4] {
  match p_algorithm {
    ResizeAlgorithm::NearestNeighbor => sample_nearest_neighbor(p_pixels, p_width, p_height, p_x, p_y),
    ResizeAlgorithm::Bilinear => sample_bilinear(p_pixels, p_width, p_height, p_x, p_y),
    ResizeAlgorithm::Bicubic => sample_bicubic(p_pixels, p_width, p_height, p_x, p_y),
    ResizeAlgorithm::Lanczos => sample_lanczos(p_pixels, p_width, p_height, p_x, p_y),
    ResizeAlgorithm::Auto => sample_bicubic(p_pixels, p_width, p_height, p_x, p_y),
  }
}

/// Applies the rotation to the image by copying the pixels from the source image to the destination image
/// at the proper rotated position.
/// * `image` - The image to rotate.
/// * `degrees` - The degrees to rotate the image.
/// * `width` - The new width of the image after rotation.
/// * `height` - The new height of the image after rotation.
/// * `algorithm` - The interpolation algorithm to use while sampling source pixels.
fn apply_rotation(p_image: &mut Image, p_degrees: f32, p_width: u32, p_height: u32, p_algorithm: ResizeAlgorithm) {
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

  p_image.set_new_pixels(pixels, p_width, p_height);
}

fn rotate_internal(
  p_image: &mut Image,
  p_degrees: f32,
  p_algorithm: Option<ResizeAlgorithm>,
) -> (ResizeAlgorithm, u32, u32, u32, u32, Duration) {
  let start = Instant::now();
  let (old_width, old_height) = p_image.dimensions::<u32>();
  let (target_width, target_height) = calc_image_new_size(old_width, old_height, p_degrees);
  let resolved_algorithm = get_resize_algorithm(p_algorithm, old_width, old_height, target_width, target_height);

  apply_rotation(p_image, p_degrees, target_width, target_height, resolved_algorithm);

  let (new_width, new_height) = p_image.dimensions::<u32>();
  (resolved_algorithm, old_width, old_height, new_width, new_height, start.elapsed())
}

/// Rotates the image by the specified number of degrees.\
/// The image will be resized to fit the rotated image without cropping.\
/// * `image` - The image to rotate.
/// * `degrees` - The number of degrees to rotate the image. Positive values rotate clockwise, negative values rotate counter-clockwise.
/// * `algorithm` - The interpolation algorithm to use. When `None`, an appropriate algorithm is selected automatically.
pub fn rotate(p_image: &mut Image, p_degrees: f32, p_algorithm: Option<ResizeAlgorithm>) {
  let (resolved_algorithm, old_width, old_height, new_width, new_height, duration) = rotate_internal(p_image, p_degrees, p_algorithm);
  DebugTransform::Rotate(
    resolved_algorithm,
    p_degrees,
    old_width,
    old_height,
    new_width,
    new_height,
    duration,
  )
  .log();
}

/// Rotates the image 90 degrees clockwise.
/// * `image` - The image to rotate.
/// * `algorithm` - The interpolation algorithm to use. When `None`, an appropriate algorithm is selected automatically.
pub fn rotate_90(p_image: &mut Image, p_algorithm: Option<ResizeAlgorithm>) {
  let (resolved_algorithm, old_width, old_height, new_width, new_height, duration) = rotate_internal(p_image, 90.0, p_algorithm);
  DebugTransform::Rotate(resolved_algorithm, 90.0, old_width, old_height, new_width, new_height, duration).log();
}

/// Rotates the image 90 degrees counter-clockwise.
/// * `image` - The image to rotate.
/// * `algorithm` - The interpolation algorithm to use. When `None`, an appropriate algorithm is selected automatically.
pub fn rotate_90_ccw(p_image: &mut Image, p_algorithm: Option<ResizeAlgorithm>) {
  let (resolved_algorithm, old_width, old_height, new_width, new_height, duration) = rotate_internal(p_image, -90.0, p_algorithm);
  DebugTransform::Rotate(resolved_algorithm, -90.0, old_width, old_height, new_width, new_height, duration).log();
}

/// Rotates the image 180 degrees.
/// * `image` - The image to rotate.
/// * `algorithm` - The interpolation algorithm to use. When `None`, an appropriate algorithm is selected automatically.
pub fn rotate_180(p_image: &mut Image, p_algorithm: Option<ResizeAlgorithm>) {
  let (resolved_algorithm, old_width, old_height, new_width, new_height, duration) = rotate_internal(p_image, 180.0, p_algorithm);
  DebugTransform::Rotate(resolved_algorithm, 180.0, old_width, old_height, new_width, new_height, duration).log();
}
