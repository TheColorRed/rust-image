/// Converts RGB color to HSL color space.
/// - `p_r`: The red channel (0-255).
/// - `p_g`: The green channel (0-255).
/// - `p_b`: The blue channel (0-255).
/// Returns a tuple `(H, S, L)` with hue in degrees (0-360) and saturation/lightness in 0-1.
pub fn rgb_to_hsl(p_r: u8, p_g: u8, p_b: u8) -> (f32, f32, f32) {
  let (h, max, min) = hue_max_min(p_r, p_g, p_b);
  let l = (max + min) / 2.0;
  let d = max - min;
  let s = if d == 0.0 {
    0.0
  } else if l > 0.5 {
    d / (2.0 - max - min)
  } else {
    d / (max + min)
  };
  (h, s, l)
}

/// Shared hue computation for the HSL and HSV conversions.
///
/// Returns `(hue in degrees, max channel, min channel)` with channels normalized to 0-1.
pub(crate) fn hue_max_min(p_r: u8, p_g: u8, p_b: u8) -> (f32, f32, f32) {
  let rf = p_r as f32 / 255.0;
  let gf = p_g as f32 / 255.0;
  let bf = p_b as f32 / 255.0;
  let max = rf.max(gf).max(bf);
  let min = rf.min(gf).min(bf);
  let d = max - min;
  if d == 0.0 {
    return (0.0, max, min);
  }
  let h = if max == rf {
    (gf - bf) / d + if gf < bf { 6.0 } else { 0.0 }
  } else if max == gf {
    (bf - rf) / d + 2.0
  } else {
    (rf - gf) / d + 4.0
  };
  (h * 60.0, max, min)
}
