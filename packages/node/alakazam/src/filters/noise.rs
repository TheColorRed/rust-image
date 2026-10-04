use crate::common::*;

#[napi]
/// Applies a noise filter to the image.
/// @param layer The layer to apply the noise filter to.
/// @param amount The amount of noise to add.
/// @param distribution The distribution type of the noise ("uniform" or "gaussian").
/// @param options Optional apply options for masking and area.
pub fn noise(layer: &mut Layer, amount: f64, distribution: String, options: Option<&ApplyOptions>) {
  println!("Added noise: amount={}, distribution={}", amount, distribution);
  noise_impl(layer, amount, distribution, options);
}

fn noise_impl(layer: &mut Layer, amount: f64, distribution: String, options: Option<&ApplyOptions>) {
  // Get an ImageRef that keeps the inner guard alive only for the duration
  // of the filter call, allowing us to mutate the image in-place without
  // cloning the pixels.
  layer.get_underlying_layer().with_image_mut(|img| {
    abra::filters::prelude::noise::noise(amount as f32)
      .with_distribution(match distribution.as_str() {
        "uniform" => abra::filters::prelude::noise::NoiseDistribution::Uniform,
        "gaussian" => abra::filters::prelude::noise::NoiseDistribution::Gaussian,
        _ => abra::filters::prelude::noise::NoiseDistribution::Uniform,
      })
      .with_options(options.unwrap_or(&ApplyOptions::default()).to_apply_options())
      .apply(img);
  });
  layer.mark_dirty();
}

#[napi]
/// Applies a despeckle filter to the image.
/// @param layer The layer to apply the despeckle filter to.
/// @param radius The radius of the despeckle effect.
/// @param threshold The threshold of the despeckle effect.
/// @param options Optional apply options for masking and area.
pub fn despeckle(layer: &mut Layer, radius: f64, threshold: f64, options: Option<&ApplyOptions>) {
  despeckle_impl(layer, radius, threshold, options);
}

fn despeckle_impl(layer: &mut Layer, radius: f64, threshold: f64, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    abra::filters::prelude::noise::despeckle(radius as f32, threshold as f32).with_options(options).apply(img);
  });
  layer.mark_dirty();
}

#[napi]
/// Applies a median filter to the image.
/// @param layer The layer to apply the median filter to.
/// @param radius The radius of the median effect.
/// @param options Optional apply options for masking and area.
pub fn median(layer: &mut Layer, radius: f64, options: Option<&ApplyOptions>) {
  median_impl(layer, radius, options);
}

fn median_impl(layer: &mut Layer, radius: f64, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    abra::filters::prelude::noise::median(radius as f32).with_options(options).apply(img);
  });
  layer.mark_dirty();
}
