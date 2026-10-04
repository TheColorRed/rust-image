use crate::apply_options::ApplyOptions;
use crate::common::*;
use abra::adjustments::prelude::*;

#[napi]
/// Automatically adjusts the colors of a layer.
/// @param layer The layer to adjust.
/// @param options Optional adjustment options, including area and mask.
pub fn auto_color(layer: &mut Layer, options: Option<&ApplyOptions>) {
  // Use a mutable reference to the underlying Layer so the image copy-on-write
  // path is used. This ensures changes don't silently write into a shared
  // Image instance and allows us to mark the canvas as needing a recompose.
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    color::auto_color().with_options(options).apply(img);
  });
  layer.mark_dirty();
}

#[napi]
/// Automatically adjusts the tone of a layer.
/// @param layer The layer to adjust.
/// @param options Optional adjustment options, including area and mask.
pub fn auto_tone(layer: &mut Layer, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    color::auto_tone().with_options(options).apply(img);
  });
  layer.mark_dirty();
}

#[napi]
/// Converts the colors of a layer to grayscale.
/// @param layer The layer to convert to grayscale.
/// @param options Optional adjustment options, including area and mask.
pub fn grayscale(layer: &mut Layer, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    color::grayscale().with_options(options).apply(img);
  });
  layer.mark_dirty();
}

#[napi]
/// Inverts the colors of a layer.
/// @param layer The layer to invert.
/// @param options Optional adjustment options, including area and mask.
pub fn invert(layer: &mut Layer, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    color::invert().with_options(options).apply(img);
  });
  layer.mark_dirty();
}

#[napi]
/// Adjusts the brightness of a layer.
/// @param layer The layer to adjust.
/// @param value The brightness adjustment value (-100 to 100).
/// @param options Optional adjustment options, including area and mask.
pub fn brightness(layer: &mut Layer, value: f64, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    levels::brightness(value as i32).with_options(options).apply(img);
  });
  layer.mark_dirty();
}

#[napi]
/// Adjusts the contrast of a layer.
/// @param layer The layer to adjust.
/// @param value The contrast adjustment value (-100 to 100).
/// @param options Optional adjustment options, including area and mask.
pub fn contrast(layer: &mut Layer, value: f64, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    levels::contrast(value as i32).with_options(options).apply(img);
  });
  layer.mark_dirty();
}

#[napi]
/// Adjusts the exposure of a layer.
/// @param layer The layer to adjust.
/// @param exposure The exposure adjustment value.
/// @param offset The offset adjustment value.
/// @param gamma The gamma adjustment value.
/// @param options Optional adjustment options, including area and mask.
pub fn exposure(layer: &mut Layer, exposure: f64, offset: f64, gamma: f64, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    levels::exposure(exposure).with_offset(offset).with_gamma(gamma).with_options(options).apply(img);
  });
  layer.mark_dirty();
}

#[napi]
/// Adjusts the vibrance of a layer.
/// @param layer The layer to adjust.
/// @param vibrance The vibrance adjustment value.
/// @param saturation The saturation adjustment value.
/// @param options Optional adjustment options, including area and mask.
pub fn vibrance(layer: &mut Layer, vibrance: f64, saturation: f64, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    levels::vibrance(vibrance).with_saturation(saturation).with_options(options).apply(img);
  });
  layer.mark_dirty();
}
