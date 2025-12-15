use crate::common::*;
use abra::adjustments::prelude::*;

#[napi]
pub fn auto_color(layer: &mut Layer) {
  // Use a mutable reference to the underlying Layer so the image copy-on-write
  // path is used. This ensures changes don't silently write into a shared
  // Image instance and allows us to mark the canvas as needing a recompose.
  let layer_ref = layer.get_underlying_layer_mut();
  color::auto_color(&mut *layer_ref, None);
  layer.mark_dirty();
}

#[napi]
pub fn auto_tone(layer: &mut Layer) {
  let layer_ref = layer.get_underlying_layer_mut();
  color::auto_tone(&mut *layer_ref, None);
  layer.mark_dirty();
}

#[napi]
pub fn grayscale(layer: &mut Layer) {
  let layer_ref = layer.get_underlying_layer_mut();
  color::grayscale(&mut *layer_ref);
  layer.mark_dirty();
}

#[napi]
pub fn invert(layer: &mut Layer) {
  let layer_ref = layer.get_underlying_layer_mut();
  color::invert(&mut *layer_ref);
  layer.mark_dirty();
}

#[napi]
pub fn brightness(layer: &mut Layer, value: f64) {
  let layer_ref = layer.get_underlying_layer_mut();
  levels::brightness(&mut *layer_ref, value as i32, None);
  layer.mark_dirty();
}

#[napi]
pub fn contrast(layer: &mut Layer, value: f64) {
  let layer_ref = layer.get_underlying_layer_mut();
  levels::contrast(&mut *layer_ref, value as i32, None);
  layer.mark_dirty();
}

#[napi]
pub fn exposure(layer: &mut Layer, exposure: f64, offset: f64, gamma: f64) {
  let layer_ref = layer.get_underlying_layer_mut();
  levels::exposure(&mut *layer_ref, exposure as f32, offset as f32, gamma as f32, None);
  layer.mark_dirty();
}

#[napi]
pub fn vibrance(layer: &mut Layer, vibrance: f64, saturation: f64) {
  let layer_ref = layer.get_underlying_layer_mut();
  levels::vibrance(&mut *layer_ref, vibrance as f32, saturation as f32, None);
  layer.mark_dirty();
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  #[test]
  fn binding_auto_color_mutates_layer() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let image_path = std::path::Path::new(&manifest).join("../../../assets/kelsey.jpg");
    let mut project =
      crate::project::Project::new_from_file("test".to_string(), image_path.to_str().unwrap().to_string());
    let mut layer_opt = project
      .get_layer_by_name("Background".to_string())
      .expect("Background layer");
    let before = layer_opt.get_underlying_layer().image().get_pixel(0, 0);
    auto_color(&mut layer_opt);
    let after = layer_opt.get_underlying_layer().image().get_pixel(0, 0);
    assert_ne!(before, after, "auto_color should mutate the pixel data");
  }
}
