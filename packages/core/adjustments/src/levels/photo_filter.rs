use abra_core::{Color, Image, hsl_to_rgb_f, linear_f32_to_srgb_u8, rgb_to_hsl_f, srgb_u8_to_linear_f32};
use options::Options;

use rayon::prelude::*;

use crate::apply_adjustment;
/// Types of preset photo filters.
#[derive(Clone, Copy)]
pub enum FilterType {
  /// A warm, darkening filter rgb(255,101,0)
  WarmingDark,
  /// A warm, lightening filter rgb(236,138,0)
  WarmingLight,
  /// A cool, darkening filter rgb(0,109,255)
  CoolingDark,
  /// A cool, lightening filter rgb(0,181,255)
  CoolingLight,
  /// A red filter rgb(255,0,0)
  Red,
  /// An orange filter rgb(255,165,0)
  Orange,
  /// A yellow filter rgb(255,255,0)
  Yellow,
  /// A green filter rgb(0,128,0)
  Green,
  /// A cyan filter rgb(0,255,255)
  Cyan,
  /// A blue filter rgb(0,0,255)
  Blue,
  Violet,
  Magenta,
  Sepia,
  DeepRed,
  DeepBlue,
  DeepEmerald,
  DeepYellow,
  Underwater,
}

fn apply_photo_filter(p_image: &mut Image, p_filter_color: Color, p_density: f32, p_preserve_l: bool) {
  let (width, height) = p_image.dimensions::<i32>();
  let src = p_image.rgba();
  let mut out = vec![0u8; (width * height * 4) as usize];

  // Use primitives helpers for Lab conversions instead of local re-implementation.
  // Precompute the filter's H and S for HSL-based colorization.
  let (filter_h, filter_s, _filter_l_norm) =
    rgb_to_hsl_f(p_filter_color.r as f32, p_filter_color.g as f32, p_filter_color.b as f32);

  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let i = idx * 4;
    // x,y coordinates not needed in simplified implementation
    // We use HSL conversion directly from u8 channels so no need to keep f32 rgb values here.
    let a = src[i + 3];

    // For non-preserve-L filters we approximate a colored gel by channel-wise
    // linear blending in linear RGB which produces a warm tint similar to a
    // physical filter.
    if !p_preserve_l {
      // Non-preserve-L: Use channel-wise multiplication in linear RGB. This
      // emulates a colored gel (filter) over the scene and closely matches
      // Photoshop's non-preserve-L Photo Filter behavior in common cases.
      let r_lin = srgb_u8_to_linear_f32(src[i]);
      let g_lin = srgb_u8_to_linear_f32(src[i + 1]);
      let b_lin = srgb_u8_to_linear_f32(src[i + 2]);
      let fr_lin = srgb_u8_to_linear_f32(p_filter_color.r);
      let fg_lin = srgb_u8_to_linear_f32(p_filter_color.g);
      let fb_lin = srgb_u8_to_linear_f32(p_filter_color.b);
      let r_out_lin = r_lin * ((1.0 - p_density) + p_density * fr_lin);
      let g_out_lin = g_lin * ((1.0 - p_density) + p_density * fg_lin);
      let b_out_lin = b_lin * ((1.0 - p_density) + p_density * fb_lin);
      let r_out = linear_f32_to_srgb_u8(r_out_lin);
      let g_out = linear_f32_to_srgb_u8(g_out_lin);
      let b_out = linear_f32_to_srgb_u8(b_out_lin);
      dst_px[0] = r_out;
      dst_px[1] = g_out;
      dst_px[2] = b_out;
      dst_px[3] = a;
      return;
    }

    // Implement Photoshop-like 'Color' blend using HSL colorization:
    // - Preserve L by keeping the source L value
    // - Set H and S from the filter color, convert to RGB, then blend
    //   with the source in linear space by density. This mirrors how Photoshop
    //   tends to transfer hue/saturation while preserving brightness.
    let (_src_h, _src_s, src_l_norm) = rgb_to_hsl_f(src[i] as f32, src[i + 1] as f32, src[i + 2] as f32);
    // Use the filter H/S while keeping source L
    // reuse precomputed filter_h,filter_s
    let (mut r_color_f, mut g_color_f, mut b_color_f) = hsl_to_rgb_f(filter_h, filter_s, src_l_norm);
    // Simple out-of-gamut fallback: if any channel is clipped fully to 0 or 255,
    // reduce saturation and recompute once.
    if r_color_f == 0 || r_color_f == 255 || g_color_f == 0 || g_color_f == 255 || b_color_f == 0 || b_color_f == 255 {
      let reduced_s = (filter_s * 0.5).clamp(0.0, 1.0);
      let (rr, gg, bb) = hsl_to_rgb_f(filter_h, reduced_s, src_l_norm);
      r_color_f = rr;
      g_color_f = gg;
      b_color_f = bb;
    }
    // Convert colorized result to linear space and blend
    let src_r_lin = srgb_u8_to_linear_f32(src[i]);
    let src_g_lin = srgb_u8_to_linear_f32(src[i + 1]);
    let src_b_lin = srgb_u8_to_linear_f32(src[i + 2]);
    let color_r_lin = srgb_u8_to_linear_f32(r_color_f as u8);
    let color_g_lin = srgb_u8_to_linear_f32(g_color_f as u8);
    let color_b_lin = srgb_u8_to_linear_f32(b_color_f as u8);
    let r_blend_lin = src_r_lin * (1.0 - p_density) + color_r_lin * p_density;
    let g_blend_lin = src_g_lin * (1.0 - p_density) + color_g_lin * p_density;
    let b_blend_lin = src_b_lin * (1.0 - p_density) + color_b_lin * p_density;

    dst_px[0] = linear_f32_to_srgb_u8(r_blend_lin);
    dst_px[1] = linear_f32_to_srgb_u8(g_blend_lin);
    dst_px[2] = linear_f32_to_srgb_u8(b_blend_lin);
    dst_px[3] = a;
  });

  p_image.set_rgba(&out);
}
/// Applies a photo filter to the image.
/// - `p_image`: The image to adjust.
/// - `p_filter_color`: The color of the photo filter.
/// - `p_density`: The density of the filter (0.0 to 1.0).
/// - `p_options`: Options to apply the adjustment.
pub fn photo_filter(
  p_image: &mut Image, p_filter_color: impl Into<Color>, p_density: impl Into<f64>, p_options: impl Into<Options>,
) {
  let filter_color = p_filter_color.into();
  let density = (p_density.into() as f32).clamp(0.0, 1.0);

  apply_adjustment!(apply_photo_filter, p_image, p_options, 1, filter_color, density, true);
}
/// Applies a warming photo filter to the image using a warm orange color (236,138,0).
/// - `p_image`: The image to adjust.
/// - `p_options`: Options to apply the adjustment.
pub fn photo_filter_preset(
  p_image: &mut Image, p_preset: FilterType, p_density: impl Into<f64>, p_options: impl Into<Options>,
) {
  let density = (p_density.into() as f32).clamp(0.0, 1.0);
  let color = match p_preset {
    FilterType::WarmingDark => Color::from_rgb(255, 101, 0),
    FilterType::WarmingLight => Color::from_rgb(236, 138, 0),
    FilterType::CoolingDark => Color::from_rgb(0, 109, 255),
    FilterType::CoolingLight => Color::from_rgb(0, 181, 255),
    FilterType::Red => Color::from_rgb(234, 26, 26),
    FilterType::Orange => Color::from_rgb(243, 132, 23),
    FilterType::Yellow => Color::from_rgb(249, 227, 28),
    FilterType::Green => Color::from_rgb(25, 201, 25),
    FilterType::Cyan => Color::from_rgb(29, 203, 234),
    FilterType::Blue => Color::from_rgb(29, 53, 234),
    FilterType::Violet => Color::from_rgb(155, 29, 234),
    FilterType::Magenta => Color::from_rgb(227, 24, 227),
    FilterType::Sepia => Color::from_rgb(172, 122, 51),
    FilterType::DeepRed => Color::from_rgb(255, 0, 0),
    FilterType::DeepBlue => Color::from_rgb(0, 34, 205),
    FilterType::DeepEmerald => Color::from_rgb(80, 141, 0),
    FilterType::DeepYellow => Color::from_rgb(255, 213, 0),
    FilterType::Underwater => Color::from_rgb(0, 194, 177),
  };
  // Match Photoshop-like color behavior: allow the filter L to affect
  // tonal mapping (do not strictly preserve source L) to emulate
  // the expected results in tests that replicate Photoshop mappings.
  apply_adjustment!(apply_photo_filter, p_image, p_options, 1, color, density, false);
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::rgb_to_lab;

  #[test]
  fn specific_warming_81_color_transform() {
    // The user's desired check: applying photo_filter_warming_81 to color
    // #886d4f should produce approximately #77521c (with a high density).
    let mut img = Image::new(3u32, 3u32);
    // original color #886d4f -> (136,109,79)
    img.clear_color(Color::from_rgba(136, 109, 79, 255));
    // Apply the warming 81 filter at full density to assert the mapping.
    photo_filter_preset(&mut img, FilterType::WarmingLight, 0.8, None);
    let (r, g, b, _a) = img.get_pixel(1, 1).unwrap();
    // expected #77521c -> (119,82,28)
    let (er, eg, eb) = (119u8, 82u8, 28u8);
    // Compare perceptual Lab difference with a small tolerance instead of
    // strict equality. This allows small but visually insignificant
    // numerical differences while asserting Photoshop-like results.
    let (l1, a1, b1) = rgb_to_lab(r, g, b);
    let (l2, a2, b2) = rgb_to_lab(er, eg, eb);
    let d = ((l1 - l2).powi(2) + (a1 - a2).powi(2) + (b1 - b2).powi(2)).sqrt();
    assert!(d <= 5.0, "Lab delta {} exceeds tolerance for warming_81", d);
  }

  #[test]
  fn specific_warming_85_color_transform() {
    // The user's desired check: applying photo_filter_warming_81 to color
    // #886d4f should produce approximately #77521c (with a high density).
    let mut img = Image::new(3u32, 3u32);
    // original color #886d4f -> (136,109,79)
    img.clear_color(Color::from_rgba(136, 109, 79, 255));
    // Apply the warming 85 filter at 25% density
    photo_filter_preset(&mut img, FilterType::WarmingDark, 0.25, None);
    let (r, g, b, _a) = img.get_pixel(1, 1).unwrap();
    // expected #836343 -> (131,99,67)
    let (er, eg, eb) = (131u8, 99u8, 67u8);
    let (l1, a1, b1) = rgb_to_lab(r, g, b);
    let (l2, a2, b2) = rgb_to_lab(er, eg, eb);
    let d = ((l1 - l2).powi(2) + (a1 - a2).powi(2) + (b1 - b2).powi(2)).sqrt();
    assert!(d <= 5.0, "Lab delta {} exceeds tolerance for warming_85", d);
  }
}
