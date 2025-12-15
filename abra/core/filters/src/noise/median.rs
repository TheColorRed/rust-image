use crate::common::*;
use abra_core::color::Histogram;
use abra_core::{Channels, Resize, if_pick};

/// Naive histogram-based median filter - O(r²) per pixel
/// Good for small radius values (< 8)
fn apply_median_naive(src: &[u8], out: &mut [u8], width: usize, height: usize, radius: u32) {
  let diameter = (radius * 2 + 1) as usize;
  let window_size = (diameter * diameter) as u64;

  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let x = idx % width;
    let y = idx / width;

    let mut r_hist = [0u64; 256];
    let mut g_hist = [0u64; 256];
    let mut b_hist = [0u64; 256];

    for dy in -(radius as isize)..=(radius as isize) {
      for dx in -(radius as isize)..=(radius as isize) {
        let nx = (x as isize + dx).clamp(0, (width - 1) as isize) as usize;
        let ny = (y as isize + dy).clamp(0, (height - 1) as isize) as usize;
        let n_idx = (ny * width + nx) * 4;
        r_hist[src[n_idx] as usize] += 1;
        g_hist[src[n_idx + 1] as usize] += 1;
        b_hist[src[n_idx + 2] as usize] += 1;
      }
    }

    dst_px[0] = Histogram::median_from_hist(&r_hist, window_size);
    dst_px[1] = Histogram::median_from_hist(&g_hist, window_size);
    dst_px[2] = Histogram::median_from_hist(&b_hist, window_size);
    dst_px[3] = src[idx * 4 + 3];
  });
}

/// Sliding window histogram median filter - O(r) per pixel
/// Based on Huang et al. "A fast two-dimensional median filtering algorithm" (1979)
/// Optimized for large radius values (>= 8)
fn apply_median_sliding(src: &[u8], out: &mut [u8], width: usize, height: usize, radius: u32) {
  let r = radius as isize;

  // Process row by row to maintain sliding window
  for y in 0..height {
    let mut r_hist = [0u64; 256];
    let mut g_hist = [0u64; 256];
    let mut b_hist = [0u64; 256];

    // Initialize histogram for first pixel in row
    for dy in -r..=r {
      let ny = ((y as isize) + dy).clamp(0, (height - 1) as isize) as usize;
      for dx in -r..=r {
        let nx = dx.clamp(0, (width - 1) as isize) as usize;
        let idx = (ny * width + nx) * 4;
        r_hist[src[idx] as usize] += 1;
        g_hist[src[idx + 1] as usize] += 1;
        b_hist[src[idx + 2] as usize] += 1;
      }
    }

    let diameter = (radius * 2 + 1) as u64;

    // Process first pixel
    let out_idx = y * width * 4;
    out[out_idx] = Histogram::median_from_hist(&r_hist, diameter * diameter);
    out[out_idx + 1] = Histogram::median_from_hist(&g_hist, diameter * diameter);
    out[out_idx + 2] = Histogram::median_from_hist(&b_hist, diameter * diameter);
    out[out_idx + 3] = src[out_idx + 3];

    // Slide window horizontally across row
    for x in 1..width {
      // Remove leftmost column from histogram
      let left_x = ((x as isize) - r - 1).clamp(0, (width - 1) as isize) as usize;
      for dy in -r..=r {
        let ny = ((y as isize) + dy).clamp(0, (height - 1) as isize) as usize;
        let idx = (ny * width + left_x) * 4;
        r_hist[src[idx] as usize] = r_hist[src[idx] as usize].saturating_sub(1);
        g_hist[src[idx + 1] as usize] = g_hist[src[idx + 1] as usize].saturating_sub(1);
        b_hist[src[idx + 2] as usize] = b_hist[src[idx + 2] as usize].saturating_sub(1);
      }

      // Add rightmost column to histogram
      let right_x = ((x as isize) + r).clamp(0, (width - 1) as isize) as usize;
      for dy in -r..=r {
        let ny = ((y as isize) + dy).clamp(0, (height - 1) as isize) as usize;
        let idx = (ny * width + right_x) * 4;
        r_hist[src[idx] as usize] += 1;
        g_hist[src[idx + 1] as usize] += 1;
        b_hist[src[idx + 2] as usize] += 1;
      }

      // Calculate median and store result
      let out_idx = (y * width + x) * 4;
      out[out_idx] = Histogram::median_from_hist(&r_hist, diameter * diameter);
      out[out_idx + 1] = Histogram::median_from_hist(&g_hist, diameter * diameter);
      out[out_idx + 2] = Histogram::median_from_hist(&b_hist, diameter * diameter);
      out[out_idx + 3] = src[out_idx + 3];
    }
  }
}

/// Downsampled median filter for very large radii
/// Downsamples the image, applies median filter, then upsamples back
/// Trade-off: Speed vs precision, but at radius >150 fine details are lost anyway
fn apply_median_downsampled(p_image: &mut Image, radius: u32) {
  let (width, height) = p_image.dimensions::<u32>();

  // Determine downsampling scale based on radius
  let scale = if_pick!(radius >= 300 => 8, radius >= 200 => 4, else => 2);
  let down_w = (width / scale).max(1);
  let down_h = (height / scale).max(1);
  let scaled_radius = (radius as f32 / scale as f32).max(1.0).round() as u32;

  // Downsample
  let mut tmp_img = Image::new_from_pixels(width, height, p_image.rgba().to_vec(), Channels::RGBA);
  tmp_img.resize(down_w, down_h, None);

  // Apply median filter at scaled resolution
  let src = tmp_img.rgba();
  let mut out = vec![0u8; (down_w * down_h * 4) as usize];

  // Use sliding window for downsampled image (still efficient)
  apply_median_sliding(src, &mut out, down_w as usize, down_h as usize, scaled_radius);
  tmp_img.set_rgba_owned(out);

  // Upsample back to original size
  tmp_img.resize(width, height, None);
  p_image.set_rgba_owned(tmp_img.into_rgba_vec());
}

fn apply_median(p_image: &mut Image, p_radius: f32) {
  let radius = p_radius.max(0.0).round() as u32;
  if radius == 0 {
    return;
  }

  let (width, height) = p_image.dimensions::<usize>();

  // Choose optimal algorithm based on radius size
  match radius {
    // Small radius: Parallel histogram approach wins despite O(r²) per pixel
    // Parallelization overhead is worth it for small windows
    0..=10 => {
      let src = p_image.rgba();
      let mut out = vec![0u8; width * height * 4];
      apply_median_naive(src, &mut out, width, height, radius);
      p_image.set_rgba_owned(out);
    }

    // Large radius: Downsample, filter, then upsample
    // Trade-off: Speed vs precision, but at radius >10 fine details are lost anyway
    _ => {
      apply_median_downsampled(p_image, radius);
    }
  }
}

/// Applies a median filter to the image.
/// - `p_image`: The image to apply the filter to.
/// - `p_radius`: The radius of the median filter.
/// - `p_apply_options`: Options for applying the filter.
pub fn median<'a>(p_image: impl Into<ImageRef<'a>>, p_radius: f32, p_apply_options: impl Into<Options>) {
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  apply_filter!(apply_median, image, p_apply_options, 1, p_radius);
}
