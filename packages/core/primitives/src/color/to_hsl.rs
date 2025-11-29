use crate::color::to_rgb;

/// Converts RGB color to HSL color space.
/// - `r`: The red channel (0-255).
/// - `g`: The green channel (0-255).
/// - `b`: The blue channel (0-255).
/// Returns a tuple `(H, S, L)` representing the HSL color.
pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
  let rf = r as f32 / 255.0;
  let gf = g as f32 / 255.0;
  let bf = b as f32 / 255.0;
  let max = rf.max(gf).max(bf);
  let min = rf.min(gf).min(bf);
  let l = (max + min) / 2.0;
  let mut h = 0.0;
  let mut s = 0.0;
  if max != min {
    let d = max - min;
    s = if l > 0.5 {
      d / (2.0 - max - min)
    } else {
      d / (max + min)
    };
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
/// - `h`: The hue component (0-360).
/// - `s`: The saturation component (0-1).
/// - `v`: The value component (0-1).
/// Returns a tuple `(H, S, L)` representing the HSL color.
pub fn hsv_to_hsl(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
  let l = v * (1.0 - s / 2.0);
  let s_l = if l == 0.0 || l == 1.0 {
    0.0
  } else {
    (v - l) / l.min(1.0 - l)
  };
  (h, s_l, l)
}
/// Converts LAB color to HSL color space.
/// - `l`: The lightness component (0-100).
/// - `a`: The a component (-128 to 127).
/// - `b`: The b component (-128 to 127).
/// Returns a tuple `(H, S, L)` representing the HSL color.
pub fn lab_to_hsl(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
  let (r, g, b) = to_rgb::lab_to_rgb(l, a, b);
  rgb_to_hsl(r, g, b)
}
/// Converts RGB color to HSL color space using f32 channel inputs and returns a normalized
/// hue in the range [0, 1] (as the original helper used in other callers), with S and L in [0,1].
pub fn rgb_to_hsl_f(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
  let r = r / 255.0;
  let g = g / 255.0;
  let b = b / 255.0;
  let max = r.max(g.max(b));
  let min = r.min(g.min(b));
  let l = (max + min) / 2.0;
  if (max - min).abs() < 1e-5 {
    return (0.0, 0.0, l);
  }
  let d = max - min;
  let s = if l > 0.5 {
    d / (2.0 - max - min)
  } else {
    d / (max + min)
  };
  let h = if (max - r).abs() < 1e-6 {
    (g - b) / d + if g < b { 6.0 } else { 0.0 }
  } else if (max - g).abs() < 1e-6 {
    (b - r) / d + 2.0
  } else {
    (r - g) / d + 4.0
  } / 6.0;
  (h, s, l)
}
