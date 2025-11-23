use core::{Channels, Image, Resize, pick};

use rayon::prelude::*;
use std::time::Instant;

use core::image::apply_area::{PreparedArea, apply_processed_pixels_to_image, prepare_area_pixels};
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

  // Prepare pixel buffer for processing using helper functions (handles area/rect/expansion)
  let prepared: PreparedArea = prepare_area_pixels(p_image, area, kernel_radius);
  // If area is empty (no pixels), early-return
  if prepared.area_w == 0 || prepared.area_h == 0 {
    return;
  }
  let pixels: &[u8] = prepared.pixels.as_ref();
  let width = prepared.rect_w as usize;
  let height = prepared.rect_h as usize;
  let prepared_meta = prepared.meta();

  // If radius is very large and area is sufficiently large, downsample and approximate
  let vertical = if p_radius >= 24
    && options.is_some()
    && (prepared.area_w as i64 * prepared.area_h as i64) > (image_w as i64 * image_h as i64 / 4)
    && (width >= 128 || height >= 128)
  {
    // choose a scale that reduces the radius to a reasonable size
    let scale = pick!(p_radius >= 96 => 8, p_radius >= 48 => 4, else => 2);
    let down_w = (width / scale).max(1) as u32;
    let down_h = (height / scale).max(1) as u32;

    // Build a temporary sub-image and downscale
    let mut tmp_img = Image::new_from_pixels(width as u32, height as u32, pixels.to_vec(), Channels::RGBA);
    tmp_img.resize(down_w, down_h, None);
    let new_radius = (p_radius as f32 / scale as f32).max(1.0).round() as u32;

    // Apply separable gaussian on the small image (no area), this is faster because of far fewer pixels.
    let blurred_small = separable_gaussian_blur_pixels(tmp_img.rgba(), down_w as usize, down_h as usize, new_radius);
    tmp_img.set_rgba_owned(blurred_small);

    // Upscale back to original processing size
    tmp_img.resize(width as u32, height as u32, None);
    tmp_img.into_rgba_vec()
  } else {
    separable_gaussian_blur_pixels(&pixels, width, height, p_radius)
  };

  // Write back results using helper that handles fast-path and blend
  let mask_img_bytes: Option<&[u8]> = options.as_ref().and_then(|o| o.mask().map(|m| m.image().rgba()));
  apply_processed_pixels_to_image(p_image, vertical, &prepared_meta, area, mask_img_bytes);
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
    let orig = img.to_rgba_vec();

    // Apply blur to center 4x4 area (white pixel should spread)
    gaussian_blur(&mut img, 2, ApplyOptions::new().with_area(Area::rect((2.0, 2.0), (4.0, 4.0))));

    // Ensure dimensions unchanged
    assert_eq!(img.dimensions::<u32>(), (8, 8));

    // Check outside area unchanged
    let mut changed_count = 0usize;
    // Diagnostic code removed
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
    let pixels = img.to_rgba_vec();
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
    let pixels = img.to_rgba_vec();
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
    let pixels = img.to_rgba_vec();
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
