use crate::common::*;

#[napi]
/// Applies a pinch distortion effect to the image.
/// @param layer The layer to apply the pinch effect to.
/// @param amount The amount of pinch effect to apply. Positive values pinch inward, negative values bulge outward.
pub fn pinch(layer: &mut Layer, amount: f64) {
  let layer_ref = layer.get_underlying_layer_mut();
  distort::pinch(&mut *layer_ref, amount as f32, None);
  layer.mark_dirty();
}

#[napi]
/// Applies a ripple distortion effect to the image.
/// @param layer The layer to apply the ripple effect to.
/// @param amount The amount of ripple effect to apply. Positive values create inward ripples, negative values create outward ripples.
/// @param size The size of the ripple effect. Can be "small", "medium", or "large".
/// @param shape The shape of the ripple effect. Can be "circular", "square", "random", or an angle (0-360 degrees, where 0 = vertical, 90 = horizontal).
pub fn ripple(
  layer: &mut Layer, amount: f64, #[napi(ts_arg_type = "\"small\" | \"medium\" | \"large\"")] size: String,
  #[napi(ts_arg_type = "\"circular\" | \"square\" | \"random\" | number")] shape: napi::Either<String, f64>,
) {
  let layer_ref = layer.get_underlying_layer_mut();
  let ripple_size = match size.as_str() {
    "small" => distort::RippleSize::Small,
    "medium" => distort::RippleSize::Medium,
    "large" => distort::RippleSize::Large,
    _ => distort::RippleSize::Medium,
  };
  let ripple_shape = match shape {
    napi::Either::A(shape_str) => match shape_str.as_str() {
      "circular" => distort::RippleShape::Circular,
      "square" => distort::RippleShape::Square,
      "random" => distort::RippleShape::Random,
      _ => distort::RippleShape::Circular,
    },
    napi::Either::B(angle) => distort::RippleShape::Angle(angle as f32),
  };
  distort::ripple(&mut *layer_ref, amount as f32, ripple_size, ripple_shape, None);
  layer.mark_dirty();
}
