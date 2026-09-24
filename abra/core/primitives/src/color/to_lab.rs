use crate::color::{hsl_to_rgb, hsv_to_rgb};

fn srgb_to_linear(p_c: f32) -> f32 {
  if p_c <= 0.04045 { p_c / 12.92 } else { ((p_c + 0.055) / 1.055).powf(2.4) }
}
fn linear_rgb_to_xyz(p_r: f32, p_g: f32, p_b: f32) -> (f32, f32, f32) {
  let x = 0.4124564 * p_r + 0.3575761 * p_g + 0.1804375 * p_b;
  let y = 0.2126729 * p_r + 0.7151522 * p_g + 0.0721750 * p_b;
  let z = 0.0193339 * p_r + 0.1191920 * p_g + 0.9503041 * p_b;
  (x, y, z)
}
fn f_xyz(p_t: f32) -> f32 {
  const EPS: f32 = 216.0 / 24389.0; // (6/29)^3
  const K: f32 = 24389.0 / 27.0; // (29/6)^3
  if p_t > EPS { p_t.powf(1.0 / 3.0) } else { (K * p_t + 16.0) / 116.0 }
}
fn linear_rgb_to_lab(p_r_lin: f32, p_g_lin: f32, p_b_lin: f32) -> (f32, f32, f32) {
  // XYZ <-> Lab helpers (D65 white point)
  const XN: f32 = 0.95047;
  const YN: f32 = 1.00000;
  const ZN: f32 = 1.08883;

  let (x, y, z) = linear_rgb_to_xyz(p_r_lin, p_g_lin, p_b_lin);
  let fx = f_xyz(x / XN);
  let fy = f_xyz(y / YN);
  let fz = f_xyz(z / ZN);
  let l = 116.0 * fy - 16.0;
  let a = 500.0 * (fx - fy);
  let b = 200.0 * (fy - fz);
  (l, a, b)
}
/// Converts sRGB color to Lab color space.
/// - `p_r`: The red channel (0-255).
/// - `p_g`: The green channel (0-255).
/// - `p_b`: The blue channel (0-255).
/// Returns a tuple `(L, a, b)` representing the Lab color.
pub fn rgb_to_lab(p_r: u8, p_g: u8, p_b: u8) -> (f32, f32, f32) {
  let r_lin = srgb_to_linear(p_r as f32 / 255.0);
  let g_lin = srgb_to_linear(p_g as f32 / 255.0);
  let b_lin = srgb_to_linear(p_b as f32 / 255.0);

  linear_rgb_to_lab(r_lin, g_lin, b_lin)
}
/// Converts HSL color to Lab color space.
/// - `p_h`: The hue component (0-360).
/// - `p_s`: The saturation component (0-1).
/// - `p_l`: The lightness component (0-1).
/// Returns a tuple `(L, a, b)` representing the Lab color.
pub fn hsl_to_lab(p_h: f32, p_s: f32, p_l: f32) -> (f32, f32, f32) {
  let (r, g, b) = hsl_to_rgb(p_h, p_s, p_l);
  rgb_to_lab(r, g, b)
}
/// Converts HSV color to Lab color space.
/// - `p_h`: The hue component (0-360).
/// - `p_s`: The saturation component (0-1).
/// - `p_v`: The value component (0-1).
/// Returns a tuple `(L, a, b)` representing the Lab color.
pub fn hsv_to_lab(p_h: f32, p_s: f32, p_v: f32) -> (f32, f32, f32) {
  let (r, g, b) = hsv_to_rgb(p_h, p_s, p_v);
  rgb_to_lab(r, g, b)
}
/// Converts an sRGB channel represented as u8 (0-255) to linear f32 (0-1).
/// - `p_v`: The sRGB channel value (0-255).
/// Returns the linear channel value (0-1).
pub fn srgb_u8_to_linear_f32(p_v: u8) -> f32 {
  srgb_to_linear(p_v as f32 / 255.0)
}
