use crate::common::*;
use abra::filters::prelude::{noise::NoiseDistribution, *};

#[napi]
/// Applies a noise filter to the image.
/// @param layer The layer to apply the noise filter to.
/// @param amount The amount of noise to add.
/// @param distribution The distribution type of the noise ("uniform" or "gaussian").
pub fn noise(layer: &mut Layer, amount: f64, distribution: String) {
  let layer_ref = layer.get_underlying_layer_mut();
  noise::noise(
    &mut *layer_ref,
    amount as f32,
    match distribution.as_str() {
      "uniform" => NoiseDistribution::Uniform,
      "gaussian" => NoiseDistribution::Gaussian,
      _ => NoiseDistribution::Uniform,
    },
    None,
  );
  println!("Added noise: amount={}, distribution={}", amount, distribution);
  layer.mark_dirty();
}

#[napi]
pub fn despeckle<'a>(layer: &mut Layer, radius: f64, threshold: f64) {
  let layer_ref = layer.get_underlying_layer_mut();
  noise::despeckle(layer_ref, radius as f32, threshold as f32, None);
  layer.mark_dirty();
}

#[napi]
pub fn median<'a>(layer: &mut Layer, radius: f64) {
  let layer_ref = layer.get_underlying_layer_mut();
  noise::median(layer_ref, radius as f32, None);
  layer.mark_dirty();
}
