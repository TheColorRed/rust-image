use super::luma::{LumaStandard, luma};

/// Converts RGB color to YCbCr color space (full range, ITU-R BT.601), which keeps brightness apart from color.
///
/// That separation makes it good at telling colors apart in different lighting: a patch of skin in shadow and in sun
/// has very different brightness (`Y`) but a similar color (`Cb`, `Cr`).
/// - `p_r`: The red channel (0-255).
/// - `p_g`: The green channel (0-255).
/// - `p_b`: The blue channel (0-255).
/// Returns a tuple `(Y, Cb, Cr)`, each 0-255. `Cb` and `Cr` are 128 for any gray.
pub fn rgb_to_ycbcr(p_r: u8, p_g: u8, p_b: u8) -> (f32, f32, f32) {
  let (r, g, b) = (p_r as f32, p_g as f32, p_b as f32);
  let y = luma(r, g, b, LumaStandard::Rec601);
  let cb = 128.0 + 0.564 * (b - y);
  let cr = 128.0 + 0.713 * (r - y);
  (y, cb, cr)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn gray_has_no_color() {
    for level in [0u8, 64, 128, 255] {
      let (y, cb, cr) = rgb_to_ycbcr(level, level, level);
      assert!((y - level as f32).abs() < 0.01);
      assert!((cb - 128.0).abs() < 0.01 && (cr - 128.0).abs() < 0.01, "gray {level}: cb {cb}, cr {cr}");
    }
  }

  #[test]
  fn red_leans_toward_cr_and_blue_toward_cb() {
    let (_, cb_red, cr_red) = rgb_to_ycbcr(255, 0, 0);
    assert!(cr_red > 200.0 && cb_red < 128.0);
    let (_, cb_blue, cr_blue) = rgb_to_ycbcr(0, 0, 255);
    assert!(cb_blue > 200.0 && cr_blue < 128.0);
  }

  #[test]
  fn skin_tone_lands_in_the_usual_skin_range() {
    // 77-127 for Cb and 133-173 for Cr is the range commonly used to find skin.
    let (_, cb, cr) = rgb_to_ycbcr(230, 190, 150);
    assert!((77.0..=127.0).contains(&cb) && (133.0..=173.0).contains(&cr), "cb {cb}, cr {cr}");
  }
}
