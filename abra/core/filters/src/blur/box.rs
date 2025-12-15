use crate::common::*;

fn apply_box_blur(image: &mut Image, radius: u32) {
  if radius == 0 {
    return;
  }

  let (width, height) = image.dimensions::<u32>();
  let width = width as usize;
  let height = height as usize;
  let kernel_radius = radius as i32;

  let src = image.rgba();
  let mut current = src.to_vec(); // working buffer for reading
  let mut tmp = vec![0u8; current.len()];
  let row_stride = width * 4;

  // Horizontal pass: read from current, write to tmp (per-row parallel)
  tmp.par_chunks_mut(row_stride).enumerate().for_each(|(y, row_out)| {
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
        let idx_out = x * 4;
        if idx_out + 3 < row_out.len() {
          row_out[idx_out] = (r_sum / count as f32) as u8;
          row_out[idx_out + 1] = (g_sum / count as f32) as u8;
          row_out[idx_out + 2] = (b_sum / count as f32) as u8;
          row_out[idx_out + 3] = (a_sum / count as f32) as u8;
        }
      }
    }
  });

  // swap: tmp (horizontal result) becomes current, reuse tmp as destination for vertical pass
  std::mem::swap(&mut current, &mut tmp);

  // Vertical pass: read from current, write to tmp (per-row parallel)
  tmp.par_chunks_mut(row_stride).enumerate().for_each(|(y, row_out)| {
    for x in 0..width {
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
        let idx_out = x * 4;
        if idx_out + 3 < row_out.len() {
          row_out[idx_out] = (r_sum / count as f32) as u8;
          row_out[idx_out + 1] = (g_sum / count as f32) as u8;
          row_out[idx_out + 2] = (b_sum / count as f32) as u8;
          row_out[idx_out + 3] = (a_sum / count as f32) as u8;
        }
      }
    }
  });

  // Write back the processed result
  image.set_rgba_owned(tmp);
}

/// Applies a box blur to an image.
/// - `p_image`: The image to be blurred.
/// - `p_radius`: The radius of the box blur.
/// - `p_options`: Additional options for applying the blur.
pub fn box_blur<'a>(p_image: impl Into<ImageRef<'a>>, p_radius: impl Into<f64>, p_apply_options: impl Into<Options>) {
  let p_radius = p_radius.into().max(0.0) as u32;
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  apply_filter!(apply_box_blur, image, p_apply_options, p_radius as i32, p_radius);
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Area;
  use abra_core::Image;
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
