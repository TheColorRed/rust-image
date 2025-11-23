#![allow(unused_imports, unused_variables, unused_mut)]
use core::image::apply_area::{
  apply_processed_pixels_to_image, blend_area_pixels, compute_area_mask, prepare_area_pixels,
};
use core::{Image, Resize};
use options::ApplyOptions;
use rayon::prelude::*;

/// Applies a box blur to an image.
pub fn box_blur(image: &mut Image, radius: u32, options: impl Into<Option<ApplyOptions>>) {
  if radius == 0 {
    return;
  }

  let (width, height) = image.dimensions::<u32>();
  let width = width as i32;
  let height = height as i32;
  let kernel_radius = radius as i32;

  let options = options.into();
  let area = options.as_ref().and_then(|o| o.area());
  let prepared = prepare_area_pixels(image, area, radius as i32);
  if prepared.area_w == 0 || prepared.area_h == 0 {
    return;
  }
  let src = prepared.pixels.as_ref();
  let width = prepared.rect_w as usize;
  let height = prepared.rect_h as usize;
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
        let nx = (x as i32 + dx).clamp(0, width as i32 - 1) as usize;
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
        let ny = (y as i32 + dy).clamp(0, height as i32 - 1) as usize;
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

  // Write back, blend if area/mask/feather present
  let full_image_processed = prepared.area_min_x == 0
    && prepared.area_min_y == 0
    && prepared.area_w == image.dimensions::<u32>().0 as i32
    && prepared.area_h == image.dimensions::<u32>().1 as i32
    && options.as_ref().and_then(|o| o.mask()).is_none();

  if full_image_processed {
    image.set_rgba_owned(tmp);
  } else {
    let mask_img_bytes: Option<&[u8]> = options.as_ref().and_then(|o| o.mask().map(|m| m.image().rgba()));
    let meta = prepared.meta();
    apply_processed_pixels_to_image(image, tmp, &meta, area, mask_img_bytes);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::Area;
  use core::Image;
  use options::ApplyOptions;

  #[test]
  fn box_blur_area_writes_back_only_area() {
    let mut img = Image::new(8, 8);
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 0u8, 0u8, 255));
    let orig = img.to_rgba_vec();
    box_blur(&mut img, 2, ApplyOptions::new().with_area(Area::rect((2.0, 2.0), (4.0, 4.0))));
    // Ensure outside area unchanged
    for y in 0..8u32 {
      for x in 0..8u32 {
        let idx = ((y * 8 + x) * 4) as usize;
        if x < 2 || x >= 6 || y < 2 || y >= 6 {
          assert_eq!(img.rgba()[idx], orig[idx]);
          assert_eq!(img.rgba()[idx + 1], orig[idx + 1]);
          assert_eq!(img.rgba()[idx + 2], orig[idx + 2]);
          assert_eq!(img.rgba()[idx + 3], orig[idx + 3]);
        }
      }
    }
  }
}
