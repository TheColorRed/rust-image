use crate::common::*;
use abra::filters::prelude::{noise::NoiseDistribution, *};

#[napi]
/// Applies a noise filter to the image.
/// @param layer The layer to apply the noise filter to.
/// @param amount The amount of noise to add.
/// @param distribution The distribution type of the noise ("uniform" or "gaussian").
/// @param options Optional apply options for masking and area.
pub fn noise(layer: &mut Layer, amount: f64, distribution: String, options: Option<&ApplyOptions>) {
  let layer_ref = layer.get_underlying_layer_mut();
  noise::noise(
    &mut *layer_ref,
    amount as f32,
    match distribution.as_str() {
      "uniform" => NoiseDistribution::Uniform,
      "gaussian" => NoiseDistribution::Gaussian,
      _ => NoiseDistribution::Uniform,
    },
    options.unwrap_or(&ApplyOptions::default()).to_apply_options(),
  );
  println!("Added noise: amount={}, distribution={}", amount, distribution);
  layer.mark_dirty();
}

#[napi]
/// Applies a despeckle filter to the image.
/// @param layer The layer to apply the despeckle filter to.
/// @param radius The radius of the despeckle effect.
/// @param threshold The threshold of the despeckle effect.
/// @param options Optional apply options for masking and area.
pub fn despeckle<'a>(layer: &mut Layer, radius: f64, threshold: f64, options: Option<&ApplyOptions>) {
  let layer_ref = layer.get_underlying_layer_mut();
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  noise::despeckle(layer_ref, radius as f32, threshold as f32, options);
  layer.mark_dirty();
}

#[napi]
/// Applies a median filter to the image.
/// @param layer The layer to apply the median filter to.
/// @param radius The radius of the median effect.
/// @param options Optional apply options for masking and area.
pub fn median<'a>(layer: &mut Layer, radius: f64, options: Option<&ApplyOptions>) {
  let layer_ref = layer.get_underlying_layer_mut();
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  noise::median(layer_ref, radius as f32, Some(options));
  layer.mark_dirty();
}
