use super::to_hsl::hue_max_min;

/// Converts RGB color to HSV color space.
/// - `p_r`: The red channel (0-255).
/// - `p_g`: The green channel (0-255).
/// - `p_b`: The blue channel (0-255).
/// Returns a tuple `(H, S, V)` with hue in degrees (0-360) and saturation/value in 0-1.
pub fn rgb_to_hsv(p_r: u8, p_g: u8, p_b: u8) -> (f32, f32, f32) {
  let (h, max, min) = hue_max_min(p_r, p_g, p_b);
  let s = if max == 0.0 { 0.0 } else { (max - min) / max };
  (h, s, max)
}
