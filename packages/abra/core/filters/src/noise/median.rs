use crate::common::*;
use abra_core::Bins;
use abra_core::{Channels, ResizeTarget, Size, Transform, if_pick};

/// Naive histogram-based median filter - O(r²) per pixel
/// Good for small radius values (< 8)
fn apply_median_naive(p_src: &[u8], p_out: &mut [u8], p_width: usize, p_height: usize, p_radius: u32) {
  p_out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let x = idx % p_width;
    let y = idx / p_width;

    let mut r_hist = Bins::new();
    let mut g_hist = Bins::new();
    let mut b_hist = Bins::new();

    for dy in -(p_radius as isize)..=(p_radius as isize) {
      for dx in -(p_radius as isize)..=(p_radius as isize) {
        let nx = (x as isize + dx).clamp(0, (p_width - 1) as isize) as usize;
        let ny = (y as isize + dy).clamp(0, (p_height - 1) as isize) as usize;
        let n_idx = (ny * p_width + nx) * 4;
        r_hist.add(p_src[n_idx]);
        g_hist.add(p_src[n_idx + 1]);
        b_hist.add(p_src[n_idx + 2]);
      }
    }

    dst_px[0] = r_hist.median();
    dst_px[1] = g_hist.median();
    dst_px[2] = b_hist.median();
    dst_px[3] = p_src[idx * 4 + 3];
  });
}

/// Sliding window histogram median filter - O(r) per pixel
/// Based on Huang et al. "A fast two-dimensional median filtering algorithm" (1979)
/// Optimized for large radius values (>= 8)
fn apply_median_sliding(p_src: &[u8], p_out: &mut [u8], p_width: usize, p_height: usize, p_radius: u32) {
  let r = p_radius as isize;

  // Process row by row to maintain sliding window
  for y in 0..p_height {
    let mut r_hist = Bins::new();
    let mut g_hist = Bins::new();
    let mut b_hist = Bins::new();

    // Initialize histogram for first pixel in row
    for dy in -r..=r {
      let ny = ((y as isize) + dy).clamp(0, (p_height - 1) as isize) as usize;
      for dx in -r..=r {
        let nx = dx.clamp(0, (p_width - 1) as isize) as usize;
        let idx = (ny * p_width + nx) * 4;
        r_hist.add(p_src[idx]);
        g_hist.add(p_src[idx + 1]);
        b_hist.add(p_src[idx + 2]);
      }
    }

    // Process first pixel
    let out_idx = y * p_width * 4;
    p_out[out_idx] = r_hist.median();
    p_out[out_idx + 1] = g_hist.median();
    p_out[out_idx + 2] = b_hist.median();
    p_out[out_idx + 3] = p_src[out_idx + 3];

    // Slide window horizontally across row
    for x in 1..p_width {
      // Remove leftmost column from histogram
      let left_x = ((x as isize) - r - 1).clamp(0, (p_width - 1) as isize) as usize;
      for dy in -r..=r {
        let ny = ((y as isize) + dy).clamp(0, (p_height - 1) as isize) as usize;
        let idx = (ny * p_width + left_x) * 4;
        r_hist.remove(p_src[idx]);
        g_hist.remove(p_src[idx + 1]);
        b_hist.remove(p_src[idx + 2]);
      }

      // Add rightmost column to histogram
      let right_x = ((x as isize) + r).clamp(0, (p_width - 1) as isize) as usize;
      for dy in -r..=r {
        let ny = ((y as isize) + dy).clamp(0, (p_height - 1) as isize) as usize;
        let idx = (ny * p_width + right_x) * 4;
        r_hist.add(p_src[idx]);
        g_hist.add(p_src[idx + 1]);
        b_hist.add(p_src[idx + 2]);
      }

      // Calculate median and store result
      let out_idx = (y * p_width + x) * 4;
      p_out[out_idx] = r_hist.median();
      p_out[out_idx + 1] = g_hist.median();
      p_out[out_idx + 2] = b_hist.median();
      p_out[out_idx + 3] = p_src[out_idx + 3];
    }
  }
}

/// Downsampled median filter for very large radii
/// Downsamples the image, applies median filter, then upsamples back
/// Trade-off: Speed vs precision, but at radius >150 fine details are lost anyway
fn apply_median_downsampled(p_image: &mut Image, p_radius: u32) {
  let (width, height) = p_image.dimensions::<u32>();

  // Determine downsampling scale based on radius
  let scale = if_pick!(p_radius >= 300 => 8, p_radius >= 200 => 4, else => 2);
  let down_w = (width / scale).max(1);
  let down_h = (height / scale).max(1);
  let scaled_radius = (p_radius as f32 / scale as f32).max(1.0).round() as u32;

  // Downsample
  let mut tmp_img = Image::new_from_pixels(width, height, p_image.rgba().to_vec(), Channels::RGBA);
  tmp_img.resize(ResizeTarget::Exact(Size::new(down_w, down_h)), None);

  // Apply median filter at scaled resolution
  let src = tmp_img.rgba();
  let mut out = vec![0u8; (down_w * down_h * 4) as usize];

  // Use sliding window for downsampled image (still efficient)
  apply_median_sliding(src, &mut out, down_w as usize, down_h as usize, scaled_radius);
  tmp_img.set_rgba(out);

  // Upsample back to original size
  tmp_img.resize(ResizeTarget::Exact(Size::new(width, height)), None);
  p_image.set_rgba(tmp_img.into_rgba_vec());
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
      p_image.set_rgba(out);
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
#[derive(Clone)]
pub struct Median {
  radius: f32,
  options: Options,
}
impl Effect for Median {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }
  fn padding(&self) -> i32 {
    self.radius.max(0.0).round() as i32
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_median(p_image, self.radius);
  }
}
pub fn median(p_radius: f32) -> Median {
  Median {
    radius: p_radius,
    options: None,
  }
}
