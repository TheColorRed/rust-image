use crate::color::{hsl_to_rgb, hsv_to_rgb};

fn srgb_to_linear(c: f32) -> f32 {
  if c <= 0.04045 {
    c / 12.92
  } else {
    ((c + 0.055) / 1.055).powf(2.4)
  }
}
fn linear_rgb_to_xyz(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
  let x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b;
  let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
  let z = 0.0193339 * r + 0.1191920 * g + 0.9503041 * b;
  (x, y, z)
}
fn f_xyz(t: f32) -> f32 {
  const EPS: f32 = 216.0 / 24389.0; // (6/29)^3
  const K: f32 = 24389.0 / 27.0; // (29/6)^3
  if t > EPS {
    t.powf(1.0 / 3.0)
  } else {
    (K * t + 16.0) / 116.0
  }
}
fn linear_rgb_to_lab(r_lin: f32, g_lin: f32, b_lin: f32) -> (f32, f32, f32) {
  // XYZ <-> Lab helpers (D65 white point)
  const XN: f32 = 0.95047;
  const YN: f32 = 1.00000;
  const ZN: f32 = 1.08883;

  let (x, y, z) = linear_rgb_to_xyz(r_lin, g_lin, b_lin);
  let fx = f_xyz(x / XN);
  let fy = f_xyz(y / YN);
  let fz = f_xyz(z / ZN);
  let l = 116.0 * fy - 16.0;
  let a = 500.0 * (fx - fy);
  let b = 200.0 * (fy - fz);
  (l, a, b)
}
/// Converts sRGB color to Lab color space.
/// - `r`: The red channel (0-255).
/// - `g`: The green channel (0-255).
/// - `b`: The blue channel (0-255).
/// Returns a tuple `(L, a, b)` representing the Lab color.
pub fn rgb_to_lab(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
  let r_lin = srgb_to_linear(r as f32 / 255.0);
  let g_lin = srgb_to_linear(g as f32 / 255.0);
  let b_lin = srgb_to_linear(b as f32 / 255.0);

  linear_rgb_to_lab(r_lin, g_lin, b_lin)
}
/// Converts HSL color to Lab color space.
/// - `h`: The hue component (0-360).
/// - `s`: The saturation component (0-1).
/// - `l`: The lightness component (0-1).
/// Returns a tuple `(L, a, b)` representing the Lab color.
pub fn hsl_to_lab(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
  let (r, g, b) = hsl_to_rgb(h, s, l);
  rgb_to_lab(r, g, b)
}
/// Converts HSV color to Lab color space.
/// - `h`: The hue component (0-360).
/// - `s`: The saturation component (0-1).
/// - `v`: The value component (0-1).
/// Returns a tuple `(L, a, b)` representing the Lab color.
pub fn hsv_to_lab(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
  let (r, g, b) = hsv_to_rgb(h, s, v);
  rgb_to_lab(r, g, b)
}
/// Converts an sRGB channel represented as u8 (0-255) to linear f32 (0-1).
/// - `v`: The sRGB channel value (0-255).
/// Returns the linear channel value (0-1).
pub fn srgb_u8_to_linear_f32(v: u8) -> f32 {
  srgb_to_linear(v as f32 / 255.0)
}
