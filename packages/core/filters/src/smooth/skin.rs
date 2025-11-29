use abra_core::Image;
use options::Options;
// Local rgb->hsv implementation (small, avoids dependency on primitives crate here)
fn rgb_to_hsv_local(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
  let rf = r as f32 / 255.0;
  let gf = g as f32 / 255.0;
  let bf = b as f32 / 255.0;
  let max = rf.max(gf).max(bf);
  let min = rf.min(gf).min(bf);
  let v = max;
  let d = max - min;
  let s = if max == 0.0 { 0.0 } else { d / max };
  let mut h = 0.0;
  if max != min {
    h = if max == rf {
      (gf - bf) / d + (if gf < bf { 6.0 } else { 0.0 })
    } else if max == gf {
      (bf - rf) / d + 2.0
    } else {
      (rf - gf) / d + 4.0
    };
    h *= 60.0;
  }
  (h, s, v)
}
use crate::{apply_filter};
use crate::blur::surface_blur;

const H_MIN: f32 = 0.0;
const H_MAX: f32 = 50.0;
const S_MIN: f32 = 0.12;
const S_MAX: f32 = 0.68;
const V_MIN: f32 = 0.25;

const FEATHER_PX: u32 = 12;
const RADIUS_PX: u32 = 8;
const SURFACE_THRESHOLD: u8 = 20;

/// Compute a per-pixel HSV-based skin alpha mask for the given image.
/// Returns a Vec<f32> with values in [0.0, 1.0], the same size as the image pixels (w*h).
fn compute_skin_mask_hsv(img: &Image, feather: u32) -> Vec<f32> {
  let (w, h) = img.dimensions::<usize>();
  let rgba = img.rgba();
  let mut mask: Vec<f32> = vec![0.0f32; w * h];

  for y in 0..h {
    for x in 0..w {
      let idx = (y * w + x) * 4;
      let r = rgba[idx];
      let g = rgba[idx + 1];
      let b = rgba[idx + 2];
      let (h_deg, s, v) = rgb_to_hsv_local(r, g, b);
      let is_skin = h_deg >= H_MIN && h_deg <= H_MAX && s >= S_MIN && s <= S_MAX && v >= V_MIN;
      mask[y * w + x] = if is_skin { 1.0 } else { 0.0 };
    }
  }

  if feather > 0 {
    box_blur_f32_inplace(&mut mask, w, h, feather as usize);
    // clamp
    for v in mask.iter_mut() {
      if *v < 0.0 {
        *v = 0.0;
      }
      if *v > 1.0 {
        *v = 1.0;
      }
    }
  }

  mask
}

/// Simple separable box blur on a float mask (in-place). Radius is the feather radius.
fn box_blur_f32_inplace(mask: &mut [f32], width: usize, height: usize, radius: usize) {
  if radius == 0 {
    return;
  }
  let mut tmp = vec![0.0f32; mask.len()];
  let kernel = 2 * radius + 1;

  // Horizontal pass
  for y in 0..height {
    let mut sum = 0.0f32;
    for x in 0..width + radius {
      // add right
      if x < width {
        sum += mask[y * width + x];
      }
      // subtract left
      if x >= kernel {
        sum -= mask[y * width + x - kernel];
      }
      if x >= radius {
        let idx = y * width + (x - radius);
        tmp[idx] = sum / kernel as f32;
      }
    }
  }

  // Vertical pass
  for x in 0..width {
    let mut sum = 0.0f32;
    for y in 0..height + radius {
      if y < height {
        sum += tmp[y * width + x];
      }
      if y >= kernel {
        sum -= tmp[(y - kernel) * width + x];
      }
      if y >= radius {
        let idx = (y - radius) * width + x;
        mask[idx] = sum / kernel as f32;
      }
    }
  }
}

fn apply_smooth_skin(p_image: &mut Image, p_amount: f32) {
  if p_amount <= 0.0 {
    return;
  }

  // Compute a mask for the prepared area (tmp image passed by process_image)
  let feather = FEATHER_PX as usize;
  let mask = compute_skin_mask_hsv(p_image, feather as u32);

  // Make a blurred copy of the prepared tmp image
  let mut blurred = p_image.clone();
  // Use the shared surface blur function (no special options)
  surface_blur(&mut blurred, RADIUS_PX, SURFACE_THRESHOLD, None::<options::ApplyOptions>);

  // Blend blurred -> original based on mask and amount; write back into tmp image
  let w = p_image.dimensions::<usize>().0;
  let h = p_image.dimensions::<usize>().1;
  let orig = p_image.rgba();
  let blurred_pixels = blurred.rgba();

  let mut out = orig.to_vec();
  for y in 0..h {
    for x in 0..w {
      let pi = (y * w + x) * 4;
      let a = (mask[y * w + x] * p_amount).clamp(0.0, 1.0);
      if a <= 0.0 {
        continue;
      }
      let or = orig[pi] as f32;
      let og = orig[pi + 1] as f32;
      let ob = orig[pi + 2] as f32;
      let oa = orig[pi + 3] as f32;

      let br = blurred_pixels[pi] as f32;
      let bg = blurred_pixels[pi + 1] as f32;
      let bb = blurred_pixels[pi + 2] as f32;
      let ba = blurred_pixels[pi + 3] as f32;

      out[pi] = (br * a + or * (1.0 - a)).clamp(0.0, 255.0) as u8;
      out[pi + 1] = (bg * a + og * (1.0 - a)).clamp(0.0, 255.0) as u8;
      out[pi + 2] = (bb * a + ob * (1.0 - a)).clamp(0.0, 255.0) as u8;
      out[pi + 3] = (ba * a + oa * (1.0 - a)).clamp(0.0, 255.0) as u8;
    }
  }

  p_image.set_rgba_owned(out);
}
/// Smooths the skin in the image.
/// - `p_image`: The image to be processed.
/// - `p_amount`: The amount of smoothing to apply (0.0 to 1.0).
/// - `p_options`: Additional options for the smoothing operation.
pub fn smooth_skin(p_image: &mut Image, p_amount: impl Into<f64>, p_options: impl Into<Options>) {
  let amount = p_amount.into().clamp(0.0, 1.0) as f32;
  apply_filter!(apply_smooth_skin, p_image, p_options, 1, amount);
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Image;
  use abra_core::Color;
  use options::ApplyOptions;

  #[test]
  fn compute_skin_mask_hsv_identify_color() {
    // 3x1 image: left black, center skin-like color, right black
    let mut img = Image::new(3, 1);
    img.set_pixel(0, 0, (0, 0, 0, 255));
    img.set_pixel(1, 0, (230, 190, 150, 255)); // skin-like
    img.set_pixel(2, 0, (0, 0, 0, 255));
    let (r, g, b, _a) = img.get_pixel(1, 0).unwrap();
    let hsv = rgb_to_hsv_local(r, g, b);
    eprintln!("hsv center: {:?}", hsv);
    let mask = compute_skin_mask_hsv(&img, 1);
    assert!(mask[1] > 0.8, "center should be detected as skin");
    assert!(mask[0] < 0.2 && mask[2] < 0.2, "sides should not be detected as skin");
  }

  #[test]
  fn compute_skin_mask_hsv_feathering() {
    let mut img = Image::new(3, 1);
    img.set_pixel(0, 0, (0, 0, 0, 255));
    img.set_pixel(1, 0, (230, 190, 150, 255)); // skin-like
    img.set_pixel(2, 0, (0, 0, 0, 255));
    // feathering should cause edge pixel to have fractional alpha > 0
    let mask = compute_skin_mask_hsv(&img, 2);
    assert!(mask[1] > 0.8);
    assert!(mask[0] > 0.0 && mask[0] < 1.0);
    assert!(mask[2] > 0.0 && mask[2] < 1.0);
  }

  #[test]
  fn apply_smooth_skin_only_changes_masked() {
    let mut img = Image::new(3, 1);
    // left and right are white; center is slightly darker skin-like color
    img.set_pixel(0, 0, (255, 255, 255, 255));
    img.set_pixel(1, 0, (230, 190, 150, 255)); // skin
    img.set_pixel(2, 0, (255, 255, 255, 255));
    let orig = img.to_rgba_vec();

    smooth_skin(&mut img, 1.0, ApplyOptions::new());
    let out = img.to_rgba_vec();
    // center should have changed towards neighbors after smoothing
    assert!(out[4] != orig[4] || out[5] != orig[5] || out[6] != orig[6]);
    // outside pixels should remain white (likely unchanged)
    assert_eq!(out[0], orig[0]);
    assert_eq!(out[1], orig[1]);
    assert_eq!(out[2], orig[2]);
  }
}
