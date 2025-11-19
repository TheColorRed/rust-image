/// Converts an RGB color to HSV.
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
  let r = r as f32 / 255.0;
  let g = g as f32 / 255.0;
  let b = b as f32 / 255.0;

  let max = r.max(g).max(b);
  let min = r.min(g).min(b);
  let delta = max - min;

  let h = if delta == 0.0 {
    0.0
  } else if max == r {
    60.0 * (((g - b) / delta) % 6.0)
  } else if max == g {
    60.0 * ((b - r) / delta + 2.0)
  } else {
    60.0 * ((r - g) / delta + 4.0)
  };

  let s = if max == 0.0 { 0.0 } else { delta / max };

  let v = max;

  (h, s, v)
}
