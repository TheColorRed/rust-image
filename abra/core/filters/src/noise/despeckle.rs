use crate::common::*;

use abra_core::color::Histogram;

fn apply_despeckle(p_image: &mut Image, p_radius: f32, p_threshold: f32) {
  let (width, height) = p_image.dimensions::<u32>();
  if width == 0 || height == 0 {
    return;
  }

  let src = p_image.rgba();
  let mut out = p_image.empty_pixel_vec();
  let w = width as usize;
  let h = height as usize;

  // Convert radius to integer (clamp to reasonable range 1-30)
  let r = p_radius.clamp(1.0, 30.0).round() as i32;
  // Convert threshold from 0-255 range to integer
  let thr = p_threshold.clamp(0.0, 255.0).round() as i32;

  out.par_chunks_mut(w * 4).enumerate().for_each(|(y, row_out)| {
    // Allocate histogram once per row - uses Box<[u64; 256]> internally
    let mut hist = Histogram::new();

    for x in 0..w {
      let idx = (y * w + x) * 4;

      // Use unsafe for direct access - bounds already checked by dimensions
      let (cr, cg, cb, ca) = unsafe {
        (
          *src.get_unchecked(idx) as i32,
          *src.get_unchecked(idx + 1) as i32,
          *src.get_unchecked(idx + 2) as i32,
          *src.get_unchecked(idx + 3),
        )
      };

      // Clear and get mutable access to histogram arrays
      hist.clear();
      let (r_hist, g_hist, b_hist) = hist.rgb_mut();

      let mut min_lum = i32::MAX;
      let mut max_lum = i32::MIN;
      let mut pixel_count = 0u64;

      // Iterate over kernel neighborhood and build histograms
      for dy in -r..=r {
        let ny = ((y as i32 + dy).max(0).min(h as i32 - 1)) as usize;
        let ny_offset = ny * w * 4;

        for dx in -r..=r {
          let nx = ((x as i32 + dx).max(0).min(w as i32 - 1)) as usize;
          let n_idx = ny_offset + nx * 4;

          // Use unsafe for faster access
          let (nr, ng, nb) =
            unsafe { (*src.get_unchecked(n_idx), *src.get_unchecked(n_idx + 1), *src.get_unchecked(n_idx + 2)) };

          r_hist[nr as usize] += 1;
          g_hist[ng as usize] += 1;
          b_hist[nb as usize] += 1;
          pixel_count += 1;

          // Compute luminance using integer approximation
          let lum = (299 * (nr as i32) + 587 * (ng as i32) + 114 * (nb as i32)) / 1000;
          min_lum = min_lum.min(lum);
          max_lum = max_lum.max(lum);
        }
      }

      // Current pixel luminance
      let center_lum = (299 * cr + 587 * cg + 114 * cb) / 1000;

      // Compute median using histogram (much faster than sorting!)
      let mr = hist.red_median(pixel_count) as i32;
      let mg = hist.green_median(pixel_count) as i32;
      let mb = hist.blue_median(pixel_count) as i32;

      // If pixel is an outlier (local min or local max), replace with median
      let mut out_r = cr as u8;
      let mut out_g = cg as u8;
      let mut out_b = cb as u8;

      if center_lum == min_lum || center_lum == max_lum {
        out_r = mr as u8;
        out_g = mg as u8;
        out_b = mb as u8;
      } else {
        // Selective median: if the center differs enough from median, replace
        let d_r = (cr - mr).abs();
        let d_g = (cg - mg).abs();
        let d_b = (cb - mb).abs();
        let diff = d_r.max(d_g).max(d_b);
        if diff > thr {
          out_r = mr as u8;
          out_g = mg as u8;
          out_b = mb as u8;
        }
      }

      let out_idx = x * 4;
      row_out[out_idx] = out_r;
      row_out[out_idx + 1] = out_g;
      row_out[out_idx + 2] = out_b;
      row_out[out_idx + 3] = ca;
    }
  });

  p_image.set_rgba_owned(out);
}
/// Applies a despeckle filter to the image, removing isolated noise pixels while preserving edges.
/// - `p_image`: The image to apply the filter to.
/// - `p_apply_options`: Options to specify for the filter.
pub fn despeckle<'a>(
  p_image: impl Into<ImageRef<'a>>, p_radius: f32, p_threshold: f32, p_apply_options: impl Into<Options>,
) {
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  apply_filter!(apply_despeckle, image, p_apply_options, 1, p_radius, p_threshold);
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Image;
  use options::ApplyOptions;

  #[test]
  fn despeckle_removes_isolated_speck() {
    let mut img = Image::new(5, 5);
    // Fill with black
    for y in 0..5u32 {
      for x in 0..5u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    // Add single white speck in center
    img.set_pixel(2, 2, (255u8, 255u8, 255u8, 255));

    despeckle(&mut img, 1.0, 13.0, ApplyOptions::new());

    let (r, g, b, _) = img.get_pixel(2, 2).unwrap();
    assert_eq!(r, 0);
    assert_eq!(g, 0);
    assert_eq!(b, 0);
  }

  #[test]
  fn despeckle_preserves_edge() {
    let mut img = Image::new(3, 3);
    // Create simple edge: left column black, right column white
    for y in 0..3u32 {
      img.set_pixel(0, y, (0u8, 0u8, 0u8, 255));
      img.set_pixel(1, y, (255u8, 255u8, 255u8, 255));
      img.set_pixel(2, y, (255u8, 255u8, 255u8, 255));
    }
    // Add isolated noise near edge: a dark pixel in the white region (should be replaced only if not edge)
    img.set_pixel(2, 1, (0u8, 0u8, 0u8, 255));

    // Apply despeckle (should preserve the edge while removing isolated speck)
    despeckle(&mut img, 1.0, 13.0, ApplyOptions::new());

    // The pixel (2,1) should become white (replaced by median) because it's an isolated speck
    let (r, g, b, _) = img.get_pixel(2, 1).unwrap();
    assert_ne!(r, 0);
    assert_ne!(g, 0);
    assert_ne!(b, 0);
  }

  #[test]
  fn median_computation_for_edge_speck() {
    use abra_core::color::Histogram;

    // 3x3 image with col0 black, col1 white, col2 white except center black at (2,1)
    let mut img = Image::new(3, 3);
    for y in 0..3u32 {
      img.set_pixel(0, y, (0u8, 0u8, 0u8, 255));
      img.set_pixel(1, y, (255u8, 255u8, 255u8, 255));
      img.set_pixel(2, y, (255u8, 255u8, 255u8, 255));
    }
    img.set_pixel(2, 1, (0u8, 0u8, 0u8, 255));

    let src = img.rgba();
    let w = 3usize;
    let y = 1usize;
    let x = 2usize;
    let mut hist = Histogram::new();
    let (r_hist, g_hist, b_hist) = hist.rgb_mut();
    let r = 1i32;
    let h = 3usize;
    let mut pixel_count = 0u64;

    for dy in -r..=r {
      let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
      for dx in -r..=r {
        let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
        let n_idx = (ny * w + nx) * 4;
        r_hist[src[n_idx] as usize] += 1;
        g_hist[src[n_idx + 1] as usize] += 1;
        b_hist[src[n_idx + 2] as usize] += 1;
        pixel_count += 1;
      }
    }

    let mr = hist.red_median(pixel_count);
    let mg = hist.green_median(pixel_count);
    let mb = hist.blue_median(pixel_count);

    // Debug: print neighbors to help track median behavior
    // debug: neighbors and median computed
    assert_eq!(mr, 255);
    assert_eq!(mg, 255);
    assert_eq!(mb, 255);

    // Compute neighbor min/max luminance (excluding center) to verify neighbor_span is 0
    let mut neighbor_min_lum = i32::MAX;
    let mut neighbor_max_lum = i32::MIN;

    for dy in -r..=r {
      let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
      for dx in -r..=r {
        let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
        if nx == x && ny == y {
          continue;
        }
        let n_idx = (ny * w + nx) * 4;
        let nr = src[n_idx] as i32;
        let ng = src[n_idx + 1] as i32;
        let nb = src[n_idx + 2] as i32;
        let lum = (299 * nr + 587 * ng + 114 * nb) / 1000;
        if lum < neighbor_min_lum {
          neighbor_min_lum = lum;
        }
        if lum > neighbor_max_lum {
          neighbor_max_lum = lum;
        }
      }
    }
    // debug: neighbor span: neighbor_min_lum, neighbor_max_lum
    assert_eq!(neighbor_min_lum, neighbor_max_lum);
  }
}
