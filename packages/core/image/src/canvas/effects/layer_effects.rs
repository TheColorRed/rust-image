use core::Image;
use std::sync::{Arc, Mutex};

use crate::{
  canvas::layer_inner::LayerInner,
  effects::{DropShadow, Stroke, drop_shadow::apply_drop_shadow, stroke::apply_stroke},
};

#[derive(Clone)]
/// Options for various layer effects.
pub struct LayerEffects {
  /// Options for drop shadow effect.
  pub drop_shadow: Option<DropShadow>,
  /// Options for stroke effect.
  pub stroke: Option<Stroke>,
  /// Reference to apply effects to (used when chaining from layer.effects())
  pub(crate) layer_inner: Option<Arc<Mutex<LayerInner>>>,
}

impl LayerEffects {
  /// Creates a new LayerEffects with default settings.
  pub fn new() -> Self {
    LayerEffects {
      drop_shadow: None,
      stroke: None,
      layer_inner: None,
    }
  }

  /// Internal: sets the layer reference for auto-application.
  /// - `layer`: The layer to associate with these effects.
  pub(crate) fn with_layer(mut self, layer: Arc<Mutex<LayerInner>>) -> Self {
    self.layer_inner = Some(layer);
    self
  }

  /// Applies the configured effects to the given image and returns the result.
  /// - `image`: The image to apply effects to.
  pub(crate) fn apply(&self, image: Arc<Image>) -> Arc<Image> {
    let mut result_image = image;

    // Apply stroke first so it outlines the original layer bounds,
    // then apply drop shadow to the stroked image.
    if let Some(stroke_opts) = &self.stroke {
      result_image = apply_stroke(result_image, stroke_opts);
    }

    if let Some(drop_shadow_opts) = &self.drop_shadow {
      result_image = apply_drop_shadow(result_image, drop_shadow_opts);
    }

    result_image
  }

  /// Adds a drop shadow effect with the given options.
  /// The effect will be applied during the final rendering of the layer.
  /// - `options`: The drop shadow options to apply.
  pub fn with_drop_shadow(mut self, options: DropShadow) -> Self {
    self.drop_shadow = Some(options);
    self
  }

  /// Adds a stroke effect with the given options.
  /// The effect will be applied during the final rendering of the layer.
  /// - `options`: The stroke options to apply.
  pub fn with_stroke(mut self, options: Stroke) -> Self {
    self.stroke = Some(options);
    self
  }
}

impl Drop for LayerEffects {
  fn drop(&mut self) {
    if let Some(layer) = &self.layer_inner {
      if let Ok(mut inner) = layer.lock() {
        // Commit a cloned snapshot of the current effects configuration.
        inner.set_effects(self.clone());
      }
    }
  }
}
