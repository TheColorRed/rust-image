use abra_core::{Image, IntoNumber, TransformAlgorithm, color::rgb_to_ycbcr};
use filters::sobel::sobel_magnitude;
use mask::Mask;
use rayon::prelude::*;

const CB_RANGE: (f32, f32) = (77.0, 127.0);
const CR_RANGE: (f32, f32) = (135.0, 173.0);
const CHROMA_MARGIN: f32 = 6.0;
const Y_DARK: f32 = 30.0;
const Y_FULL: f32 = 50.0;
const EDGE_START: f32 = 10.0;
const EDGE_FULL: f32 = 32.0;
const REFERENCE_SIZE: f32 = 1200.0;
const EDGE_SPREAD: f32 = 0.002;
const EDGE_SPREAD_MAX: usize = 24;
const EDGE_GAIN: f32 = 2.5;
const FEATHER_FRACTION: f32 = 0.0045;

fn box_blur_in_place(p_values: &mut [f32], p_width: usize, p_height: usize, p_radius: usize) {
  if p_radius == 0 || p_width == 0 || p_height == 0 {
    return;
  }
  let divisor = (2 * p_radius + 1) as f32;
  let mut scratch = vec![0.0; p_values.len()];
  let blur_rows = |source: &[f32], destination: &mut [f32], width: usize| {
    destination.par_chunks_mut(width).zip(source.par_chunks(width)).for_each(|(out, row)| {
      let mut sum: f32 = row[..(p_radius + 1).min(width)].iter().sum();
      for x in 0..width {
        out[x] = sum / divisor;
        if x + p_radius + 1 < width {
          sum += row[x + p_radius + 1];
        }
        if x >= p_radius {
          sum -= row[x - p_radius];
        }
      }
    });
  };
  let transpose = |source: &[f32], source_width: usize, source_height: usize, destination: &mut [f32]| {
    destination.par_chunks_mut(source_height).enumerate().for_each(|(x, out)| {
      for (y, value) in out.iter_mut().enumerate() {
        *value = source[y * source_width + x];
      }
    });
  };
  blur_rows(p_values, &mut scratch, p_width);
  transpose(&scratch, p_width, p_height, p_values);
  blur_rows(p_values, &mut scratch, p_height);
  transpose(&scratch, p_height, p_width, p_values);
}

fn feather(p_values: &mut [f32], p_width: usize, p_height: usize, p_feather: f32) {
  let long_side = p_width.max(p_height) as f32;
  let radius = (long_side * FEATHER_FRACTION * p_feather).round() as usize;
  box_blur_in_place(p_values, p_width, p_height, radius.clamp(1, (p_width.max(p_height) / 4).max(1)));
}

fn mask_from_values(p_width: u32, p_height: u32, p_values: Vec<f32>) -> Mask {
  let values = p_values.into_iter().map(|value| (value * 255.0).round().clamp(0.0, 255.0) as u8).collect();
  Mask::from_values(p_width, p_height, values)
}

/// Detects skin-colored, non-edge pixels and returns a soft grayscale mask for them.
///
/// `p_feather` is a multiple of the standard edge fade: `1.0` uses the standard width, `2.0` doubles it, and values
/// below zero use the narrowest fade. A segmentation mask can be prepared with [`soften_skin_mask`] instead.
pub fn skin_mask(p_image: &Image, p_feather: impl IntoNumber) -> Mask {
  let (width, height) = p_image.dimensions::<usize>();
  let long_side = width.max(height) as f32;
  let mut values: Vec<f32> = p_image
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
  let edge_step = ((long_side / REFERENCE_SIZE).round() as usize).max(1);
  let mut protection: Vec<f32> = sobel_magnitude(p_image, edge_step)
    .into_par_iter()
    .map(|edge| {
      let t = ((edge - EDGE_START) / (EDGE_FULL - EDGE_START)).clamp(0.0, 1.0);
      t * t * (3.0 - 2.0 * t)
    })
    .collect();
  let spread = ((long_side * EDGE_SPREAD).round() as usize).clamp(1, EDGE_SPREAD_MAX);
  box_blur_in_place(&mut protection, width, height, spread);
  values
    .par_iter_mut()
    .zip(protection.par_iter())
    .for_each(|(value, protected)| *value *= 1.0 - (protected * EDGE_GAIN).min(1.0));
  feather(&mut values, width, height, p_feather.into::<f32>().max(0.0));
  mask_from_values(width as u32, height as u32, values)
}

/// Resizes a supplied skin or segmentation mask to `p_image` and softens its boundary with the same fade used by
/// [`skin_mask`].
pub fn soften_skin_mask(p_image: &Image, p_mask: &Mask, p_feather: impl IntoNumber) -> Mask {
  let (width, height) = p_image.dimensions::<u32>();
  let resized;
  let mask = if p_mask.dimensions::<u32>() == (width, height) {
    p_mask
  } else {
    resized = p_mask.resized(width, height, TransformAlgorithm::Bilinear);
    &resized
  };
  let mut values: Vec<f32> = mask.values().par_iter().map(|value| *value as f32 / 255.0).collect();
  feather(&mut values, width as usize, height as usize, p_feather.into::<f32>().max(0.0));
  mask_from_values(width, height, values)
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Color;

  #[test]
  fn detected_mask_is_the_size_of_the_photo() {
    let image = Image::new_from_color(16, 12, Color::from_rgb(220, 180, 140));
    let mask = skin_mask(&image, 1.0);
    assert_eq!(mask.dimensions::<u32>(), (16, 12));
    assert!(mask.get(8, 6).unwrap() > 0);
  }
}
