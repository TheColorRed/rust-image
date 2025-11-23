//! Helpers for preparing pixel areas, computing feather/mask alpha maps, and blending processed
//! sub-images back into an `Image`.
//!
//! This module provides a single source of truth for area/mask/feather handling used by
//! filters and adjustments that operate on a sub-rectangle of an image. It contains helpers to:
//! - Extract a sub-rectangle pixel buffer expanded with padding for kernel operations.
//! - Compute a per-pixel alpha mask combining `Area` feathering and an optional `Mask` image.
//! - Blend processed pixel buffers back into the destination image using the computed mask.
//!
//! This module should be considered the canonical implementation for area/feather/mask handling.
use crate::geometry::Area;
use crate::{Channels, Image};
use rayon::prelude::*;
use std::borrow::Cow;

/// Result structure containing a prepared (potentially owned) pixel buffer and the processing rects.
pub struct PreparedArea<'a> {
  /// Pixel buffer for the processing rectangle: width * height * 4
  pub pixels: Cow<'a, [u8]>,
  /// The full image width/height
  pub image_width: usize,
  pub image_height: usize,
  /// The processing target area (unexpanded by kernel) in image coordinates
  pub area_min_x: i32,
  pub area_min_y: i32,
  pub area_w: i32,
  pub area_h: i32,
  /// The expanded rect (expanded by kernel padding) that contains neighboring pixels needed for convolution
  pub rect_min_x: i32,
  pub rect_min_y: i32,
  pub rect_w: i32,
  pub rect_h: i32,
}

/// Small metadata subset used for mask computation and blending that does not include the
/// potentially borrowed pixel buffer. This avoids keeping an immutable borrow of the
/// image while the image needs to be mutated.
#[derive(Clone, Copy, Debug)]
pub struct PreparedAreaMeta {
  pub image_width: usize,
  pub image_height: usize,
  pub area_min_x: i32,
  pub area_min_y: i32,
  pub area_w: i32,
  pub area_h: i32,
  pub rect_min_x: i32,
  pub rect_min_y: i32,
  pub rect_w: i32,
  pub rect_h: i32,
}

impl<'a> PreparedArea<'a> {
  pub fn meta(&self) -> PreparedAreaMeta {
    PreparedAreaMeta {
      image_width: self.image_width,
      image_height: self.image_height,
      area_min_x: self.area_min_x,
      area_min_y: self.area_min_y,
      area_w: self.area_w,
      area_h: self.area_h,
      rect_min_x: self.rect_min_x,
      rect_min_y: self.rect_min_y,
      rect_w: self.rect_w,
      rect_h: self.rect_h,
    }
  }
}

/// Prepare pixel data for processing a sub-rectangle area. Returns a `PreparedArea` containing
/// a borrowed or owned pixel buffer depending on whether an area is provided.
/// - `image`: the source image
/// - `area`: the optional area to process; if `None` we return a borrowed slice of the full image
/// - `kernel_padding`: padding in pixels to ensure neighbor pixels for convolution are included
pub fn prepare_area_pixels<'a>(image: &'a Image, area: Option<&Area>, kernel_padding: i32) -> PreparedArea<'a> {
  let (image_w, image_h) = image.dimensions::<u32>();
  let image_w = image_w as i32;
  let image_h = image_h as i32;

  let rgba = image.rgba();

  match area {
    Some(a) => {
      let (mut min_x, mut min_y, mut max_x, mut max_y) = a.bounds::<i32>();
      if min_x < 0 {
        min_x = 0;
      }
      if min_y < 0 {
        min_y = 0;
      }
      if max_x > image_w {
        max_x = image_w;
      }
      if max_y > image_h {
        max_y = image_h;
      }
      if min_x >= max_x || min_y >= max_y {
        // Area empty: return empty Owned buffer and zero rect (avoid panic downstream)
        return PreparedArea {
          pixels: Cow::Owned(vec![]),
          image_width: image_w as usize,
          image_height: image_h as usize,
          area_min_x: min_x,
          area_min_y: min_y,
          area_w: 0,
          area_h: 0,
          rect_min_x: min_x,
          rect_min_y: min_y,
          rect_w: 0,
          rect_h: 0,
        };
      }
      let rect_min_x = (min_x - kernel_padding).max(0);
      let rect_min_y = (min_y - kernel_padding).max(0);
      let rect_max_x = (max_x + kernel_padding).min(image_w);
      let rect_max_y = (max_y + kernel_padding).min(image_h);
      let rect_w = (rect_max_x - rect_min_x) as usize;
      let rect_h = (rect_max_y - rect_min_y) as usize;

      // Extract pixels row by row to a fresh buffer
      let row_stride = (image.dimensions::<usize>().0 * 4) as usize;
      let mut pixels: Vec<u8> = Vec::with_capacity(rect_w * rect_h * 4);
      for ry in rect_min_y..rect_max_y {
        let start = (ry as usize * row_stride) + (rect_min_x as usize * 4);
        let end = start + (rect_w * 4);
        pixels.extend_from_slice(&rgba[start..end]);
      }

      PreparedArea {
        pixels: Cow::Owned(pixels),
        image_width: image_w as usize,
        image_height: image_h as usize,
        area_min_x: min_x,
        area_min_y: min_y,
        area_w: (max_x - min_x) as i32,
        area_h: (max_y - min_y) as i32,
        rect_min_x: rect_min_x,
        rect_min_y: rect_min_y,
        rect_w: rect_w as i32,
        rect_h: rect_h as i32,
      }
    }
    None => PreparedArea {
      pixels: Cow::Borrowed(rgba),
      image_width: image_w as usize,
      image_height: image_h as usize,
      area_min_x: 0,
      area_min_y: 0,
      area_w: image_w,
      area_h: image_h,
      rect_min_x: 0,
      rect_min_y: 0,
      rect_w: image_w,
      rect_h: image_h,
    },
  }
}

/// Compute a per-pixel alpha mask (0.0 .. 1.0) for a prepared area based on `Area` feathering and optional `Mask`.
/// - `prepared`: prepared area metadata
/// - `area`: optional area (may be None). If None, mask is all ones.
/// - `mask_image`: optional RGBA mask image bytes (full image size RGBA bytes). If provided it will be sampled and combined multiplicatively.
pub fn compute_area_mask(prepared: &PreparedAreaMeta, area: Option<&Area>, mask_image: Option<&[u8]>) -> Vec<f32> {
  let width = prepared.rect_w as usize;
  let height = prepared.rect_h as usize;
  let rect_min_x = prepared.rect_min_x as i32;
  let rect_min_y = prepared.rect_min_y as i32;

  let mut mask: Vec<f32> = vec![0.0f32; width * height];
  let feather_amount = area.map(|a| a.feather()).unwrap_or(0) as i32;

  // compute area coverage / feather first (0..1)
  mask.par_chunks_mut(width).enumerate().for_each(|(py, chunk)| {
    for px in 0..chunk.len() {
      let gx = (rect_min_x as i32 + px as i32) as f32 + 0.5;
      let gy = (rect_min_y as i32 + py as i32) as f32 + 0.5;
      let v = if let Some(a) = area {
        if a.contains((gx, gy)) {
          if feather_amount > 0 {
            let closest = a.path.closest_point(gx, gy);
            let d = ((gx - closest.x).powi(2) + (gy - closest.y).powi(2)).sqrt();
            (d as f32 / feather_amount as f32).clamp(0.0, 1.0)
          } else {
            1.0
          }
        } else {
          0.0
        }
      } else {
        1.0
      };
      chunk[px] = v;
    }
  });

  // If a mask image is provided, sample and multiply.
  if let Some(mask_img) = mask_image {
    // mask_image is an RGBA buffer length = image_width * image_height * 4
    let img_w = prepared.image_width as usize;
    let _img_h = prepared.image_height as usize;
    for py in 0..height {
      for px in 0..width {
        let gx = (prepared.rect_min_x as usize + px) as usize;
        let gy = (prepared.rect_min_y as usize + py) as usize;
        let idx = (gy * img_w + gx) * 4;
        if idx + 3 < mask_img.len() {
          // convert mask pixel to grayscale
          let r = mask_img[idx] as u32;
          let g = mask_img[idx + 1] as u32;
          let b = mask_img[idx + 2] as u32;
          let gray = (((299 * r + 587 * g + 114 * b) + 500) / 1000) as u8;
          let mul = (gray as f32) / 255.0f32;
          let m_idx = py * width + px;
          mask[m_idx] *= mul;
        }
      }
    }
  }

  mask
}

/// Blend processed pixels (of size prepared.rect_w * prepared.rect_h) back into the original image
/// using the provided `mask` (0..1 floats) and writing into the image alpha correctly.
pub fn blend_area_pixels(image: &mut Image, processed: &[u8], prepared_meta: &PreparedAreaMeta, mask: &[f32]) {
  let (image_w, _) = image.dimensions::<usize>();
  let row_stride = (image_w * 4) as usize;
  let rect_w = prepared_meta.rect_w as usize;
  // copy original image
  let orig = image.rgba();
  let mut out = orig.to_vec();

  out.par_chunks_mut(row_stride).enumerate().for_each(|(row_y, row)| {
    if row_y < prepared_meta.rect_min_y as usize || row_y >= (prepared_meta.rect_min_y + prepared_meta.rect_h) as usize
    {
      return;
    }
    let py = row_y - prepared_meta.rect_min_y as usize;
    for px in 0..rect_w {
      let m_idx = py * rect_w + px;
      let alpha = mask[m_idx];
      if alpha <= 0.0 {
        continue;
      }
      let out_x = prepared_meta.rect_min_x as usize + px;
      let out_idx = out_x * 4;
      let processed_idx = (py * rect_w + px) * 4;

      // In-range check
      if out_idx + 3 >= row.len() || processed_idx + 3 >= processed.len() {
        continue;
      }

      let br = processed[processed_idx] as f32;
      let bg = processed[processed_idx + 1] as f32;
      let bb = processed[processed_idx + 2] as f32;
      let ba = processed[processed_idx + 3] as f32;
      let or = row[out_idx] as f32;
      let og = row[out_idx + 1] as f32;
      let ob = row[out_idx + 2] as f32;
      let oa = row[out_idx + 3] as f32;

      let fr = (br * alpha + or * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
      let fg = (bg * alpha + og * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
      let fb = (bb * alpha + ob * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
      let fa = (ba * alpha + oa * (1.0 - alpha)).clamp(0.0, 255.0) as u8;

      row[out_idx] = fr;
      row[out_idx + 1] = fg;
      row[out_idx + 2] = fb;
      row[out_idx + 3] = fa;
    }
  });

  image.set_rgba_owned(out);
}

/// High-level convenience: apply an already-processed RGBA buffer back into the destination image
/// using the prepared rect meta. This function takes ownership of `processed` and will perform
/// a fast-path replacement if the entire image was processed; otherwise it computes a mask
/// (area feather + optional mask image) and blends the processed buffer into place.
pub fn apply_processed_pixels_to_image(
  image: &mut Image, processed: Vec<u8>, prepared: &PreparedAreaMeta, area: Option<&Area>, mask_image: Option<&[u8]>,
) {
  let (image_w, image_h) = image.dimensions::<usize>();
  let full_image_processed = prepared.area_min_x == 0
    && prepared.area_min_y == 0
    && prepared.area_w as usize == image_w
    && prepared.area_h as usize == image_h
    && area.map(|a| a.feather()).unwrap_or(0) == 0
    && mask_image.is_none();

  if full_image_processed {
    // Fast path: no blending required; replace the entire image
    image.set_rgba_owned(processed);
  } else {
    let mask = compute_area_mask(prepared, area, mask_image);
    blend_area_pixels(image, processed.as_slice(), prepared, &mask);
  }
}

/// Convenience wrapper to run a processor over the prepared area pixel buffer
/// and then apply the processed pixels back to the destination image.
/// - `p_image`: the destination image to modify.
/// - `p_options`: optional `ApplyOptions` containing area and mask info.
/// - `p_kernel_padding`: padding around the kernel for processing.
/// - `p_processor`: closure that processes the prepared image area.
pub fn apply_processing<F>(
  p_image: &mut Image, p_options: Option<&ApplyOptions>, p_kernel_padding: impl Into<i32>, p_processor: F,
) where
  F: FnOnce(&mut Image),
{
  let area = p_options.as_ref().and_then(|o| o.area());
  let mask: Option<&[u8]> = p_options.as_ref().and_then(|o| o.mask().map(|m| m.image().rgba()));
  let kernel_padding = p_kernel_padding.into();
  // Prepare a sub-area for processing
  let prepared = prepare_area_pixels(p_image, area, kernel_padding);
  if prepared.area_w == 0 || prepared.area_h == 0 {
    return;
  }
  let width = prepared.rect_w as usize;
  let height = prepared.rect_h as usize;
  let pixels = prepared.pixels.as_ref();
  let mut tmp_img = Image::new_from_pixels(width as u32, height as u32, pixels.to_vec(), Channels::RGBA);
  // Run processor on the prepared pixels (mutably)
  p_processor(&mut tmp_img);
  // Apply result back to destination using the helper (handles fast-path), using metadata for blending
  let meta = prepared.meta();
  apply_processed_pixels_to_image(p_image, tmp_img.into_rgba_vec(), &meta, area, mask);
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Image;
  use crate::color::Color;
  use crate::geometry::Area;
  use std::borrow::Cow;

  #[test]
  fn prepare_area_pixels_full_image_borrowed() {
    let img = Image::new_from_color(8, 8, Color::from_rgba(0, 0, 0, 255));
    let prepared = prepare_area_pixels(&img, None, 2);
    // Full image should be borrowed, not owned (Cow::Borrowed)
    match prepared.pixels {
      Cow::Borrowed(_) => (),
      Cow::Owned(_) => panic!("Expected borrowed pixels for full image"),
    }
    assert_eq!(prepared.rect_w as usize, 8);
    assert_eq!(prepared.rect_h as usize, 8);
  }

  #[test]
  fn compute_area_mask_feathered() {
    let img = Image::new_from_color(16, 16, Color::from_rgba(255, 255, 255, 255));
    let area = Area::rect((2.0, 2.0), (8.0, 8.0)).with_feather(4);
    let prepared = prepare_area_pixels(&img, Some(&area), 2);
    let meta = prepared.meta();
    let mask = compute_area_mask(&meta, Some(&area), None);
    // center pixel should be near 1.0
    let center_x = (2 + 8 / 2) as usize;
    let center_y = (2 + 8 / 2) as usize;
    let idx = (center_y - meta.rect_min_y as usize) * meta.rect_w as usize + (center_x - meta.rect_min_x as usize);
    assert!(mask[idx] > 0.8);
    // near edge should be between 0 and 1
    let edge_x = (2 + 1) as usize;
    let edge_y = (2 + 1) as usize;
    let idx2 = (edge_y - meta.rect_min_y as usize) * meta.rect_w as usize + (edge_x - meta.rect_min_x as usize);
    assert!(mask[idx2] < 1.0 && mask[idx2] > 0.0);
  }

  #[test]
  fn blend_area_pixels_blends() {
    let mut img = Image::new_from_color(8, 8, Color::from_rgba(0, 0, 0, 255));
    // Make a processed buffer that is all white
    let processed = vec![255u8; 8 * 8 * 4];
    let area = Area::rect((2.0, 2.0), (4.0, 4.0)).with_feather(0);
    let prepared = prepare_area_pixels(&img, Some(&area), 0);
    let meta = prepared.meta();
    let mask = compute_area_mask(&meta, Some(&area), None);
    blend_area_pixels(&mut img, &processed, &meta, &mask);
    // Check that center of area has changed to white
    let idx = ((3 * 8 + 3) * 4) as usize;
    assert_eq!(img.rgba()[idx], 255);
    assert_eq!(img.rgba()[idx + 1], 255);
    assert_eq!(img.rgba()[idx + 2], 255);
  }
}
