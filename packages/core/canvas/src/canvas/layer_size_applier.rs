//! Utilities for applying layer size options to layers.

use abra_core::Resize;

use super::layer_inner::LayerInner;
use super::options_new_layer::LayerSize;

/// Applies a LayerSize option to a layer, resizing it according to the specified strategy.
///
/// # Arguments
/// * `layer` - The layer to resize
/// * `size` - The LayerSize strategy to apply
/// * `canvas_width` - The width of the parent canvas (used for Contain/Cover)
/// * `canvas_height` - The height of the parent canvas (used for Contain/Cover)
pub(crate) fn apply_layer_size(layer: &mut LayerInner, size: LayerSize, canvas_width: u32, canvas_height: u32) {
  match size {
    LayerSize::Maintain => {
      // Do nothing - keep original size
    }
    LayerSize::Contain(algorithm) => {
      let (layer_width, layer_height) = layer.dimensions::<u32>();

      let width_ratio = canvas_width as f32 / layer_width as f32;
      let height_ratio = canvas_height as f32 / layer_height as f32;
      let scale = width_ratio.min(height_ratio);

      let new_width = (layer_width as f32 * scale) as u32;
      let new_height = (layer_height as f32 * scale) as u32;

      layer.image_mut().resize(new_width, new_height, algorithm);
    }
    LayerSize::Cover(algorithm) => {
      let (layer_width, layer_height) = layer.dimensions::<u32>();

      let width_ratio = canvas_width as f32 / layer_width as f32;
      let height_ratio = canvas_height as f32 / layer_height as f32;
      let scale = width_ratio.max(height_ratio);

      let new_width = (layer_width as f32 * scale) as u32;
      let new_height = (layer_height as f32 * scale) as u32;

      layer.image_mut().resize(new_width, new_height, algorithm);
    }
    LayerSize::Specific(w, h, algorithm) => {
      layer.image_mut().resize(w, h, algorithm);
    }
    LayerSize::Percentage(amount, algorithm) => {
      layer.image_mut().resize_percentage(amount, algorithm);
    }
  }
}
