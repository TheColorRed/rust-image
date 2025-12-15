//! Utilities for applying layer options when creating a new layer.

use super::anchor::Anchor;
use super::layer_inner::LayerInner;
use super::layer_size_applier;
use super::options_new_layer::NewLayerOptions;

/// Applies layer options (anchor, size, opacity, blend mode) to a newly created layer.
///
/// This handles the common pattern of applying NewLayerOptions to a layer with proper defaults.
///
/// # Arguments
/// * `layer` - The layer to apply options to
/// * `options` - The options to apply (if None, defaults are used)
/// * `canvas_width` - The width of the parent canvas
/// * `canvas_height` - The height of the parent canvas
pub(crate) fn apply_layer_options(
  layer: &mut LayerInner, options: Option<&NewLayerOptions>, canvas_width: u32, canvas_height: u32,
) {
  match options {
    Some(opts) => {
      // Apply anchor
      if let Some(anchor) = opts.anchor {
        layer.anchor_to_canvas(anchor);
      } else {
        layer.anchor_to_canvas(Anchor::Center);
      }

      // Apply size
      if let Some(size) = opts.size {
        layer_size_applier::apply_layer_size(layer, size, canvas_width, canvas_height);
      }

      // Apply opacity
      if let Some(opacity) = opts.opacity {
        layer.set_opacity(opacity);
      }

      // Apply blend mode
      if let Some(blend_mode) = opts.blend_mode {
        layer.set_blend_mode(blend_mode);
      }
    }
    None => {
      // Apply defaults when no options provided
      layer.anchor_to_canvas(Anchor::Center);
    }
  }
}
