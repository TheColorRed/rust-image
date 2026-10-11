use abra_core::Image;
use std::sync::{Arc, Mutex};

use crate::{
  effects::{DropShadow, Stroke, stroke::apply_stroke},
  layer_inner::LayerInner,
};

/// Options for various layer effects.
#[derive(Clone)]
pub struct LayerEffects<'a> {
  pub drop_shadow: Option<DropShadow<'a>>,
  pub stroke: Option<Stroke<'a>>,
  pub layer_inner: Option<Arc<Mutex<LayerInner<'a>>>>,
}

/// Result of applying effects which may include an offset and changed canvas size.
pub struct EffectResult {
  pub image: Arc<Image>,
  pub offset: (i32, i32),
  pub content_dimensions: (u32, u32),
}

impl EffectResult {
  pub fn into_tuple(self) -> (Arc<Image>, (i32, i32), (u32, u32)) {
    (self.image, self.offset, self.content_dimensions)
  }
}

impl<'a> LayerEffects<'a> {
  pub fn new() -> Self {
    LayerEffects {
      drop_shadow: None,
      stroke: None,
      layer_inner: None,
    }
  }

  pub(crate) fn with_layer(mut self, p_layer: Arc<Mutex<LayerInner<'a>>>) -> Self {
    self.layer_inner = Some(p_layer);
    self
  }

  pub(crate) fn apply_with_offset(&self, p_image: Arc<Image>) -> EffectResult {
    let original_dimensions = p_image.dimensions::<u32>();
    let mut result_image = p_image.clone();
    let mut offset = (0i32, 0i32);

    if let Some(stroke_opts) = &self.stroke {
      result_image = apply_stroke(result_image, stroke_opts);
    }

    if let Some(drop_shadow_opts) = &self.drop_shadow {
      let (img, pad) = crate::effects::drop_shadow::apply_drop_shadow_with_offset(result_image, drop_shadow_opts);
      result_image = img;
      offset = (offset.0 + pad.0, offset.1 + pad.1);
    }

    EffectResult {
      image: result_image,
      offset,
      content_dimensions: original_dimensions,
    }
  }

  pub fn with_drop_shadow(mut self, p_options: DropShadow<'a>) -> Self {
    self.drop_shadow = Some(p_options);
    self
  }

  pub fn with_stroke(mut self, p_options: Stroke<'a>) -> Self {
    self.stroke = Some(p_options);
    self
  }
}

impl<'a> Drop for LayerEffects<'a> {
  fn drop(&mut self) {
    if let Some(layer) = self.layer_inner.take() {
      if let Ok(mut inner) = layer.lock() {
        inner.set_effects(self.clone());
      }
    }
  }
}
