//! Utilities for applying layer size options to layers.

use abra_core::{Resize, ResizeTarget, Size};

use super::layer_inner::LayerInner;
use super::options_new_layer::LayerSize;

/// Applies a LayerSize option to a layer, resizing it according to the specified strategy.
///
/// # Arguments
/// * `p_layer` - The layer to resize
/// * `p_size` - The LayerSize strategy to apply
/// * `p_canvas_width` - The width of the parent canvas (used for Contain/Cover)
/// * `p_canvas_height` - The height of the parent canvas (used for Contain/Cover)
pub(crate) fn apply_layer_size(p_layer: &mut LayerInner, p_size: LayerSize, p_canvas_width: u32, p_canvas_height: u32) {
  match p_size {
    LayerSize::Maintain => {
      // Do nothing - keep original size
    }
    LayerSize::Contain(algorithm) => {
      let (layer_width, layer_height) = p_layer.dimensions::<u32>();

      let width_ratio = p_canvas_width as f32 / layer_width as f32;
      let height_ratio = p_canvas_height as f32 / layer_height as f32;
      let scale = width_ratio.min(height_ratio);

      let new_width = (layer_width as f32 * scale) as u32;
      let new_height = (layer_height as f32 * scale) as u32;

      p_layer.image_mut().resize(ResizeTarget::Exact(Size::new(new_width, new_height)), algorithm);
    }
    LayerSize::Cover(algorithm) => {
      let (layer_width, layer_height) = p_layer.dimensions::<u32>();

      let width_ratio = p_canvas_width as f32 / layer_width as f32;
      let height_ratio = p_canvas_height as f32 / layer_height as f32;
      let scale = width_ratio.max(height_ratio);

      let new_width = (layer_width as f32 * scale) as u32;
      let new_height = (layer_height as f32 * scale) as u32;

      p_layer.image_mut().resize(ResizeTarget::Exact(Size::new(new_width, new_height)), algorithm);
    }
    LayerSize::Specific(w, h, algorithm) => {
      p_layer.image_mut().resize(ResizeTarget::Exact(Size::new(w, h)), algorithm);
    }
    LayerSize::Percentage(amount, algorithm) => {
      p_layer.image_mut().resize(ResizeTarget::Scale(amount), algorithm);
    }
  }
}
