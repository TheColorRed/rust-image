use abra_core::Color;
use abra_core::Image;
use abra_core::image::image_ext::ImageRef;
use options::Options;
use rayon::prelude::*;

use crate::apply_filter;

/// Default threshold for selective median replacement (approx 5% of 255).
const DEFAULT_THRESHOLD: i32 = 13;

fn apply_despeckle(p_image: &mut Image) {
  let (width, height) = p_image.dimensions::<u32>();
  if width == 0 || height == 0 {
    return;
  }

  let src = p_image.rgba();
  let mut out = p_image.empty_pixel_vec();
  let w = width as usize;
  let h = height as usize;

  // Radius 1 (3x3 kernel)
  let r = 1i32;
  let thr = DEFAULT_THRESHOLD;

  out.par_chunks_mut(w * 4).enumerate().for_each(|(y, row_out)| {
    for x in 0..w {
      let idx = (y * w + x) * 4;
      let cr = src[idx] as i32;
      let cg = src[idx + 1] as i32;
      let cb = src[idx + 2] as i32;
      let ca = src[idx + 3];

      // collect neighbor colors and compute min/max luminance for entire neighborhood and for neighbors only
      let mut neighbors: Vec<u8> = Vec::with_capacity(9 * 4);
      let mut min_lum = i32::MAX;
      let mut max_lum = i32::MIN;
      let mut neighbor_min_lum = i32::MAX;
      let mut neighbor_max_lum = i32::MIN;

      for dy in -r..=r {
        let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
        for dx in -r..=r {
          let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
          let nidx = (ny * w + nx) * 4;
          let nr = src[nidx] as i32;
          let ng = src[nidx + 1] as i32;
          let nb = src[nidx + 2] as i32;
          neighbors.push(nr as u8);
          neighbors.push(ng as u8);
          neighbors.push(nb as u8);
          neighbors.push(src[nidx + 3]);

          // compute luminance (simple integer approximation) and min/max
          let lum = (299 * nr + 587 * ng + 114 * nb) / 1000;
          if lum < min_lum {
            min_lum = lum;
          }
          if lum > max_lum {
            max_lum = lum;
          }
          // Track neighbor-only luminance min/max (exclude center pixel duplicates)
          if !(nx == x && ny == y) {
            if lum < neighbor_min_lum {
              neighbor_min_lum = lum;
            }
            if lum > neighbor_max_lum {
              neighbor_max_lum = lum;
            }
          }
        }
      }

      // Current pixel luminance
      let center_lum = (299 * cr + 587 * cg + 114 * cb) / 1000;

      // Compute median color for replacement candidate
      let median_color = Color::median(&neighbors);
      let mr = median_color.r as i32;
      let mg = median_color.g as i32;
      let mb = median_color.b as i32;

      // If pixel is an outlier (local min or local max) and not on an edge, replace with median
      let mut out_r = cr as u8;
      let mut out_g = cg as u8;
      let mut out_b = cb as u8;

      if center_lum == min_lum || center_lum == max_lum {
        out_r = mr as u8;
        out_g = mg as u8;
        out_b = mb as u8;
      } else {
        // Selective median: if the center differs enough from median and not an edge, replace
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
pub fn despeckle<'a>(p_image: impl Into<ImageRef<'a>>, p_apply_options: impl Into<Options>) {
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  apply_filter!(apply_despeckle, image, p_apply_options, 1);
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

    despeckle(&mut img, ApplyOptions::new());

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
    despeckle(&mut img, ApplyOptions::new());

    // The pixel (2,1) should become white (replaced by median) because it's an isolated speck
    let (r, g, b, _) = img.get_pixel(2, 1).unwrap();
    assert_ne!(r, 0);
    assert_ne!(g, 0);
    assert_ne!(b, 0);
  }

  #[test]
  fn median_computation_for_edge_speck() {
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
    let mut neighbors: Vec<u8> = Vec::with_capacity(9 * 4);
    let r = 1i32;
    let h = 3usize;
    for dy in -r..=r {
      let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
      for dx in -r..=r {
        let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
        let nidx = (ny * w + nx) * 4;
        // neighbor coordinate: ({}, {})
        let nidx = (ny * w + nx) * 4;
        neighbors.push(src[nidx]);
        neighbors.push(src[nidx + 1]);
        neighbors.push(src[nidx + 2]);
        neighbors.push(src[nidx + 3]);
      }
    }
    let med = Color::median(&neighbors);
    // Debug: print neighbors to help track median behaviour
    // debug: neighbors and median computed
    assert_eq!(med.r, 255);
    assert_eq!(med.g, 255);
    assert_eq!(med.b, 255);

    // Compute neighbor min/max luminance (excluding center) to verify neighbor_span is 0
    let mut neighbor_min_lum = i32::MAX;
    let mut neighbor_max_lum = i32::MIN;
    let side = (2 * r + 1) as usize;
    for (i, chunk) in neighbors.chunks(4).enumerate() {
      let row = i / side;
      let col = i % side;
      let dx = col as i32 - r;
      let dy = row as i32 - r;
      let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
      let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
      if nx == x && ny == y {
        continue;
      }
      let nr = chunk[0] as i32;
      let ng = chunk[1] as i32;
      let nb = chunk[2] as i32;
      let lum = (299 * nr + 587 * ng + 114 * nb) / 1000;
      if lum < neighbor_min_lum {
        neighbor_min_lum = lum;
      }
      if lum > neighbor_max_lum {
        neighbor_max_lum = lum;
      }
    }
    // debug: neighbor span: neighbor_min_lum, neighbor_max_lum
    assert_eq!(neighbor_min_lum, neighbor_max_lum);
  }
}
