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
/// * `p_layer` - The layer to apply options to
/// * `p_options` - The options to apply (if None, defaults are used)
/// * `p_canvas_width` - The width of the parent canvas
/// * `p_canvas_height` - The height of the parent canvas
pub(crate) fn apply_layer_options(
  p_layer: &mut LayerInner, p_options: Option<&NewLayerOptions>, p_canvas_width: u32, p_canvas_height: u32,
) {
  match p_options {
    Some(opts) => {
      // Apply anchor
      if let Some(anchor) = opts.anchor {
        p_layer.anchor_to_canvas(anchor);
      } else {
        p_layer.anchor_to_canvas(Anchor::Center);
      }

      // Apply size
      if let Some(size) = opts.size {
        layer_size_applier::apply_layer_size(p_layer, size, p_canvas_width, p_canvas_height);
      }

      // Apply opacity
      if let Some(opacity) = opts.opacity {
        p_layer.set_opacity(opacity);
      }

      // Apply blend mode
      if let Some(blend_mode) = opts.blend_mode {
        p_layer.set_blend_mode(blend_mode);
      }
    }
    None => {
      // Apply defaults when no options provided
      p_layer.anchor_to_canvas(Anchor::Center);
    }
  }
}
