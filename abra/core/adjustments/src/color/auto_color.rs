use abra_core::{Histogram, Image, image::image_ext::ImageRef, lab_to_rgb, rgb_to_lab};
use options::Options;

use rayon::prelude::*;

use crate::apply_adjustment;

fn apply_auto_color(p_image: &mut Image) {
  let (width, height) = p_image.dimensions::<i32>();
  let src = p_image.rgba();
  let mut out = vec![0u8; (width * height * 4) as usize];

  // Algorithm parameters
  let clip_fraction: f32 = 0.005; // 0.5% per tail
  let midtone_low: f32 = 0.3;
  let midtone_high: f32 = 0.7;
  let neutralize_intensity: f32 = 1.0;

  // Helper to compute percentiles (low/high) per channel from histogram
  // Compute histogram using helper struct.
  let hist = Histogram::from_image_skip_transparent(p_image);

  // Use histogram helpers to compute channel clip bounds
  // Clip bounds still available via Histogram helpers, but to compute mapping we use LUTs
  let _ = hist.red_clip_bounds(clip_fraction);
  let _ = hist.green_clip_bounds(clip_fraction);
  let _ = hist.blue_clip_bounds(clip_fraction);

  // Second pass: compute midtone mean a & b (Lab) using post-levels LUTs.
  let lut_r = hist.red_levels_lut(clip_fraction);
  let lut_g = hist.green_levels_lut(clip_fraction);
  let lut_b = hist.blue_levels_lut(clip_fraction);

  let (sum_a, sum_b_lab, midtone_count) = src
    .par_chunks(4)
    .map(|px| {
      if px[3] == 0 {
        return (0.0f64, 0.0f64, 0u64);
      }
      let r_u8 = lut_r[px[0] as usize];
      let g_u8 = lut_g[px[1] as usize];
      let b_u8 = lut_b[px[2] as usize];
      let r = (r_u8 as f32) / 255.0;
      let g = (g_u8 as f32) / 255.0;
      let b = (b_u8 as f32) / 255.0;
      let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
      // exclude very dark or bright or clipped pixels
      if lum < midtone_low || lum > midtone_high {
        return (0.0f64, 0.0f64, 0u64);
      }
      // skip saturated pixels (one channel full black/white in mapped space)
      if (r <= 0.001 || g <= 0.001 || b <= 0.001) || (r >= 0.999 || g >= 0.999 || b >= 0.999) {
        return (0.0f64, 0.0f64, 0u64);
      }
      let (_l_lab, a_lab, b_lab) = rgb_to_lab(r_u8, g_u8, b_u8);
      (a_lab as f64, b_lab as f64, 1u64)
    })
    .reduce(|| (0.0f64, 0.0f64, 0u64), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));

  let mean_a = if midtone_count > 0 {
    sum_a / (midtone_count as f64)
  } else {
    0.0
  };
  let mean_b_c = if midtone_count > 0 {
    sum_b_lab / (midtone_count as f64)
  } else {
    0.0
  };

  // If there are very few midtone samples we avoid neutralizing (prevents noise)
  let min_midtone_samples: u64 = 5;
  let perform_neutralize = midtone_count >= min_midtone_samples;

  // Third pass: produce final pixels with levels (via LUT) and neutralization applied
  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let i = idx * 4;
    let a = src[i + 3];
    let r = src[i];
    let g = src[i + 1];
    let b = src[i + 2];

    // Apply per-channel stretch via LUT
    let r_u8 = lut_r[r as usize];
    let g_u8 = lut_g[g as usize];
    let b_u8 = lut_b[b as usize];

    let (l_lab, mut a_lab, mut b_lab) = rgb_to_lab(r_u8, g_u8, b_u8);
    // Compute luminance in [0,1] for this pixel so we can apply neutralization
    let r_m = (r_u8 as f32) / 255.0;
    let g_m = (g_u8 as f32) / 255.0;
    let b_m = (b_u8 as f32) / 255.0;
    let lum = 0.2126 * r_m + 0.7152 * g_m + 0.0722 * b_m;
    // Apply neutralization only for midtones; optionally ramp at edges
    let mut applied_a_lab = a_lab as f64;
    let mut applied_b_lab = b_lab as f64;
    if perform_neutralize && neutralize_intensity > 0.0 {
      // Compute a linear weight between midtone_low..midtone_high
      let denom = (midtone_high - midtone_low).max(1e-6);
      let mut w = (((lum as f64) - (midtone_low as f64)) / (denom as f64)).clamp(0.0f64, 1.0f64);
      // Ramp using a smoothstep for softer transitions
      w = w * w * (3.0 - 2.0 * w);
      if w > 0.0 {
        applied_a_lab = a_lab as f64 - mean_a * (neutralize_intensity as f64) * w;
        applied_b_lab = b_lab as f64 - mean_b_c * (neutralize_intensity as f64) * w;
      }
    }
    a_lab = applied_a_lab as f32;
    b_lab = applied_b_lab as f32;
    let (nr, ng, nb) = lab_to_rgb(l_lab, a_lab, b_lab);
    // no test-time debug printing
    dst_px[0] = nr;
    dst_px[1] = ng;
    dst_px[2] = nb;
    dst_px[3] = a;
  });
  p_image.set_rgba(&out);
}
/// Applies an auto color adjustment to the image.
/// This function analyzes the image's histogram to perform
/// levels stretching and color neutralization.
/// - `p_image`: The image to adjust.
/// - `p_options`: Options to apply the adjustment.
pub fn auto_color<'a>(p_image: impl Into<ImageRef<'a>>, p_options: impl Into<Options>) {
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  apply_adjustment!(apply_auto_color, image, p_options, 1);
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Color;

  #[test]
  fn dark_pixels_remain_dark_after_auto_color() {
    let mut img = Image::new(5u32, 5u32);
    // Fill with near-black color that has a slight green cast. If we neutralize
    // midtones incorrectly, these pixels could become tinted. They should remain dark.
    img.clear_color(Color::from_rgba(6, 4, 5, 255));
    // Apply auto color with default options
    auto_color(&mut img, None);
    let (r, g, b, _a) = img.get_pixel(2, 2).unwrap();
    // Assert maximum channel remains small (no hue pops)
    assert!(r <= 12 && g <= 12 && b <= 12, "Dark pixel got too bright: {},{},{}", r, g, b);
  }
  #[test]
  fn debug_lab_roundtrip() {
    // Sanity check: converting a near-black value to Lab and back should produce a small color
    let (r, g, b) = (6u8, 4u8, 5u8);
    let (l, a, b_) = rgb_to_lab(r, g, b);
    let (rr, gg, bb) = lab_to_rgb(l, a, b_);
    eprintln!("DEBUG LAB RT input=({},{},{}) lab=({:.3},{:.3},{:.3}) out=({},{},{})", r, g, b, l, a, b_, rr, gg, bb);
    // should remain near original
    assert!(rr <= 16 && gg <= 16 && bb <= 16, "Roundtrip produced too bright color");
  }
}
