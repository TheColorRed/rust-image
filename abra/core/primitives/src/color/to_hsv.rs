use crate::color::to_rgb::lab_to_rgb;

/// Converts RGB color to HSV color space.
/// - `p_r`: The red channel (0-255).
/// - `p_g`: The green channel (0-255).
/// - `p_b`: The blue channel (0-255).
/// Returns a tuple `(H, S, V)` representing the HSV color.
pub fn rgb_to_hsv(p_r: u8, p_g: u8, p_b: u8) -> (f32, f32, f32) {
  let rf = p_r as f32 / 255.0;
  let gf = p_g as f32 / 255.0;
  let bf = p_b as f32 / 255.0;
  let max = rf.max(gf).max(bf);
  let min = rf.min(gf).min(bf);
  let v = max;
  let d = max - min;
  let s = if max == 0.0 { 0.0 } else { d / max };
  let mut h = 0.0;
  if max != min {
    h = if max == rf {
      (gf - bf) / d + (if gf < bf { 6.0 } else { 0.0 })
    } else if max == gf {
      (bf - rf) / d + 2.0
    } else {
      (rf - gf) / d + 4.0
    };
    h *= 60.0;
  }
  (h, s, v)
}
/// Converts HSL color to HSV color space.
/// - `p_h`: The hue component (0-360).
/// - `p_s`: The saturation component (0-1).
/// - `p_l`: The lightness component (0-1).
/// Returns a tuple `(H, S, V)` representing the HSV color.
pub fn hsl_to_hsv(p_h: f32, p_s: f32, p_l: f32) -> (f32, f32, f32) {
  let v = p_l + p_s * p_l.min(1.0 - p_l);
  let sv = if v == 0.0 { 0.0 } else { 2.0 * (1.0 - p_l / v) };
  (p_h, sv, v)
}
/// Converts Lab color to HSV color space.
/// - `p_l`: The lightness component (0-100).
/// - `p_a`: The a component (-128 to 127).
/// - `p_b`: The b component (-128 to 127).
/// Returns a tuple `(H, S, V)` representing the HSV color.
pub fn lab_to_hsv(p_l: f32, p_a: f32, p_b: f32) -> (f32, f32, f32) {
  let (r, g, p_b) = lab_to_rgb(p_l, p_a, p_b);
  rgb_to_hsv(r, g, p_b)
}
