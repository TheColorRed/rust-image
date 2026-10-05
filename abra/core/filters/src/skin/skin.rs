use crate::common::*;

use crate::sobel::sobel_magnitude;
use abra_core::color::rgb_to_ycbcr;
use abra_core::image::gpu::{GpuAux, GpuPass};
use abra_core::{ResizeTarget, Size, Transform, TransformAlgorithm};
use mask::Mask;
use std::sync::Arc;

// Skin is found by color in YCbCr, which keeps brightness (Y) apart from color (Cb, Cr), so skin in shadow and in sun
// both match. These are close to the ranges commonly used for skin, with the low end of Cr raised a little so that
// whites and grays, which sit at 128, fall fully outside. A pixel inside them is skin, and one more than
// `CHROMA_MARGIN` outside them is not; in between it counts partly, so the edge of the skin is not a hard line.
const CB_RANGE: (f32, f32) = (77.0, 127.0);
const CR_RANGE: (f32, f32) = (135.0, 173.0);
const CHROMA_MARGIN: f32 = 6.0;
// Below `Y_DARK` a pixel is too dark for its color to mean anything (hair, shadows, pupils), and by `Y_FULL` it counts
// completely.
const Y_DARK: f32 = 30.0;
const Y_FULL: f32 = 50.0;

// Skin is smooth, while eyes, lashes, brows, lips, nostrils and strands of hair are full of edges. Where the brightness
// changes by more than `EDGE_START` levels from one pixel to the next the mask starts to close, and by `EDGE_FULL` the
// pixel is left completely alone. Edges are measured as if the photo were `REFERENCE_SIZE` pixels on its long side, by
// spacing the edge kernel's taps out on a larger photo, so a big photo is protected the same as a small one without its
// grain being multiplied too. The protected zone is then widened (`EDGE_SPREAD`, a fraction of the long side, up to
// `EDGE_SPREAD_MAX`) and multiplied by `EDGE_GAIN`, so the area around a feature is kept too, not just its outline, and
// the middle of the zone stays fully protected.
const EDGE_START: f32 = 10.0;
const EDGE_FULL: f32 = 32.0;
const REFERENCE_SIZE: f32 = 1200.0;
const EDGE_SPREAD: f32 = 0.002;
const EDGE_SPREAD_MAX: usize = 24;
const EDGE_GAIN: f32 = 2.5;

// How far the edge of the skin mask fades out, as a fraction of the long side, so an effect on the skin does not end in a
// line at any size. It is never more than a quarter of the long side.
const FEATHER_FRACTION: f32 = 0.0045;

/// Simple separable box blur on a float mask (in place), `2 * p_radius + 1` pixels wide and tall. Values past the border
/// count as 0 and the sum is always divided by the full box, so the mask fades toward its edges, the same as the shader
/// that blurs it on the GPU. Rows are blurred in parallel, and the columns are blurred as the rows of the transposed mask.
fn box_blur_f32_inplace(p_mask: &mut [f32], p_width: usize, p_height: usize, p_radius: usize) {
  if p_radius == 0 {
    return;
  }
  let box_width = (2 * p_radius + 1) as f32;
  let mut scratch = vec![0.0f32; p_mask.len()];

  // Blurs each row of `source` (rows `width` long) into `destination` with a sliding sum.
  let blur_rows = |source: &[f32], destination: &mut [f32], width: usize| {
    destination.par_chunks_mut(width).zip(source.par_chunks(width)).for_each(|(out, row)| {
      // The box around the first pixel covers the pixels 0..=radius; the part before the row counts as zero.
      let mut sum: f32 = row[..(p_radius + 1).min(width)].iter().sum();
      for x in 0..width {
        out[x] = sum / box_width;
        if x + p_radius + 1 < width {
          sum += row[x + p_radius + 1];
        }
        if x >= p_radius {
          sum -= row[x - p_radius];
        }
      }
    });
  };
  // Writes the transpose of `source` (`source_width` wide, `source_height` tall) into `destination`.
  let transpose = |source: &[f32], source_width: usize, source_height: usize, destination: &mut [f32]| {
    destination.par_chunks_mut(source_height).enumerate().for_each(|(x, out)| {
      for (y, value) in out.iter_mut().enumerate() {
        *value = source[y * source_width + x];
      }
    });
  };

  blur_rows(p_mask, &mut scratch, p_width);
  transpose(&scratch, p_width, p_height, p_mask);
  blur_rows(p_mask, &mut scratch, p_height);
  transpose(&scratch, p_height, p_width, p_mask);
}

pub(super) fn feather_mask(p_mask: &mut [f32], p_width: usize, p_height: usize, p_feather: f32) {
  let long_side = p_width.max(p_height) as f32;
  let feather = (long_side * FEATHER_FRACTION * p_feather).round() as usize;
  box_blur_f32_inplace(p_mask, p_width, p_height, feather.clamp(1, (p_width.max(p_height) / 4).max(1)));
}

pub(super) fn supplied_skin_mask(p_image: &Image, p_mask: &Mask, p_feather: f32) -> Vec<f32> {
  let (width, height) = p_image.dimensions::<usize>();
  supplied_skin_mask_values(p_mask, width as u32, height as u32, p_feather)
}

pub(super) fn supplied_skin_mask_aux(p_mask: &Mask, p_width: u32, p_height: u32, p_feather: f32) -> GpuAux {
  let mask = supplied_skin_mask_values(p_mask, p_width, p_height, p_feather);
  let rgba: Vec<u8> = mask
    .into_iter()
    .flat_map(|value| {
      let gray = (value * 255.0).round().clamp(0.0, 255.0) as u8;
      [gray, gray, gray, 255]
    })
    .collect();
  GpuAux {
    width: p_width,
    height: p_height,
    rgba: Arc::from(rgba),
  }
}

fn supplied_skin_mask_values(p_mask: &Mask, p_width: u32, p_height: u32, p_feather: f32) -> Vec<f32> {
  let mut mask_image = p_mask.image().clone();
  mask_image.resize(ResizeTarget::Exact(Size::new(p_width, p_height)), TransformAlgorithm::Bilinear);
  let (width, height) = (p_width as usize, p_height as usize);
  let mut values: Vec<f32> = mask_image.rgba().par_chunks_exact(4).map(|pixel| pixel[0] as f32 / 255.0).collect();
  feather_mask(&mut values, width, height, p_feather);
  values
}

/// How much each pixel of the photo is skin, from 0 to 1, one value per pixel: skin-colored pixels that are not on or
/// near an edge, with the border faded out. `p_feather` is how far it fades, as a multiple of the standard fade: 1 is the
/// standard, 2 is twice as wide, and 0 is as narrow as it can be.
pub(super) fn skin_mask(p_image: &Image, p_feather: f32) -> Vec<f32> {
  let (w, h) = p_image.dimensions::<usize>();
  let long_side = w.max(h) as f32;

  // How much each pixel looks like skin, from its color alone: 1 inside the skin ranges of Cb and Cr, falling to 0
  // across `CHROMA_MARGIN` outside them, and 0 for pixels too dark to have a color.
  let mut mask: Vec<f32> = p_image
    .rgba()
    .par_chunks_exact(4)
    .map(|pixel| {
      let (y, cb, cr) = rgb_to_ycbcr(pixel[0], pixel[1], pixel[2]);
      let ramp = |value: f32, (low, high): (f32, f32)| {
        ((value - (low - CHROMA_MARGIN)) / CHROMA_MARGIN)
          .min(((high + CHROMA_MARGIN) - value) / CHROMA_MARGIN)
          .clamp(0.0, 1.0)
      };
      ramp(cb, CB_RANGE) * ramp(cr, CR_RANGE) * ((y - Y_DARK) / (Y_FULL - Y_DARK)).clamp(0.0, 1.0)
    })
    .collect();

  // How much to keep each pixel out of the effect because it is on or near an edge. An edge spread over more pixels
  // in a big photo measures weaker per pixel, so it is measured with the taps spaced out, at the reference size.
  let edge_step = ((long_side / REFERENCE_SIZE).round() as usize).max(1);
  let mut protection: Vec<f32> = sobel_magnitude(p_image, edge_step)
    .into_par_iter()
    .map(|edge| {
      let t = ((edge - EDGE_START) / (EDGE_FULL - EDGE_START)).clamp(0.0, 1.0);
      t * t * (3.0 - 2.0 * t)
    })
    .collect();
  let spread = ((long_side * EDGE_SPREAD).round() as usize).clamp(1, EDGE_SPREAD_MAX);
  box_blur_f32_inplace(&mut protection, w, h, spread);
  mask.par_iter_mut().zip(protection.par_iter()).for_each(|(value, kept)| *value *= 1.0 - (kept * EDGE_GAIN).min(1.0));

  // Fade the border of the mask so the effect does not end in a line.
  feather_mask(&mut mask, w, h, p_feather);
  mask
}

/// The same steps as [`skin_mask`], as the first six shader passes. The mask travels in the alpha channel from pass to
/// pass; a pass after these reads it there, and the original photo, which has the real alpha, comes back for the last
/// pass. Every number the shaders use comes from the constants above, as uniforms, so the CPU and GPU cannot drift apart.
///
/// 1. `skin_edges.wgsl`: edge strength, into alpha. 2 and 3. `plane_blur.wgsl`: widens it, across then down.
/// 4. `skin_combine.wgsl`: skin color times the part not protected, into alpha. 5 and 6. `plane_blur.wgsl`: feathers it.
pub(super) fn skin_mask_passes(p_width: u32, p_height: u32, p_feather: f32) -> Vec<GpuPass> {
  let long_side = p_width.max(p_height) as f32;
  let edge_step = ((long_side / REFERENCE_SIZE).round() as u32).max(1);
  let spread = ((long_side * EDGE_SPREAD).round() as u32).clamp(1, EDGE_SPREAD_MAX as u32);
  let feather =
    ((long_side * FEATHER_FRACTION * p_feather).round() as u32).clamp(1, (p_width.max(p_height) / 4).max(1));

  let blur =
    |radius: u32, across: bool| GpuPass::new(include_str!("./plane_blur.wgsl"), ints(&[radius, across as u32]));

  let mut edges_uniform = ints(&[edge_step]);
  edges_uniform.extend(floats(&[EDGE_START, EDGE_FULL]));
  vec![
    GpuPass::new(include_str!("./skin_edges.wgsl"), edges_uniform),
    blur(spread, true),
    blur(spread, false),
    GpuPass::new(
      include_str!("./skin_combine.wgsl"),
      floats(&[
        CB_RANGE.0,
        CB_RANGE.1,
        CR_RANGE.0,
        CR_RANGE.1,
        CHROMA_MARGIN,
        Y_DARK,
        Y_FULL,
        EDGE_GAIN,
      ]),
    ),
    blur(feather, true),
    blur(feather, false),
  ]
}

/// Packs floats as the little-endian bytes of a shader uniform.
pub(super) fn floats(p_values: &[f32]) -> Vec<u8> {
  p_values.iter().flat_map(|value| value.to_le_bytes()).collect()
}

/// Packs unsigned integers as the little-endian bytes of a shader uniform.
pub(super) fn ints(p_values: &[u32]) -> Vec<u8> {
  p_values.iter().flat_map(|value| value.to_le_bytes()).collect()
}
