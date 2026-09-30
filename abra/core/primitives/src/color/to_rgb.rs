fn lab_to_linear_rgb(p_l: f32, p_a: f32, p_b: f32) -> (f32, f32, f32) {
  // XYZ <-> Lab helpers (D65 white point)
  const XN: f32 = 0.95047;
  const YN: f32 = 1.00000;
  const ZN: f32 = 1.08883;

  let fy = (p_l + 16.0) / 116.0;
  let fx = p_a / 500.0 + fy;
  let fz = fy - p_b / 200.0;

  const K: f32 = 24389.0 / 27.0;
  let x = if fx.powi(3) > 0.008856 {
    fx.powi(3)
  } else {
    // reverse of f = (k * t + 16) / 116 => t = (116 * f - 16) / k
    (116.0 * fx - 16.0) / K
  } * XN;
  let y = if p_l > (903.3 * 0.008856) { ((p_l + 16.0) / 116.0).powi(3) } else { p_l / 903.3 } * YN;
  let z = if fz.powi(3) > 0.008856 {
    fz.powi(3)
  } else {
    // reverse of f = (k * t + 16) / 116 => t = (116 * f - 16) / k
    (116.0 * fz - 16.0) / K
  } * ZN;

  // XYZ to linear RGB
  let r_lin = 3.2404542 * x - 1.5371385 * y - 0.4985314 * z;
  let g_lin = -0.9692660 * x + 1.8760108 * y + 0.0415560 * z;
  let b_lin = 0.0556434 * x - 0.2040259 * y + 1.0572252 * z;

  (r_lin, g_lin, b_lin)
}
fn linear_to_srgb(p_c: f32) -> f32 {
  if p_c <= 0.0031308 { 12.92 * p_c } else { 1.055 * p_c.powf(1.0 / 2.4) - 0.055 }
}
/// Converts HSV color to RGB color space.
/// - `p_h`: The hue component (0-360).
/// - `p_s`: The saturation component (0-1).
/// - `p_v`: The value component (0-1).
/// Returns a tuple `(R, G, B)` representing the RGB color.
pub fn hsv_to_rgb(p_h: f32, p_s: f32, p_v: f32) -> (u8, u8, u8) {
  let c = p_v * p_s;
  chroma_to_rgb(p_h, c, p_v - c)
}
/// Converts HSL color to RGB color space.
/// - `p_h`: The hue component (0-360).
/// - `p_s`: The saturation component (0-1).
/// - `p_l`: The lightness component (0-1).
/// Returns a tuple `(R, G, B)` representing the RGB color.
pub fn hsl_to_rgb(p_h: f32, p_s: f32, p_l: f32) -> (u8, u8, u8) {
  let c = (1.0 - (2.0 * p_l - 1.0).abs()) * p_s;
  chroma_to_rgb(p_h, c, p_l - c / 2.0)
}
/// Shared final step of the HSV and HSL conversions: places chroma `p_c` on the hue sector for
/// `p_h` (degrees) and adds the lightness offset `p_m`.
fn chroma_to_rgb(p_h: f32, p_c: f32, p_m: f32) -> (u8, u8, u8) {
  let x = p_c * (1.0 - (((p_h / 60.0) % 2.0) - 1.0).abs());
  let (r1, g1, b1) = if p_h < 60.0 {
    (p_c, x, 0.0)
  } else if p_h < 120.0 {
    (x, p_c, 0.0)
  } else if p_h < 180.0 {
    (0.0, p_c, x)
  } else if p_h < 240.0 {
    (0.0, x, p_c)
  } else if p_h < 300.0 {
    (x, 0.0, p_c)
  } else {
    (p_c, 0.0, x)
  };
  (((r1 + p_m) * 255.0).round() as u8, ((g1 + p_m) * 255.0).round() as u8, ((b1 + p_m) * 255.0).round() as u8)
}
/// Converts LAB color to RGB color space.
/// - `p_l`: The lightness component (0-100).
/// - `p_a`: The a component (-128 to 127).
/// - `p_b`: The b component (-128 to 127).
/// Returns a tuple `(R, G, B)` representing the RGB color.
pub fn lab_to_rgb(p_l: f32, p_a: f32, p_b: f32) -> (u8, u8, u8) {
  let (r_lin, g_lin, b_lin) = lab_to_linear_rgb(p_l, p_a, p_b);
  let r_srgb = linear_to_srgb(r_lin).clamp(0.0, 1.0);
  let g_srgb = linear_to_srgb(g_lin).clamp(0.0, 1.0);
  let b_srgb = linear_to_srgb(b_lin).clamp(0.0, 1.0);
  ((r_srgb * 255.0).round() as u8, (g_srgb * 255.0).round() as u8, (b_srgb * 255.0).round() as u8)
}
/// Converts a linear RGB channel (0-1) to an sRGB u8 (0-255) with gamma correction
/// and clamping applied.
/// - `p_c`: The linear RGB channel value (0-1).
/// Returns the sRGB channel value (0-255).
pub fn linear_f32_to_srgb_u8(p_c: f32) -> u8 {
  let out = linear_to_srgb(p_c).clamp(0.0, 1.0);
  (out * 255.0).round() as u8
}
