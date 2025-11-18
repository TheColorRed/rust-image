use rayon::prelude::*;

use crate::image::Image;

/// Applies a surface blur to an image.
///
/// - `radius`: size of the square neighborhood in pixels (>= 1)
/// - `threshold`: inclusive difference threshold [1..=255].
///   Neighboring pixels whose per-channel absolute difference from the center
///   exceeds this threshold are excluded from the average.
pub fn surface_blur(image: &mut Image, radius: u32, threshold: u8) {
  if radius == 0 {
    return;
  }

  let (width, height) = image.dimensions::<u32>();
  if width == 0 || height == 0 {
    return;
  }

  let src = image.rgba();
  let (w, h) = (width as usize, height as usize);
  let mut out = vec![0u8; w * h * 4];

  let r = radius as i32;
  let thr = threshold as i32;

  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst)| {
    let x = (idx % w) as i32;
    let y = (idx / w) as i32;
    let center_idx = idx * 4;
    let cr = src[center_idx] as i32;
    let cg = src[center_idx + 1] as i32;
    let cb = src[center_idx + 2] as i32;
    let ca = src[center_idx + 3] as i32;

    let mut sum_r = 0i32;
    let mut sum_g = 0i32;
    let mut sum_b = 0i32;
    let mut sum_a = 0i32;
    let mut count = 0i32;

    for dy in -r..=r {
      let ny = (y + dy).clamp(0, h as i32 - 1);
      for dx in -r..=r {
        let nx = (x + dx).clamp(0, w as i32 - 1);
        let nidx = ((ny as usize) * w + nx as usize) * 4;

        let nr = src[nidx] as i32;
        let ng = src[nidx + 1] as i32;
        let nb = src[nidx + 2] as i32;
        let na = src[nidx + 3] as i32;

        let dr = (nr - cr).abs();
        let dg = (ng - cg).abs();
        let db = (nb - cb).abs();

        // Use max channel difference to approximate edge sensitivity
        let diff = dr.max(dg).max(db);
        if diff <= thr {
          sum_r += nr;
          sum_g += ng;
          sum_b += nb;
          sum_a += na;
          count += 1;
        }
      }
    }

    if count > 0 {
      dst[0] = (sum_r / count).clamp(0, 255) as u8;
      dst[1] = (sum_g / count).clamp(0, 255) as u8;
      dst[2] = (sum_b / count).clamp(0, 255) as u8;
      dst[3] = (sum_a / count).clamp(0, 255) as u8;
    } else {
      // Fallback to center pixel if nothing matched (should be rare for reasonable thresholds)
      dst[0] = cr as u8;
      dst[1] = cg as u8;
      dst[2] = cb as u8;
      dst[3] = ca as u8;
    }
  });

  image.set_rgba(out);
}
