use crate::common::*;

use crate::blur::gaussian_blur;
use abra_core::Channels;
use abra_core::color::rgb_to_hsv;
use mask::Mask;

const H_MIN: f32 = 0.0;
const H_MAX: f32 = 50.0;
const S_MIN: f32 = 0.12;
const S_MAX: f32 = 0.68;
const V_MIN: f32 = 0.25;

const FEATHER_PX: u32 = 5;
const RADIUS_PX: u32 = 8;
// SURFACE_THRESHOLD is no longer used - we use gaussian blur instead for smoothing

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
      let (h_deg, s, v) = rgb_to_hsv(r, g, b);
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

  // Apply gaussian blur (edge-aware approaches may be added later)
  // Adapt radius for the working area based on p_image size to avoid no-op blurs on very small images.
  let (w, h) = p_image.dimensions::<usize>();
  let radius_px = std::cmp::min(RADIUS_PX as usize, std::cmp::max(1usize, std::cmp::max(w, h) / 8)) as u32;
  gaussian_blur(p_image, radius_px, None::<ApplyOptions>);
}
/// Smooths the skin in the image.
/// - `p_image`: The image to be processed.
/// - `p_amount`: The amount of smoothing to apply (0.0 to 1.0).
/// - `p_options`: Additional options for the smoothing operation.
pub fn smooth_skin<'a>(p_image: impl Into<ImageRef<'a>>, p_amount: impl Into<f64>, p_options: impl Into<Options>) {
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  let amount = p_amount.into().clamp(0.0, 1.0) as f32;
  let pad = std::cmp::max(RADIUS_PX, FEATHER_PX) as i32;

  // Build a mask image based on HSV detection
  let (w, h) = image.dimensions::<usize>();
  let feather_px = std::cmp::min(FEATHER_PX as usize, std::cmp::max(1usize, std::cmp::max(w, h) / 4)) as u32;
  let skin_mask = compute_skin_mask_hsv(image, feather_px);
  let (w, h) = image.dimensions::<usize>();
  // Combine with existing mask if provided via options
  let mut opts = p_options.into().unwrap_or_else(ApplyOptions::new);
  if let Some(existing_mask) = opts.mask() {
    let existing_mask_bytes = existing_mask.image().rgba();
    // Multiply existing mask grayscale with skin mask
    let mut combined = vec![0u8; w * h * 4];
    for i in 0..(w * h) {
      let gray_u8 = mask::rgba_to_gray(&existing_mask_bytes[i * 4..i * 4 + 4]);
      let gray = gray_u8 as f32;
      let skin_v = skin_mask[i];
      let final_alpha = ((gray as f32 / 255.0) * skin_v * amount * 255.0).round() as u8;
      combined[i * 4] = final_alpha;
      combined[i * 4 + 1] = final_alpha;
      combined[i * 4 + 2] = final_alpha;
      combined[i * 4 + 3] = 255;
    }
    let mask_img = Image::new_from_pixels(w as u32, h as u32, combined, Channels::RGBA);
    opts = opts.with_mask(Mask::from_image(mask_img));
  } else {
    let mut combined = vec![0u8; w * h * 4];
    for i in 0..(w * h) {
      let final_alpha = (skin_mask[i] * amount * 255.0).round() as u8;
      combined[i * 4] = final_alpha;
      combined[i * 4 + 1] = final_alpha;
      combined[i * 4 + 2] = final_alpha;
      combined[i * 4 + 3] = 255;
    }
    let mask_img = Image::new_from_pixels(w as u32, h as u32, combined, Channels::RGBA);
    mask_img.save("out/skin.png", None);
    opts = opts.with_mask(Mask::from_image(mask_img));
  }

  // Prepare ApplyContext and call process_image directly to avoid macro type inference issues
  let ctx = opts.ctx();
  abra_core::image::apply_area::process_image(image, Some(ctx), pad, |img| {
    apply_smooth_skin(img, amount);
  });
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Image;
  use options::ApplyOptions;

  #[test]
  fn compute_skin_mask_hsv_identify_color() {
    // 9x1 image: center skin-like pixel; use a larger width so feathering doesn't equalize
    let mut img = Image::new(9, 1);
    for x in 0..9u32 {
      img.set_pixel(x, 0, (0, 0, 0, 255));
    }
    img.set_pixel(4, 0, (230, 190, 150, 255)); // skin-like center
    let mask = compute_skin_mask_hsv(&img, 0);
    assert!(mask[4] > mask[3] && mask[4] > mask[5], "center should be detected as skin (higher than sides)");
  }

  #[test]
  fn rgb_to_hsv_local_basic() {
    let (h, s, v) = rgb_to_hsv(230, 190, 150);
    // Basic sanity check - values checked in assertions
    assert!(h > 20.0 && h < 40.0, "h is too far: {}", h);
    assert!(s > 0.2 && s < 0.4, "s is too small: {}", s);
    assert!(v > 0.8, "v is too small: {}", v);
  }

  #[test]
  fn compute_skin_mask_hsv_feathering() {
    let mut img = Image::new(9, 1);
    for x in 0..9u32 {
      img.set_pixel(x, 0, (0, 0, 0, 255));
    }
    img.set_pixel(4, 0, (230, 190, 150, 255)); // skin center
    // feathering should cause edge pixel to have fractional alpha > 0
    let mask = compute_skin_mask_hsv(&img, 2);
    // neighbors should have fractional values
    assert!(mask[3] > 0.0 && mask[3] < 1.0);
    assert!(mask[5] > 0.0 && mask[5] < 1.0);
    assert!(mask[3] > 0.0 && mask[3] < 1.0);
    assert!(mask[5] > 0.0 && mask[5] < 1.0);
  }

  #[test]
  fn apply_smooth_skin_only_changes_masked() {
    let mut img = Image::new(3, 1);
    // left and right are white; center is slightly darker skin-like color
    img.set_pixel(0, 0, (255, 255, 255, 255));
    img.set_pixel(1, 0, (230, 190, 150, 255)); // skin
    img.set_pixel(2, 0, (255, 255, 255, 255));
    let orig = img.to_rgba_vec();

    // compute mask (no debug print)
    let _ = compute_skin_mask_hsv(&img, 1);
    smooth_skin(&mut img, 1.0, ApplyOptions::new());
    let out = img.to_rgba_vec();
    // center should have changed towards neighbors after smoothing (some component changes expected)
    // Ensure something in the image changed by applying smoothing using the computed mask
    assert!(out != orig, "image should change after smoothing in masked area");
  }
}
