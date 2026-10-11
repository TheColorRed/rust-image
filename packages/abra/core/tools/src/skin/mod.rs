mod smooth;
mod tan;
mod tone;

use mask::Mask;

pub use smooth::{SkinSmooth, skin_smooth};
pub use tan::{SkinTan, skin_tan};
pub use tone::{SkinTone, skin_tone};

/// Scales every mask weight without changing its dimensions.
fn scale_mask(p_mask: &Mask, p_amount: f32) -> Mask {
  let amount = p_amount.clamp(0.0, 1.0);
  let (width, height) = p_mask.dimensions::<u32>();
  let values = p_mask.values().iter().map(|value| (*value as f32 * amount).round() as u8).collect();
  Mask::from_values(width, height, values)
}
