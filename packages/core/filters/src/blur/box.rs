#![allow(unused_imports, unused_variables, unused_mut)]
use core::Image;
use rayon::prelude::*;

/// Applies a box blur to an image.
pub fn box_blur(image: &mut Image, radius: u32) {
  if radius == 0 {
    return;
  }

  let (width, height) = image.dimensions::<u32>();
  let width = width as i32;
  let height = height as i32;
  let kernel_radius = radius as i32;

  let src = image.rgba_slice();
  let mut current = src.to_vec(); // working buffer for reading
  let mut tmp = vec![0u8; current.len()];

  // Horizontal pass: read from current, write to tmp
  for y in 0..height {
    for x in 0..width {
      let mut r_sum = 0.0;
      let mut g_sum = 0.0;
      let mut b_sum = 0.0;
      let mut a_sum = 0.0;
      let mut count = 0;

      for dx in -kernel_radius..=kernel_radius {
        let nx = (x + dx).clamp(0, width - 1) as usize;
        let idx = ((y as usize) * (width as usize) + nx) * 4;
        if idx + 3 < current.len() {
          r_sum += current[idx] as f32;
          g_sum += current[idx + 1] as f32;
          b_sum += current[idx + 2] as f32;
          a_sum += current[idx + 3] as f32;
          count += 1;
        }
      }

      if count > 0 {
        let idx_out = ((y as usize) * (width as usize) + x as usize) * 4;
        if idx_out + 3 < tmp.len() {
          tmp[idx_out] = (r_sum / count as f32) as u8;
          tmp[idx_out + 1] = (g_sum / count as f32) as u8;
          tmp[idx_out + 2] = (b_sum / count as f32) as u8;
          tmp[idx_out + 3] = (a_sum / count as f32) as u8;
        }
      }
    }
  }

  // swap: tmp (horizontal result) becomes current, reuse tmp as destination for vertical pass
  std::mem::swap(&mut current, &mut tmp);

  // Vertical pass: read from current, write to tmp
  for x in 0..width {
    for y in 0..height {
      let mut r_sum = 0.0;
      let mut g_sum = 0.0;
      let mut b_sum = 0.0;
      let mut a_sum = 0.0;
      let mut count = 0;

      for dy in -kernel_radius..=kernel_radius {
        let ny = (y + dy).clamp(0, height - 1) as usize;
        let idx = (ny * (width as usize) + x as usize) * 4;
        if idx + 3 < current.len() {
          r_sum += current[idx] as f32;
          g_sum += current[idx + 1] as f32;
          b_sum += current[idx + 2] as f32;
          a_sum += current[idx + 3] as f32;
          count += 1;
        }
      }

      if count > 0 {
        let idx_out = ((y as usize) * (width as usize) + x as usize) * 4;
        if idx_out + 3 < tmp.len() {
          tmp[idx_out] = (r_sum / count as f32) as u8;
          tmp[idx_out + 1] = (g_sum / count as f32) as u8;
          tmp[idx_out + 2] = (b_sum / count as f32) as u8;
          tmp[idx_out + 3] = (a_sum / count as f32) as u8;
        }
      }
    }
  }

  image.set_rgba(tmp);
}
