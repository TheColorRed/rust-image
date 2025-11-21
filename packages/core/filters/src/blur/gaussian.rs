use core::Image;

use rayon::prelude::*;
use std::time::Instant;

fn gaussian_kernel_1d(radius: u32) -> Vec<f32> {
  let mut kernel = vec![0.0; (2 * radius + 1) as usize];
  let sigma = radius as f32 / 2.0;
  let pi = std::f32::consts::PI;
  let sum = (0..=radius)
    .map(|x| {
      let value = (-(x as f32 * x as f32) / (2.0 * sigma * sigma)).exp() / (2.0 * pi * sigma * sigma);
      kernel[radius as usize + x as usize] = value;
      kernel[radius as usize - x as usize] = value;
      value
    })
    .sum::<f32>();
  kernel.iter_mut().for_each(|value| *value /= sum);
  kernel
}

/// Applies a Gaussian blur to an image using separable convolution.
/// Uses two passes: horizontal and vertical for O(r) complexity instead of O(r²).
/// * `image` - A mutable reference to the image to be blurred.
/// * `radius` - The radius of the Gaussian kernel.
pub fn gaussian_blur(image: &mut Image, radius: u32) {
  if radius == 0 {
    return;
  }
  let _duration = Instant::now();
  let kernel = gaussian_kernel_1d(radius);
  let (width, height) = image.dimensions::<u32>();
  let width = width as i32;
  let height = height as i32;
  let kernel_radius = radius as i32;

  // Get RGBA pixels
  let pixels = image.rgba().to_vec();

  // Horizontal pass - blur along x axis
  let horizontal: Vec<u8> = (0..height as usize)
    .into_par_iter()
    .flat_map(|y| {
      (0..width as usize)
        .map(|x| {
          let mut r = 0.0;
          let mut g = 0.0;
          let mut b = 0.0;
          let mut a = 0.0;

          for kx in -kernel_radius..=kernel_radius {
            let px = (x as i32 + kx).clamp(0, width - 1) as usize;
            let src_idx = (y * width as usize + px) * 4;
            let weight = kernel[(kx + kernel_radius) as usize];

            r += pixels[src_idx] as f32 * weight;
            g += pixels[src_idx + 1] as f32 * weight;
            b += pixels[src_idx + 2] as f32 * weight;
            a += pixels[src_idx + 3] as f32 * weight;
          }

          vec![
            r.clamp(0.0, 255.0) as u8,
            g.clamp(0.0, 255.0) as u8,
            b.clamp(0.0, 255.0) as u8,
            a.clamp(0.0, 255.0) as u8,
          ]
        })
        .flatten()
        .collect::<Vec<u8>>()
    })
    .collect();

  // Vertical pass - blur along y axis
  let vertical: Vec<u8> = (0..height as usize)
    .into_par_iter()
    .flat_map(|y| {
      (0..width as usize)
        .map(|x| {
          let mut r = 0.0;
          let mut g = 0.0;
          let mut b = 0.0;
          let mut a = 0.0;

          for ky in -kernel_radius..=kernel_radius {
            let py = (y as i32 + ky).clamp(0, height - 1) as usize;
            let src_idx = (py * width as usize + x) * 4;
            let weight = kernel[(ky + kernel_radius) as usize];

            r += horizontal[src_idx] as f32 * weight;
            g += horizontal[src_idx + 1] as f32 * weight;
            b += horizontal[src_idx + 2] as f32 * weight;
            a += horizontal[src_idx + 3] as f32 * weight;
          }

          vec![
            r.clamp(0.0, 255.0) as u8,
            g.clamp(0.0, 255.0) as u8,
            b.clamp(0.0, 255.0) as u8,
            a.clamp(0.0, 255.0) as u8,
          ]
        })
        .flatten()
        .collect::<Vec<u8>>()
    })
    .collect();

  image.set_rgba(vertical);
  // DebugFilters::GaussianBlur(radius as f32, duration.elapsed()).log();
}
