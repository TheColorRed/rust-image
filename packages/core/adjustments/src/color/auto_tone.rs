use abra_core::{Histogram, Image};
use options::Options;

use rayon::prelude::*;

use crate::apply_adjustment;

pub fn apply_auto_tone(p_image: &mut Image) {
  let (width, height) = p_image.dimensions::<i32>();
  let src = p_image.rgba();
  let mut out = vec![0u8; (width * height * 4) as usize];

  // Parameters: default to 0.5% clip per tail (0.005)
  let clip_fraction: f32 = 0.005;

  // Compute histogram skipping fully-transparent pixels
  let hist = Histogram::from_image_skip_transparent(p_image);

  // Build per-channel levels LUTs using histogram helpers
  let lut_r = hist.red_levels_lut(clip_fraction);
  let lut_g = hist.green_levels_lut(clip_fraction);
  let lut_b = hist.blue_levels_lut(clip_fraction);

  // Apply the per-channel LUT transform in parallel
  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let i = idx * 4;
    let a = src[i + 3];
    if a == 0 {
      // Preserve fully transparent pixel
      dst_px[0] = src[i];
      dst_px[1] = src[i + 1];
      dst_px[2] = src[i + 2];
      dst_px[3] = a;
      return;
    }
    let r = src[i];
    let g = src[i + 1];
    let b = src[i + 2];
    dst_px[0] = lut_r[r as usize];
    dst_px[1] = lut_g[g as usize];
    dst_px[2] = lut_b[b as usize];
    dst_px[3] = a;
  });
  p_image.set_rgba(&out);
}

pub fn auto_tone(p_image: &mut Image, p_options: impl Into<Options>) {
  apply_adjustment!(apply_auto_tone, p_image, p_options, 1);
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Color;

  #[test]
  fn auto_tone_stretches_channels() {
    // Create an image with 100 pixels: 50 at 10, 50 at 200 on the red channel
    let mut img = Image::new(10u32, 10u32);
    // Fill with low red (10) and later half with high red (200)
    for y in 0..5 {
      for x in 0..10 {
        img.set_pixel(x, y, (10u8, 10u8, 10u8, 255u8));
      }
    }
    for y in 5..10 {
      for x in 0..10 {
        img.set_pixel(x, y, (200u8, 200u8, 200u8, 255u8));
      }
    }
    // Apply auto tone - should stretch channel endpoints to 0 and 255
    auto_tone(&mut img, None);
    let (r1, _g1, _b1, _) = img.get_pixel(0, 0).unwrap();
    let (r2, _g2, _b2, _) = img.get_pixel(0, 9).unwrap();
    assert!(r1 <= 5, "low value not mapped to near 0: {}", r1);
    assert!(r2 >= 250, "high value not mapped to near 255: {}", r2);
  }
}
