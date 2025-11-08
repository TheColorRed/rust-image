#![allow(unused_imports, unused_variables, unused_mut)]
use crate::image::Image;
use rayon::prelude::*;

/// Applies a box blur to an image.
pub fn box_blur(image: &mut Image, radius: u32) {
  let (width, height) = image.dimensions::<u32>();
  let mut pixels = image.rgba();
  let kernel_radius = radius as i32;
  let kernel_size = 2 * kernel_radius + 1;

  // let mut temp_r = vec![0.0; (width * height) as usize];
  // let mut temp_g = vec![0.0; (width * height) as usize];
  // let mut temp_b = vec![0.0; (width * height) as usize];

  // Horizontal pass
  // pixels.par_chunks_mut(4).enumerate().for_each(|(y, pixel)| {
  //   for x in 0..width {
  //     let mut r = 0.0;
  //     let mut g = 0.0;
  //     let mut b = 0.0;
  //     let mut weight_sum = 0.0;

  //     for kx in -kernel_radius..=kernel_radius {
  //       let px = (x as i32 + kx).clamp(0, width as i32 - 1);
  //       let pixel_index = (y as i32 * width as i32 + px) as usize;
  //       r += pixels[pixel_index * 4] as f32;
  //       g += pixels[pixel_index * 4 + 1] as f32;
  //       b += pixels[pixel_index * 4 + 2] as f32;
  //       weight_sum += 1.0;
  //     }

  //     let pixel_index = (y * width as usize + x as usize) as usize;
  //     pixel[pixel_index * 4] = (r / weight_sum) as u8;
  //     pixel[pixel_index * 4 + 1] = (g / weight_sum) as u8;
  //     pixel[pixel_index * 4 + 2] = (b / weight_sum) as u8;
  //   }
  // });

  // (0..height).into_par_iter().for_each(|y| {
  //   for x in 0..width {
  //     let mut r = 0.0;
  //     let mut g = 0.0;
  //     let mut b = 0.0;
  //     let mut weight_sum = 0.0;

  //     for kx in -kernel_radius..=kernel_radius {
  //       let px = (x as u32 + kx as u32).clamp(0, width - 1);
  //       let pixel_index = (y * width + px) as usize;
  //       r += image.r[pixel_index] as f32;
  //       g += image.g[pixel_index] as f32;
  //       b += image.b[pixel_index] as f32;
  //       weight_sum += 1.0;
  //     }

  //     temp_r[(y * width + x) as usize] = r / weight_sum;
  //     temp_g[(y * width + x) as usize] = g / weight_sum;
  //     temp_b[(y * width + x) as usize] = b / weight_sum;
  //   }
  // });

  // Vertical pass
  // for x in 0..width {
  //   for y in 0..height {
  //     let mut r = 0.0;
  //     let mut g = 0.0;
  //     let mut b = 0.0;
  //     let mut weight_sum = 0.0;

  //     for ky in -kernel_radius..=kernel_radius {
  //       let py = (y as u32 + ky as u32).clamp(0, height - 1);
  //       let pixel_index = (py * width + x) as usize;
  //       r += temp_r[pixel_index];
  //       g += temp_g[pixel_index];
  //       b += temp_b[pixel_index];
  //       weight_sum += 1.0;
  //     }

  //     let pixel_index = (y * width + x) as usize;
  //     image.r[pixel_index] = (r / weight_sum) as u8;
  //     image.g[pixel_index] = (g / weight_sum) as u8;
  //     image.b[pixel_index] = (b / weight_sum) as u8;
  //   }
  // }
}
