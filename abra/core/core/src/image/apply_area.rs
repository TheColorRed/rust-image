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
use crate::image::gpu::{GpuEffect, GpuProvider, Hardware, ready_gpu_provider};
use crate::{Channels, Image, LumaStandard, PointF, Rect, luma, polygon_contains};
use rayon::prelude::*;
use std::borrow::Cow;

/// Lightweight structure representing the primitives core needs from ApplyOptions
/// without depending on the options crate.
pub struct ApplyContext<'a> {
  pub area: Option<Vec<&'a Area>>,
  pub mask_image: Option<&'a [u8]>,
  pub hardware: Hardware,
}

/// Where a processing pass reads from and writes to, in image coordinates.
#[derive(Clone, Copy, Debug)]
pub struct PreparedAreaMeta {
  /// The full image width/height.
  pub image_width: usize,
  pub image_height: usize,
  /// The processing target area (unexpanded by kernel) in image coordinates.
  pub area_min_x: i32,
  pub area_min_y: i32,
  pub area_w: i32,
  pub area_h: i32,
  /// The expanded rect (expanded by kernel padding) that contains neighboring pixels needed for convolution.
  pub rect_min_x: i32,
  pub rect_min_y: i32,
  pub rect_w: i32,
  pub rect_h: i32,
}

/// A prepared (potentially borrowed) pixel buffer for the processing rect, and where it sits in the image.
pub struct PreparedArea<'a> {
  /// Pixel buffer for the processing rectangle: `rect_w * rect_h * 4`.
  pub pixels: Cow<'a, [u8]>,
  pub meta: PreparedAreaMeta,
}

/// Prepare pixel data for processing a sub-rectangle area. Returns a `PreparedArea` containing
/// a borrowed or owned pixel buffer depending on whether an area is provided.
/// - `p_image`: the source image
/// - `p_area`: the optional area to process; if `None` we return a borrowed slice of the full image
/// - `p_kernel_padding`: padding in pixels to ensure neighbor pixels for convolution are included
fn prepare_area_pixels<'a>(p_image: &'a Image, p_area: Option<&Area>, p_kernel_padding: i32) -> PreparedArea<'a> {
  let (image_w, image_h) = p_image.dimensions::<u32>();
  let image_rect = Rect::new((0, 0), (image_w, image_h));
  let rgba = p_image.rgba();
  let meta = |area: (i32, i32, i32, i32), rect: (i32, i32, i32, i32)| PreparedAreaMeta {
    image_width: image_w as usize,
    image_height: image_h as usize,
    area_min_x: area.0,
    area_min_y: area.1,
    area_w: area.2 - area.0,
    area_h: area.3 - area.1,
    rect_min_x: rect.0,
    rect_min_y: rect.1,
    rect_w: rect.2 - rect.0,
    rect_h: rect.3 - rect.1,
  };

  let Some(area) = p_area else {
    let full = (0, 0, image_w as i32, image_h as i32);
    return PreparedArea {
      pixels: Cow::Borrowed(rgba),
      meta: meta(full, full),
    };
  };

  let visible = area.bounds().intersect(image_rect);
  if visible.is_empty() {
    // Area empty: return an empty buffer and zero rect (avoid panic downstream).
    let (x, y, _, _) = visible.edges::<i32>();
    return PreparedArea {
      pixels: Cow::Owned(vec![]),
      meta: meta((x, y, x, y), (x, y, x, y)),
    };
  }
  let padding = p_kernel_padding as f32;
  let padded = Rect::from_edges(
    visible.left() - padding,
    visible.top() - padding,
    visible.right() + padding,
    visible.bottom() + padding,
  )
  .intersect(image_rect);
  let (rect_min_x, rect_min_y, rect_max_x, rect_max_y) = padded.edges::<i32>();

  // Extract the rect's rows into a fresh buffer.
  let row_stride = image_w as usize * 4;
  let row_bytes = (rect_max_x - rect_min_x) as usize * 4;
  let mut pixels = Vec::with_capacity(row_bytes * (rect_max_y - rect_min_y) as usize);
  for y in rect_min_y..rect_max_y {
    let start = y as usize * row_stride + rect_min_x as usize * 4;
    pixels.extend_from_slice(&rgba[start..start + row_bytes]);
  }

  PreparedArea {
    pixels: Cow::Owned(pixels),
    meta: meta(visible.edges::<i32>(), (rect_min_x, rect_min_y, rect_max_x, rect_max_y)),
  }
}

/// Distance from `p_point` to the nearest edge of the closed polygon through `p_points`.
fn distance_to_outline(p_points: &[PointF], p_point: PointF) -> f32 {
  let Some(mut previous) = p_points.last().copied() else {
    return f32::MAX;
  };
  let mut nearest = f32::MAX;
  for &current in p_points {
    let edge = current - previous;
    let length_squared = edge.length_squared();
    let t = if length_squared > 0.0 { ((p_point - previous).dot(edge) / length_squared).clamp(0.0, 1.0) } else { 0.0 };
    nearest = nearest.min(p_point.distance_to(previous.lerp(current, t)));
    previous = current;
  }
  nearest
}

/// Compute a per-pixel alpha mask (0.0 .. 1.0) for a prepared area based on `Area` feathering and optional `Mask`.
/// - `p_prepared`: prepared area metadata
/// - `p_area`: optional area (may be None). If None, mask is all ones.
/// - `p_mask_image`: optional RGBA mask image bytes (full image size RGBA bytes). If provided its luma scales the
///   mask.
///
/// Inside the area, a feathered edge fades linearly from 0 at the outline to 1 at `feather` pixels in, measured as
/// the distance to the nearest point of the outline.
fn compute_area_mask(p_prepared: &PreparedAreaMeta, p_area: Option<&Area>, p_mask_image: Option<&[u8]>) -> Vec<f32> {
  let width = p_prepared.rect_w as usize;
  let height = p_prepared.rect_h as usize;
  let mut mask = vec![1.0f32; width * height];
  if width == 0 {
    return mask;
  }

  if let Some(area) = p_area {
    // Flatten the outline once; every pixel is tested against the same polygon.
    let outline = area.flatten(0.5);
    let feather = area.feather() as f32;
    mask.par_chunks_mut(width).enumerate().for_each(|(py, row)| {
      let gy = (p_prepared.rect_min_y + py as i32) as f32 + 0.5;
      for (px, value) in row.iter_mut().enumerate() {
        let point = PointF::new((p_prepared.rect_min_x + px as i32) as f32 + 0.5, gy);
        *value = if !polygon_contains(&outline, point) {
          0.0
        } else if feather > 0.0 {
          (distance_to_outline(&outline, point) / feather).min(1.0)
        } else {
          1.0
        };
      }
    });
  }

  if let Some(mask_image) = p_mask_image {
    let image_width = p_prepared.image_width;
    mask.par_chunks_mut(width).enumerate().for_each(|(py, row)| {
      let gy = p_prepared.rect_min_y as usize + py;
      for (px, value) in row.iter_mut().enumerate() {
        let index = (gy * image_width + p_prepared.rect_min_x as usize + px) * 4;
        if let Some(pixel) = mask_image.get(index..index + 3) {
          *value *= luma(pixel[0] as f32, pixel[1] as f32, pixel[2] as f32, LumaStandard::Rec601) / 255.0;
        }
      }
    });
  }

  mask
}

/// A weight from `0.0` to `1.0` as a byte from 0 to 255, rounded. Every place that blends by an area or mask goes
/// through this, so the image path, the CPU and the GPU agree on the weight of every pixel.
pub fn weight_to_byte(p_weight: f32) -> u8 {
  (p_weight.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Mixes one channel: `p_weight` (0 to 255) of `p_after` and the rest of `p_before`, in whole numbers, rounded to the
/// nearest value. The blend shader for live images does exactly these steps.
fn mix_byte(p_before: u8, p_after: u8, p_weight: u32) -> u8 {
  ((p_after as u32 * p_weight + p_before as u32 * (255 - p_weight) + 127) / 255) as u8
}

/// Mixes `p_after` into `p_before` pixel by pixel: a pixel with weight 0 stays as it was, 255 takes the new one, and
/// values between mix them. Both are RGBA; `p_weights` has one byte per pixel (see [`weight_to_byte`]).
pub fn mix_by_weights(p_before: &[u8], p_after: &[u8], p_weights: impl IntoIterator<Item = u8>) -> Vec<u8> {
  p_before
    .chunks_exact(4)
    .zip(p_after.chunks_exact(4))
    .zip(p_weights)
    .flat_map(|((before, after), weight)| (0..4).map(move |channel| mix_byte(before[channel], after[channel], weight as u32)))
    .collect()
}

/// Puts `p_processed` (the whole image, processed) into `p_image`, limited by the areas and mask in `p_ctx`: only where
/// they allow, and mixed in at the edges. With neither, it simply replaces the image.
fn put_whole_image(p_image: &mut Image, p_processed: Vec<u8>, p_ctx: Option<&ApplyContext<'_>>) {
  let Some(ctx) = p_ctx.filter(|ctx| ctx.area.is_some() || ctx.mask_image.is_some()) else {
    p_image.set_rgba(p_processed);
    return;
  };
  let (width, height) = p_image.dimensions::<u32>();
  let weights = area_weights(width, height, ctx).into_iter().map(weight_to_byte);
  let mixed = mix_by_weights(&p_image.to_rgba_vec(), &p_processed, weights);
  p_image.set_rgba(mixed);
}

/// Runs `p_processor` over the whole image and then limits the result to the areas and mask in `p_ctx`, instead of
/// cropping the image to the area first like [`apply_in_area`]. Use it for an operation that depends on where a pixel
/// is in the image, such as a gradient or something centered on the image: a crop would move its origin.
/// - `p_image`: The image to modify.
/// - `p_ctx`: Optional area and mask (from `ApplyOptions`).
/// - `p_processor`: Processes the whole image on the CPU.
pub fn apply_to_whole_image_in_area<F>(p_image: &mut Image, p_ctx: Option<ApplyContext<'_>>, p_processor: F)
where
  F: FnOnce(&mut Image),
{
  let limited = p_ctx.as_ref().is_some_and(|ctx| ctx.area.is_some() || ctx.mask_image.is_some());
  if !limited {
    p_processor(p_image);
    return;
  }
  let before = p_image.to_rgba_vec();
  p_processor(p_image);
  let processed = p_image.to_rgba_vec();
  p_image.set_rgba(before);
  put_whole_image(p_image, processed, p_ctx.as_ref());
}

/// Like [`apply_to_whole_image_in_area`], but on the GPU: it never falls back to the CPU, and returns an error when no
/// GPU is available or the GPU fails, leaving `p_image` as it was.
pub fn apply_to_whole_image_in_area_gpu(
  p_image: &mut Image, p_ctx: Option<ApplyContext<'_>>, p_gpu: &dyn GpuEffect,
) -> Result<(), String> {
  let provider = ready_gpu_provider(Hardware::Gpu).ok_or("no GPU is available")?;
  let (width, height) = p_image.dimensions::<u32>();
  let processed = (provider.process)(p_gpu, width, height, &p_image.to_rgba_vec())?;
  put_whole_image(p_image, processed, p_ctx.as_ref());
  Ok(())
}

/// Blend processed pixels (of size prepared.rect_w * prepared.rect_h) back into the original image in place,
/// weighting each by the provided `p_mask` (0..1 floats). Only the rows of the processed rect are touched.
fn blend_area_pixels(p_image: &mut Image, p_processed: &[u8], p_prepared_meta: &PreparedAreaMeta, p_mask: &[f32]) {
  let (image_w, _) = p_image.dimensions::<usize>();
  let rect_w = p_prepared_meta.rect_w as usize;
  let (rect_x, rect_y) = (p_prepared_meta.rect_min_x as usize, p_prepared_meta.rect_min_y as usize);
  p_image
    .colors()
    .as_slice_mut()
    .expect("Image colors must be contiguous")
    .par_chunks_exact_mut(image_w * 4)
    .skip(rect_y)
    .take(p_prepared_meta.rect_h as usize)
    .enumerate()
    .for_each(|(py, row)| {
      for px in 0..rect_w {
        let alpha = p_mask[py * rect_w + px];
        if alpha <= 0.0 {
          continue;
        }
        let out = &mut row[(rect_x + px) * 4..(rect_x + px) * 4 + 4];
        let processed = &p_processed[(py * rect_w + px) * 4..(py * rect_w + px) * 4 + 4];
        let weight = weight_to_byte(alpha) as u32;
        for (destination, &source) in out.iter_mut().zip(processed) {
          *destination = mix_byte(*destination, source, weight);
        }
      }
    });
}

/// Apply an already-processed RGBA buffer back into the destination image using the prepared rect meta. When the
/// whole image was processed with no feathering or mask, the buffer replaces the image; otherwise it is blended in
/// through the area/mask alpha.
fn apply_processed_pixels_to_image(
  p_image: &mut Image, p_processed: Vec<u8>, p_prepared: &PreparedAreaMeta, p_area: Option<&Area>,
  p_mask_image: Option<&[u8]>,
) {
  let (image_w, image_h) = p_image.dimensions::<usize>();
  let full_image_processed = p_prepared.area_min_x == 0
    && p_prepared.area_min_y == 0
    && p_prepared.area_w as usize == image_w
    && p_prepared.area_h as usize == image_h
    && p_area.map_or(0, Area::feather) == 0
    && p_mask_image.is_none();

  if full_image_processed {
    p_image.set_rgba(p_processed);
  } else {
    let mask = compute_area_mask(p_prepared, p_area, p_mask_image);
    blend_area_pixels(p_image, &p_processed, p_prepared, &mask);
  }
}

/// How strongly an effect applies at each pixel of a `p_width` x `p_height` image when limited by `p_ctx`'s areas and
/// mask: `0.0` leaves the pixel alone, `1.0` takes the effect fully, and values between come from feathered area
/// edges and the mask's brightness. Returns `width * height` values in row order.
///
/// This is the same weight the CPU path blends processed pixels in with, over the whole image, so a renderer that
/// processes the whole image and then mixes by these weights matches [`apply_in_area`]. Where areas overlap, the
/// strongest weight wins.
pub fn area_weights(p_width: u32, p_height: u32, p_ctx: &ApplyContext<'_>) -> Vec<f32> {
  let (width, height) = (p_width as i32, p_height as i32);
  let meta = PreparedAreaMeta {
    image_width: p_width as usize,
    image_height: p_height as usize,
    area_min_x: 0,
    area_min_y: 0,
    area_w: width,
    area_h: height,
    rect_min_x: 0,
    rect_min_y: 0,
    rect_w: width,
    rect_h: height,
  };
  match &p_ctx.area {
    None => compute_area_mask(&meta, None, p_ctx.mask_image),
    Some(areas) => {
      let mut weights = vec![0.0f32; p_width as usize * p_height as usize];
      for area in areas {
        for (weight, other) in weights.iter_mut().zip(compute_area_mask(&meta, Some(area), p_ctx.mask_image)) {
          *weight = weight.max(other);
        }
      }
      weights
    }
  }
}

/// Run processing on each area of an image (or the whole image when there are none), handling
/// area/mask/feathering, then blend the processed pixels back into the image.
/// - `p_image`: The destination image to modify.
/// - `p_ctx`: Optional area, mask, and hardware selection (from `ApplyOptions`).
/// - `p_kernel_padding`: Padding around the kernel for processing.
/// - `p_gpu`: The GPU version of the processing (any [`GpuEffect`]), if the effect has one. It runs instead of `p_processor` when the
///   hardware selection allows it and a GPU provider is registered and ready.
/// - `p_processor`: Closure that processes the prepared image area on the CPU.
pub fn apply_in_area<F>(
  p_image: &mut Image, p_ctx: Option<ApplyContext<'_>>, p_kernel_padding: impl Into<i32>, p_gpu: Option<&dyn GpuEffect>,
  mut p_processor: F,
) where
  F: FnMut(&mut Image),
{
  let kernel_padding = p_kernel_padding.into();
  let mask = p_ctx.as_ref().and_then(|c| c.mask_image);
  let hardware = p_ctx.as_ref().map(|c| c.hardware).unwrap_or_default();
  let gpu = p_gpu.and_then(|effect| ready_gpu_provider(hardware).map(|provider| (effect, provider)));

  match p_ctx.as_ref().and_then(|c| c.area.clone()) {
    Some(areas) => {
      for area in areas {
        process_area(p_image, Some(area), kernel_padding, mask, gpu.as_ref(), &mut p_processor);
      }
    }
    None => process_area(p_image, None, kernel_padding, mask, gpu.as_ref(), &mut p_processor),
  }
}

/// Like [`apply_in_area`], but always on the GPU: it never falls back to the CPU, and returns an error when no GPU is
/// available or the GPU fails. When it fails, `p_image` is left as it was.
/// - `p_image`: The destination image to modify.
/// - `p_ctx`: Optional area and mask (from `ApplyOptions`).
/// - `p_kernel_padding`: Padding around the kernel for processing.
/// - `p_gpu`: The GPU version of the processing.
pub fn apply_in_area_gpu(
  p_image: &mut Image, p_ctx: Option<ApplyContext<'_>>, p_kernel_padding: impl Into<i32>, p_gpu: &dyn GpuEffect,
) -> Result<(), String> {
  let provider = ready_gpu_provider(Hardware::Gpu).ok_or("no GPU is available")?;
  let kernel_padding = p_kernel_padding.into();
  let mask = p_ctx.as_ref().and_then(|c| c.mask_image);
  let areas: Vec<Option<&Area>> = match p_ctx.as_ref().and_then(|c| c.area.clone()) {
    Some(areas) => areas.into_iter().map(Some).collect(),
    None => vec![None],
  };
  // Several areas are applied one after the other, so a failure part way would leave some of them applied.
  let snapshot = (areas.len() > 1).then(|| p_image.to_rgba_vec());

  for area in areas {
    let prepared = prepare_area_pixels(p_image, area, kernel_padding);
    let meta = prepared.meta;
    if meta.area_w == 0 || meta.area_h == 0 {
      continue;
    }
    match (provider.process)(p_gpu, meta.rect_w as u32, meta.rect_h as u32, prepared.pixels.as_ref()) {
      Ok(processed) => apply_processed_pixels_to_image(p_image, processed, &meta, area, mask),
      Err(error) => {
        if let Some(pixels) = snapshot {
          p_image.set_rgba(pixels);
        }
        return Err(error);
      }
    }
  }
  Ok(())
}

/// Process one area (or the full image when `p_area` is `None`) and blend the result back. Uses the GPU when one
/// is given, falling back to `p_processor` if it fails.
fn process_area<F>(
  p_image: &mut Image, p_area: Option<&Area>, p_kernel_padding: i32, p_mask: Option<&[u8]>,
  p_gpu: Option<&(&dyn GpuEffect, GpuProvider)>, p_processor: &mut F,
) where
  F: FnMut(&mut Image),
{
  let prepared = prepare_area_pixels(p_image, p_area, p_kernel_padding);
  let meta = prepared.meta;
  if meta.area_w == 0 || meta.area_h == 0 {
    return;
  }

  if let Some((effect, provider)) = p_gpu {
    // A GPU error is non-fatal: fall back to the CPU path.
    if let Ok(processed) = (provider.process)(*effect, meta.rect_w as u32, meta.rect_h as u32, prepared.pixels.as_ref()) {
      apply_processed_pixels_to_image(p_image, processed, &meta, p_area, p_mask);
      return;
    }
  }

  let mut area_image = Image::new_from_pixels(meta.rect_w as u32, meta.rect_h as u32, prepared.pixels, Channels::RGBA);
  p_processor(&mut area_image);
  apply_processed_pixels_to_image(p_image, area_image.into_rgba_vec(), &meta, p_area, p_mask);
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::image::gpu::{GpuOp, clear_gpu_provider, register_gpu_provider};
  use primitives::Color;
  use std::sync::Mutex;

  /// The GPU provider is global, so tests that register one must not run in parallel.
  static GPU_PROVIDER_LOCK: Mutex<()> = Mutex::new(());

  fn gpu_context() -> Option<ApplyContext<'static>> {
    Some(ApplyContext {
      area: None,
      mask_image: None,
      hardware: Hardware::Gpu,
    })
  }

  #[test]
  fn prepare_area_pixels_full_image_borrowed() {
    let img = Image::new_from_color(8, 8, Color::from_rgba(0, 0, 0, 255));
    let prepared = prepare_area_pixels(&img, None, 2);
    assert!(matches!(prepared.pixels, Cow::Borrowed(_)), "Expected borrowed pixels for full image");
    assert_eq!((prepared.meta.rect_w, prepared.meta.rect_h), (8, 8));
  }

  #[test]
  fn prepare_area_pixels_pads_and_clips() {
    let img = Image::new(10, 10);
    let area = Area::rect((1, 6), (4, 8));
    let meta = prepare_area_pixels(&img, Some(&area), 2).meta;
    assert_eq!((meta.area_min_x, meta.area_min_y, meta.area_w, meta.area_h), (1, 6, 4, 4));
    assert_eq!((meta.rect_min_x, meta.rect_min_y, meta.rect_w, meta.rect_h), (0, 4, 7, 6));
  }

  #[test]
  fn mixing_by_weights_keeps_takes_or_blends_each_pixel() {
    let before = [0u8, 0, 0, 255, 10, 20, 30, 255, 100, 100, 100, 255];
    let after = [255u8, 255, 255, 255, 110, 120, 130, 255, 200, 200, 200, 255];
    let out = mix_by_weights(&before, &after, [0u8, 255, 128]);
    assert_eq!(out[0..4], before[0..4], "weight 0 keeps the pixel");
    assert_eq!(out[4..8], after[4..8], "weight 255 takes the new one");
    // Halfway, rounded to the nearest: 100 and 200 mix to 150, and 0 and 255 to 128.
    assert_eq!(out[8], 150);
    assert_eq!(mix_by_weights(&[0, 0, 0, 255], &[255, 255, 255, 255], [128u8])[0], 128);
  }

  #[test]
  fn weights_round_to_the_nearest_byte() {
    assert_eq!((weight_to_byte(0.0), weight_to_byte(1.0), weight_to_byte(0.5)), (0, 255, 128));
    assert_eq!((weight_to_byte(-3.0), weight_to_byte(9.0)), (0, 255));
  }

  #[test]
  fn area_weights_cover_the_whole_image() {
    let ctx = ApplyContext {
      area: None,
      mask_image: None,
      hardware: Hardware::Auto,
    };
    assert_eq!(area_weights(4, 3, &ctx), vec![1.0; 12]);

    let area = Area::rect((1.0, 1.0), (2.0, 1.0));
    let ctx = ApplyContext {
      area: Some(vec![&area]),
      mask_image: None,
      hardware: Hardware::Auto,
    };
    let weights = area_weights(4, 3, &ctx);
    let at = |x: usize, y: usize| weights[y * 4 + x];
    assert_eq!((at(1, 1), at(2, 1)), (1.0, 1.0));
    assert_eq!((at(0, 1), at(3, 1), at(1, 0), at(1, 2)), (0.0, 0.0, 0.0, 0.0));
  }

  #[test]
  fn compute_area_mask_feathered() {
    let img = Image::new_from_color(16, 16, Color::from_rgba(255, 255, 255, 255));
    let area = Area::rect((2.0, 2.0), (8.0, 8.0)).with_feather(4);
    let meta = prepare_area_pixels(&img, Some(&area), 2).meta;
    let mask = compute_area_mask(&meta, Some(&area), None);
    let at = |x: i32, y: i32| mask[((y - meta.rect_min_y) * meta.rect_w + (x - meta.rect_min_x)) as usize];
    // The center pixel is well inside the feather.
    assert!(at(6, 6) > 0.8);
    // Near the edge is partly faded.
    assert!(at(3, 3) < 1.0 && at(3, 3) > 0.0);
    // Outside is untouched.
    assert_eq!(at(1, 1), 0.0);
  }

  #[test]
  fn compute_area_mask_feathers_all_sides() {
    let img = Image::new_from_color(400, 400, Color::from_rgba(0, 0, 0, 255));
    let area = Area::rect((100.0, 100.0), (200.0, 200.0)).with_feather(50);
    let meta = prepare_area_pixels(&img, Some(&area), 0).meta;
    let mask = compute_area_mask(&meta, Some(&area), None);
    let at = |x: i32, y: i32| mask[((y - meta.rect_min_y) * meta.rect_w + (x - meta.rect_min_x)) as usize];
    // Five pixels inside each side, halfway along it.
    for (x, y, side) in [
      (105, 200, "left"),
      (294, 200, "right"),
      (200, 105, "top"),
      (200, 294, "bottom"),
    ] {
      assert!(at(x, y) > 0.0 && at(x, y) < 1.0, "{side} side not feathered: {}", at(x, y));
    }
    // Near a corner the mask stays positive instead of dipping below zero.
    assert!(at(105, 105) > 0.0);
    assert_eq!(at(200, 200), 1.0);
  }

  #[test]
  fn rotated_squares_feather_from_their_own_edges() {
    let img = Image::new(100, 100);
    let diamond = Area::from_points(&[[50.0, 10.0], [90.0, 50.0], [50.0, 90.0], [10.0, 50.0]]).with_feather(10);
    let meta = prepare_area_pixels(&img, Some(&diamond), 0).meta;
    let mask = compute_area_mask(&meta, Some(&diamond), None);
    let at = |x: i32, y: i32| mask[((y - meta.rect_min_y) * meta.rect_w + (x - meta.rect_min_x)) as usize];
    // Just inside the top corner, but far from the bounding box's edges, is still mostly faded.
    assert!(at(50, 14) < 0.5, "{}", at(50, 14));
    assert_eq!(at(50, 50), 1.0);
  }

  #[test]
  fn blend_area_pixels_blends() {
    let mut img = Image::new_from_color(8, 8, Color::from_rgba(0, 0, 0, 255));
    let area = Area::rect((2.0, 2.0), (4.0, 4.0));
    let meta = prepare_area_pixels(&img, Some(&area), 0).meta;
    let processed = vec![255u8; (meta.rect_w * meta.rect_h * 4) as usize];
    let mask = compute_area_mask(&meta, Some(&area), None);
    blend_area_pixels(&mut img, &processed, &meta, &mask);
    assert_eq!(img.get_pixel(3, 3), Some((255, 255, 255, 255)));
    assert_eq!(img.get_pixel(1, 1), Some((0, 0, 0, 255)));
  }

  #[test]
  fn apply_in_area_uses_gpu_provider() {
    let _lock = GPU_PROVIDER_LOCK.lock().unwrap();
    let mut img = Image::new_from_color(8, 8, Color::from_rgba(0, 0, 0, 255));
    register_gpu_provider(GpuProvider {
      wait_until_ready: || true,
      process: |_effect, w, h, _pixels| Ok(vec![255u8; (w * h * 4) as usize]),
      new_session: || Err("unused".to_string()),
    });
    // The CPU processor would leave the image black; the GPU makes it white.
    apply_in_area(&mut img, gpu_context(), 0, Some(&GpuOp::new("", [])), |_tmp| {});
    clear_gpu_provider();
    assert_eq!(img.rgba()[0], 255);
  }

  #[test]
  fn apply_in_area_falls_back_on_gpu_error() {
    let _lock = GPU_PROVIDER_LOCK.lock().unwrap();
    let mut img = Image::new_from_color(8, 8, Color::from_rgba(0, 0, 0, 255));
    register_gpu_provider(GpuProvider {
      wait_until_ready: || true,
      process: |_effect, _w, _h, _pixels| Err("gpu error".to_string()),
      new_session: || Err("unused".to_string()),
    });
    apply_in_area(&mut img, gpu_context(), 0, Some(&GpuOp::new("", [])), |tmp| tmp.set_pixel(0, 0, (100, 0, 0, 255)));
    clear_gpu_provider();
    assert_eq!(img.rgba()[0], 100);
  }

  #[test]
  fn apply_in_area_uses_cpu_without_gpu_op() {
    let _lock = GPU_PROVIDER_LOCK.lock().unwrap();
    let mut img = Image::new_from_color(8, 8, Color::from_rgba(0, 0, 0, 255));
    register_gpu_provider(GpuProvider {
      wait_until_ready: || true,
      process: |_effect, w, h, _pixels| Ok(vec![0u8; (w * h * 4) as usize]),
      new_session: || Err("unused".to_string()),
    });
    apply_in_area(&mut img, gpu_context(), 0, None, |tmp| tmp.set_pixel(0, 0, (50, 0, 0, 255)));
    clear_gpu_provider();
    assert_eq!(img.rgba()[0], 50);
  }
}
