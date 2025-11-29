use abra_core::Image;
use options::Options;

use crate::apply_filter;

fn apply_smooth_skin(p_image: &mut Image, p_amount: f32) {}
/// Smooths the skin in the image.
/// - `p_image`: The image to be processed.
/// - `p_amount`: The amount of smoothing to apply (0.0 to 1.0).
/// - `p_options`: Additional options for the smoothing operation.
pub fn smooth_skin(p_image: &mut Image, p_amount: impl Into<f64>, p_options: impl Into<Options>) {
  let amount = p_amount.into().clamp(0.0, 1.0) as f32;
  apply_filter!(apply_smooth_skin, p_image, p_options, 1, amount);
}
