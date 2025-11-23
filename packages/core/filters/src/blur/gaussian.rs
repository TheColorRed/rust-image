use core::{Channels, Image, Resize};

use rayon::prelude::*;
use std::borrow::Cow;
use std::time::Instant;

use options::ApplyOptions;

fn gaussian_kernel_1d(radius: u32) -> Vec<f32> {
  let mut kernel = vec![0.0; (2 * radius + 1) as usize];
  let sigma = radius as f32 / 2.0;
  let pi = std::f32::consts::PI;

  // Fill kernel symmetrically
  for x in 0..=radius {
    let value = (-(x as f32 * x as f32) / (2.0 * sigma * sigma)).exp() / (2.0 * pi * sigma * sigma);
    kernel[radius as usize + x as usize] = value;
    kernel[radius as usize - x as usize] = value;
  }

  // Compute full kernel sum and normalize so the kernel sums to 1.0.
  let sum = kernel.iter().copied().sum::<f32>();
  if sum > 0.0 {
    kernel.iter_mut().for_each(|value| *value /= sum);
  }

  kernel
}

/// Applies a Gaussian blur to an image using separable convolution.
/// Uses two passes: horizontal and vertical for O(r) complexity instead of O(r²).
/// * `p_image` - A mutable reference to the image to be blurred.
/// * `p_radius` - The radius of the Gaussian kernel.
fn separable_gaussian_blur_pixels(pixels: &[u8], width: usize, height: usize, p_radius: u32) -> Vec<u8> {
  let kernel = gaussian_kernel_1d(p_radius);
  let kernel_radius = p_radius as i32;
  // kernel_radius is no longer used here; separable implementation computes its kernel locally.
  let width_i32 = width as i32;
  let height_i32 = height as i32;

  // Preallocate two buffers (horizontal then vertical) to avoid repeated allocations when processing rows.
  let mut horizontal = vec![0u8; width * height * 4];
  let mut vertical = vec![0u8; width * height * 4];

  // Horizontal pass (parallel per-row writing into horizontal buffer)
  horizontal.par_chunks_mut(width * 4).enumerate().for_each(|(y, chunk)| {
    for x in 0..width {
      let mut r = 0.0f32;
      let mut g = 0.0f32;
      let mut b = 0.0f32;
      let mut a = 0.0f32;
      for kx in -kernel_radius..=kernel_radius {
        let px = (x as i32 + kx).clamp(0, width_i32 - 1) as usize;
        let src_idx = (y * width + px) * 4;
        let weight = kernel[(kx + kernel_radius) as usize];
        r += pixels[src_idx] as f32 * weight;
        g += pixels[src_idx + 1] as f32 * weight;
        b += pixels[src_idx + 2] as f32 * weight;
        a += pixels[src_idx + 3] as f32 * weight;
      }
      let rr = r.clamp(0.0, 255.0) as u8;
      let gg = g.clamp(0.0, 255.0) as u8;
      let bb = b.clamp(0.0, 255.0) as u8;
      let aa = a.clamp(0.0, 255.0) as u8;
      let off = x * 4;
      chunk[off] = rr;
      chunk[off + 1] = gg;
      chunk[off + 2] = bb;
      chunk[off + 3] = aa;
    }
  });

  // Vertical pass: read from horizontal buffer, write into vertical buffer
  vertical.par_chunks_mut(width * 4).enumerate().for_each(|(y, chunk)| {
    for x in 0..width {
      let mut r = 0.0f32;
      let mut g = 0.0f32;
      let mut b = 0.0f32;
      let mut a = 0.0f32;
      for ky in -kernel_radius..=kernel_radius {
        let py = (y as i32 + ky).clamp(0, height_i32 - 1) as usize;
        let src_idx = (py * width + x) * 4;
        let weight = kernel[(ky + kernel_radius) as usize];
        r += horizontal[src_idx] as f32 * weight;
        g += horizontal[src_idx + 1] as f32 * weight;
        b += horizontal[src_idx + 2] as f32 * weight;
        a += horizontal[src_idx + 3] as f32 * weight;
      }
      let rr = r.clamp(0.0, 255.0) as u8;
      let gg = g.clamp(0.0, 255.0) as u8;
      let bb = b.clamp(0.0, 255.0) as u8;
      let aa = a.clamp(0.0, 255.0) as u8;
      let off = x * 4;
      chunk[off] = rr;
      chunk[off + 1] = gg;
      chunk[off + 2] = bb;
      chunk[off + 3] = aa;
    }
  });

  vertical
}

pub fn gaussian_blur(p_image: &mut Image, p_radius: u32, options: impl Into<Option<ApplyOptions>>) {
  if p_radius == 0 {
    return;
  }
  let start = std::time::Instant::now();
  let _duration = Instant::now();
  let kernel_radius = p_radius as i32;
  let (image_w, image_h) = p_image.dimensions::<u32>();
  let image_w = image_w as i32;
  let image_h = image_h as i32;
  let options = options.into();
  let area = options.as_ref().and_then(|o| o.area());
  let feather_amount = match &area {
    Some(a) => a.feather() as i32,
    None => 0,
  };

  // Get RGBA pixels
  // If an area is provided, restrict processing to that bounding box. Otherwise process the whole image.
  let (pixels_cow, area_min_x, area_min_y, area_w, area_h, rect_min_x, rect_min_y, rect_w, rect_h) = match &area {
    Some(a) => {
      let (mut min_x, mut min_y, mut max_x, mut max_y) = a.bounds::<i32>();
      if min_x < 0 {
        min_x = 0;
      }
      if min_y < 0 {
        min_y = 0;
      }
      if max_x > image_w {
        max_x = image_w;
      }
      if max_y > image_h {
        max_y = image_h;
      }
      if min_x >= max_x || min_y >= max_y {
        return;
      }
      // Expand processing region by kernel radius so we have correct neighbors for convolution.
      let rect_min_x = (min_x - kernel_radius).max(0);
      let rect_min_y = (min_y - kernel_radius).max(0);
      let rect_max_x = (max_x + kernel_radius).min(image_w);
      let rect_max_y = (max_y + kernel_radius).min(image_h);
      let rect_w = (rect_max_x - rect_min_x) as usize;
      let rect_h = (rect_max_y - rect_min_y) as usize;
      // Extract a rectangular slice of the image pixels covering the processing region (no cloning full image).
      let rgba = p_image.rgba_slice();
      let row_stride = (image_w * 4) as usize;
      let mut pixels: Vec<u8> = Vec::with_capacity(rect_w * rect_h * 4);
      for ry in rect_min_y..rect_max_y {
        let start = (ry as usize * row_stride) + (rect_min_x as usize * 4);
        let end = start + (rect_w * 4);
        pixels.extend_from_slice(&rgba[start..end]);
      }
      (
        Cow::Owned(pixels),
        min_x as i32,
        min_y as i32,
        (max_x - min_x) as i32,
        (max_y - min_y) as i32,
        rect_min_x as i32,
        rect_min_y as i32,
        rect_w as i32,
        rect_h as i32,
      )
    }
    None => {
      (Cow::Borrowed(p_image.rgba_slice()), 0, 0, image_w as i32, image_h as i32, 0, 0, image_w as i32, image_h as i32)
    }
  };
  let pixels: Cow<[u8]> = pixels_cow;
  let width = rect_w as usize;
  let height = rect_h as usize;

  // If radius is very large and area is sufficiently large, downsample and approximate
  let vertical = if p_radius >= 24
    && options.is_some()
    && (area_w * area_h) as i64 > (image_w as i64 * image_h as i64 / 4)
    && (width >= 128 || height >= 128)
  {
    // choose a scale that reduces the radius to a reasonable size
    let scale = if p_radius >= 96 {
      8
    } else if p_radius >= 48 {
      4
    } else {
      2
    };
    let down_w = (width / scale).max(1) as u32;
    let down_h = (height / scale).max(1) as u32;

    // Build a temporary sub-image and downscale
    let mut tmp_img = Image::new_from_pixels(width as u32, height as u32, pixels.to_vec(), Channels::RGBA);
    tmp_img.resize(down_w, down_h, None);
    let new_radius = (p_radius as f32 / scale as f32).max(1.0).round() as u32;

    // Apply separable gaussian on the small image (no area), this is faster because of far fewer pixels.
    let blurred_small =
      separable_gaussian_blur_pixels(tmp_img.rgba_slice(), down_w as usize, down_h as usize, new_radius);
    tmp_img.set_rgba(blurred_small);

    // Upscale back to original processing size
    tmp_img.resize(width as u32, height as u32, None);
    tmp_img.rgba()
  } else {
    separable_gaussian_blur_pixels(&pixels, width, height, p_radius)
  };

  // Write back result: if we processed a sub-area, write back only masked pixels (area/feather) into destination.
  if area_min_x == 0 && area_min_y == 0 && area_w == image_w && area_h == image_h && feather_amount == 0 {
    p_image.set_rgba(vertical);
  } else if area_min_x == 0 && area_min_y == 0 && area_w == image_w && area_h == image_h && feather_amount > 0 {
    // Feather on full image is equivalent to full blur for now.
    p_image.set_rgba(vertical);
  } else {
    // area existing: we wrote proc_min_x/proc_min_y as processing offsets; proc_w/proc_h is the area bounds (not kernel expanded)
    // Convert vertical to per-pixel and blend into p_image only for area mask.
    let mut rgba = p_image.rgba();
    let _row_stride = (image_w * 4) as usize;
    let feather = feather_amount as f32;

    // Build mask of alpha values for write area: alpha 0 outside area, 1 inside, gradient inside feather.
    // Allocate the mask with exact length, not just capacity, otherwise index access will panic.
    let mut mask: Vec<f32> = vec![0.0f32; width * height];
    // Compute mask in parallel for large areas which are costly to compute serially
    mask.par_chunks_mut(width).enumerate().for_each(|(py, chunk)| {
      for px in 0..chunk.len() {
        let gx = (rect_min_x as i32 + px as i32) as f32 + 0.5;
        let gy = (rect_min_y as i32 + py as i32) as f32 + 0.5;
        let v = if let Some(ref a) = area {
          if a.contains((gx, gy)) {
            if feather_amount > 0 {
              let closest = a.path.closest_point(gx, gy);
              let d = ((gx - closest.x).powi(2) + (gy - closest.y).powi(2)).sqrt();
              (d as f32 / feather as f32).clamp(0.0, 1.0)
            } else {
              1.0
            }
          } else {
            0.0
          }
        } else {
          1.0
        };
        chunk[px] = v;
      }
    });
    // Debugging: detect small test to print mask and outputs for diagnostics
    // Mask computed for area blending
    // Debug: compare vertical output vs original region before blending
    // orig_slice debug removed
    // masked vertical diffs loop removed
    // masked vertical diffs debug removed

    // Blend vertically result into original using mask and write into p_image.
    // mask_count removed
    // mask_count debug removed
    // mask_count debug removed
    let out_pixels = &vertical;
    // out_pixels now contains vertically blurred rectangle (rect_min_x..rect_max_x, rect_min_y..rect_max_y)
    rgba
      .par_chunks_mut((image_w * 4) as usize)
      .enumerate()
      .for_each(|(row_y, row)| {
        // row_y is in the full image coordinates; only process when it falls inside rect vertical range
        if row_y < rect_min_y as usize || row_y >= (rect_min_y + rect_h as i32) as usize {
          return;
        }
        let py = row_y - rect_min_y as usize; // convert into rect-local y
        for px in 0..(width as usize) {
          let m_idx = (py as usize * width + px as usize) as usize;
          let alpha = mask[m_idx];
          if alpha <= 0.0 {
            continue;
          }
          let out_idx = (py * width + px) * 4;
          let global_x = rect_min_x as usize + px;
          let global_y = rect_min_y as usize + py as usize;
          if global_x >= image_w as usize || global_y >= image_h as usize {
            continue;
          }
          let br = out_pixels[out_idx] as f32;
          let bg = out_pixels[out_idx + 1] as f32;
          let bb = out_pixels[out_idx + 2] as f32;
          let ba = out_pixels[out_idx + 3] as f32;
          let or = row[global_x * 4] as f32;
          let og = row[global_x * 4 + 1] as f32;
          let ob = row[global_x * 4 + 2] as f32;
          let oa = row[global_x * 4 + 3] as f32;
          let fr = (br * alpha + or * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
          let fg = (bg * alpha + og * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
          let fb = (bb * alpha + ob * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
          let fa = (ba * alpha + oa * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
          row[global_x * 4] = fr;
          row[global_x * 4 + 1] = fg;
          row[global_x * 4 + 2] = fb;
          row[global_x * 4 + 3] = fa;
        }
      });
    p_image.set_rgba(rgba);
  }
  println!("Gaussian blur took: {:?}", start.elapsed());
  // DebugFilters::GaussianBlur(radius as f32, duration.elapsed()).log();
}

#[cfg(test)]
mod tests {
  use options::ApplyOptions;

  use super::gaussian_blur;
  use core::{Area, Image};

  #[test]
  fn gaussian_blur_area_writes_back_only_area() {
    let mut img = Image::new(8, 8);
    // Single bright pixel in center
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 0u8, 0u8, 255));
    // Snapshot original values
    let orig = img.rgba();

    // Apply blur to center 4x4 area (white pixel should spread)
    gaussian_blur(&mut img, 2, ApplyOptions::new().with_area(Area::rect((2.0, 2.0), (4.0, 4.0))));

    // Ensure dimensions unchanged
    assert_eq!(img.dimensions::<u32>(), (8, 8));

    // Check outside area unchanged
    let mut changed_count = 0usize;
    for y in 0..8u32 {
      for x in 0..8u32 {
        let idx = ((y * 8 + x) * 4) as usize;
        if x < 2 || x >= 6 || y < 2 || y >= 6 {
          // Outside area should remain unchanged
          assert_eq!(img.rgba()[idx], orig[idx]);
          assert_eq!(img.rgba()[idx + 1], orig[idx + 1]);
          assert_eq!(img.rgba()[idx + 2], orig[idx + 2]);
          assert_eq!(img.rgba()[idx + 3], orig[idx + 3]);
        } else {
          // Count changes in blurred region; at least one pixel should differ
          if img.rgba()[idx] != orig[idx]
            || img.rgba()[idx + 1] != orig[idx + 1]
            || img.rgba()[idx + 2] != orig[idx + 2]
            || img.rgba()[idx + 3] != orig[idx + 3]
          {
            changed_count += 1;
          }
        }
      }
    }
    // debug removed
    let inside_idx = ((2 * 8 + 2) * 4) as usize;
    println!("inside orig={} new={}", orig[inside_idx], img.rgba()[inside_idx]);
    let outside_idx = 0usize;
    println!("outside orig={} new={}", orig[outside_idx], img.rgba()[outside_idx]);
    assert!(changed_count > 0, "No pixels in the blurred area changed");
  }

  #[test]
  fn separable_blur_changes_pixels() {
    let mut img = Image::new(8, 8);
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 0u8, 0u8, 255));
    let pixels = img.rgba();
    let out = super::separable_gaussian_blur_pixels(&pixels, 8, 8, 2);
    // Ensure center changed
    let idx = ((2 * 8 + 2) * 4) as usize;
    assert!(out[idx] != pixels[idx] || out[idx + 1] != pixels[idx + 1] || out[idx + 2] != pixels[idx + 2]);
  }

  #[test]
  fn horizontal_pass_changes_pixels() {
    let mut img = Image::new(8, 8);
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 0u8, 0u8, 255));
    let pixels = img.rgba();
    let kernel = super::gaussian_kernel_1d(2);
    let width = 8usize;
    let y = 3usize;
    let mut horiz = vec![0u8; width * 4];
    let kernel_radius = 2i32;
    for x in 0..width {
      let mut r = 0.0f32;
      let mut g = 0.0f32;
      let mut b = 0.0f32;
      let mut a = 0.0f32;
      for kx in -kernel_radius..=kernel_radius {
        let px = (x as i32 + kx).clamp(0, width as i32 - 1) as usize;
        let src_idx = (y * width + px) * 4;
        let weight = kernel[(kx + kernel_radius) as usize];
        r += pixels[src_idx] as f32 * weight;
        g += pixels[src_idx + 1] as f32 * weight;
        b += pixels[src_idx + 2] as f32 * weight;
        a += pixels[src_idx + 3] as f32 * weight;
      }
      let rr = r.clamp(0.0, 255.0) as u8;
      let gg = g.clamp(0.0, 255.0) as u8;
      let bb = b.clamp(0.0, 255.0) as u8;
      let aa = a.clamp(0.0, 255.0) as u8;
      let off = x * 4;
      horiz[off] = rr;
      horiz[off + 1] = gg;
      horiz[off + 2] = bb;
      horiz[off + 3] = aa;
    }
    let idx = (3 * 4) as usize;
    assert!(horiz[idx] != pixels[idx] || horiz[idx + 1] != pixels[idx + 1] || horiz[idx + 2] != pixels[idx + 2]);
  }

  #[test]
  fn vertical_pass_changes_pixels() {
    let mut img = Image::new(8, 8);
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 0u8, 0u8, 255));
    let pixels = img.rgba();
    let kernel = super::gaussian_kernel_1d(2);
    let width = 8usize;
    let height = 8usize;
    let kernel_radius = 2i32;
    // Horizontal
    let mut horizontal = vec![0u8; width * height * 4];
    for y in 0..height {
      for x in 0..width {
        let mut r = 0.0f32;
        let mut g = 0.0f32;
        let mut b = 0.0f32;
        let mut a = 0.0f32;
        for kx in -kernel_radius..=kernel_radius {
          let px = (x as i32 + kx).clamp(0, width as i32 - 1) as usize;
          let src_idx = (y * width + px) * 4;
          let weight = kernel[(kx + kernel_radius) as usize];
          r += pixels[src_idx] as f32 * weight;
          g += pixels[src_idx + 1] as f32 * weight;
          b += pixels[src_idx + 2] as f32 * weight;
          a += pixels[src_idx + 3] as f32 * weight;
        }
        let idx = (y * width + x) * 4;
        horizontal[idx] = r.clamp(0.0, 255.0) as u8;
        horizontal[idx + 1] = g.clamp(0.0, 255.0) as u8;
        horizontal[idx + 2] = b.clamp(0.0, 255.0) as u8;
        horizontal[idx + 3] = a.clamp(0.0, 255.0) as u8;
      }
    }
    // Vertical
    let mut vertical = vec![0u8; width * height * 4];
    for y in 0..height {
      for x in 0..width {
        let mut r = 0.0f32;
        let mut g = 0.0f32;
        let mut b = 0.0f32;
        let mut a = 0.0f32;
        for ky in -kernel_radius..=kernel_radius {
          let py = (y as i32 + ky).clamp(0, height as i32 - 1) as usize;
          let src_idx = (py * width + x) * 4;
          let weight = kernel[(ky + kernel_radius) as usize];
          r += horizontal[src_idx] as f32 * weight;
          g += horizontal[src_idx + 1] as f32 * weight;
          b += horizontal[src_idx + 2] as f32 * weight;
          a += horizontal[src_idx + 3] as f32 * weight;
        }
        let idx = (y * width + x) * 4;
        vertical[idx] = r.clamp(0.0, 255.0) as u8;
        vertical[idx + 1] = g.clamp(0.0, 255.0) as u8;
        vertical[idx + 2] = b.clamp(0.0, 255.0) as u8;
        vertical[idx + 3] = a.clamp(0.0, 255.0) as u8;
      }
    }
    let idx = ((2 * 8 + 2) * 4) as usize;
    assert!(
      vertical[idx] != pixels[idx] || vertical[idx + 1] != pixels[idx + 1] || vertical[idx + 2] != pixels[idx + 2]
    );
  }
}
