use crate::color::to_rgb;

/// Converts RGB color to HSL color space.
/// - `p_r`: The red channel (0-255).
/// - `p_g`: The green channel (0-255).
/// - `p_b`: The blue channel (0-255).
/// Returns a tuple `(H, S, L)` representing the HSL color.
pub fn rgb_to_hsl(p_r: u8, p_g: u8, p_b: u8) -> (f32, f32, f32) {
  let rf = p_r as f32 / 255.0;
  let gf = p_g as f32 / 255.0;
  let bf = p_b as f32 / 255.0;
  let max = rf.max(gf).max(bf);
  let min = rf.min(gf).min(bf);
  let l = (max + min) / 2.0;
  let mut h = 0.0;
  let mut s = 0.0;
  if max != min {
    let d = max - min;
    s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    h = if max == rf {
      (gf - bf) / d + (if gf < bf { 6.0 } else { 0.0 })
    } else if max == gf {
      (bf - rf) / d + 2.0
    } else {
      (rf - gf) / d + 4.0
    };
    h *= 60.0;
  }
  (h, s, l)
}
/// Converts HSV color to HSL color space.
/// - `p_h`: The hue component (0-360).
/// - `p_s`: The saturation component (0-1).
/// - `p_v`: The value component (0-1).
/// Returns a tuple `(H, S, L)` representing the HSL color.
pub fn hsv_to_hsl(p_h: f32, p_s: f32, p_v: f32) -> (f32, f32, f32) {
  let l = p_v * (1.0 - p_s / 2.0);
  let s_l = if l == 0.0 || l == 1.0 { 0.0 } else { (p_v - l) / l.min(1.0 - l) };
  (p_h, s_l, l)
}
/// Converts LAB color to HSL color space.
/// - `p_l`: The lightness component (0-100).
/// - `p_a`: The a component (-128 to 127).
/// - `p_b`: The b component (-128 to 127).
/// Returns a tuple `(H, S, L)` representing the HSL color.
pub fn lab_to_hsl(p_l: f32, p_a: f32, p_b: f32) -> (f32, f32, f32) {
  let (r, g, p_b) = to_rgb::lab_to_rgb(p_l, p_a, p_b);
  rgb_to_hsl(r, g, p_b)
}
/// Converts RGB color to HSL color space using f32 channel inputs and returns a normalized
/// hue in the range [0, 1] (as the original helper used in other callers), with S and L in [0,1].
pub fn rgb_to_hsl_f(p_r: f32, p_g: f32, p_b: f32) -> (f32, f32, f32) {
  let p_r = p_r / 255.0;
  let p_g = p_g / 255.0;
  let p_b = p_b / 255.0;
  let max = p_r.max(p_g.max(p_b));
  let min = p_r.min(p_g.min(p_b));
  let l = (max + min) / 2.0;
  if (max - min).abs() < 1e-5 {
    return (0.0, 0.0, l);
  }
  let d = max - min;
  let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
  let h = if (max - p_r).abs() < 1e-6 {
    (p_g - p_b) / d + if p_g < p_b { 6.0 } else { 0.0 }
  } else if (max - p_g).abs() < 1e-6 {
    (p_b - p_r) / d + 2.0
  } else {
    (p_r - p_g) / d + 4.0
  } / 6.0;
  (h, s, l)
}
