use crate::color::to_rgb::lab_to_rgb;

/// Converts RGB color to HSV color space.
/// - `r`: The red channel (0-255).
/// - `g`: The green channel (0-255).
/// - `b`: The blue channel (0-255).
/// Returns a tuple `(H, S, V)` representing the HSV color.
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
  let rf = r as f32 / 255.0;
  let gf = g as f32 / 255.0;
  let bf = b as f32 / 255.0;
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
/// - `h`: The hue component (0-360).
/// - `s`: The saturation component (0-1).
/// - `l`: The lightness component (0-1).
/// Returns a tuple `(H, S, V)` representing the HSV color.
pub fn hsl_to_hsv(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
  let v = l + s * l.min(1.0 - l);
  let sv = if v == 0.0 { 0.0 } else { 2.0 * (1.0 - l / v) };
  (h, sv, v)
}
/// Converts Lab color to HSV color space.
/// - `l`: The lightness component (0-100).
/// - `a`: The a component (-128 to 127).
/// - `b`: The b component (-128 to 127).
/// Returns a tuple `(H, S, V)` representing the HSV color.
pub fn lab_to_hsv(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
  let (r, g, b) = lab_to_rgb(l, a, b);
  rgb_to_hsv(r, g, b)
}
