/// The weighting standard used to compute luma (perceived brightness) from RGB.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LumaStandard {
  /// ITU-R BT.601 weights (0.299, 0.587, 0.114). The classic "grayscale" weights.
  #[default]
  Rec601,
  /// ITU-R BT.709 weights (0.2126, 0.7152, 0.0722). Used for sRGB relative luminance.
  Rec709,
}

impl LumaStandard {
  /// The `(r, g, b)` weights for this standard. They sum to 1.
  #[inline]
  pub const fn weights(&self) -> (f32, f32, f32) {
    match self {
      LumaStandard::Rec601 => (0.299, 0.587, 0.114),
      LumaStandard::Rec709 => (0.2126, 0.7152, 0.0722),
    }
  }
}

/// Computes the weighted luma of an RGB triple.
///
/// The result is in the same range as the inputs (e.g. 0-255 in, 0-255 out; 0-1 in, 0-1 out).
///
/// - `p_r`, `p_g`, `p_b`: The channel values.
/// - `p_standard`: The weighting standard to use.
#[inline]
pub fn luma(p_r: f32, p_g: f32, p_b: f32, p_standard: LumaStandard) -> f32 {
  let (wr, wg, wb) = p_standard.weights();
  wr * p_r + wg * p_g + wb * p_b
}
