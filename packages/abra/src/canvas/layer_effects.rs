use core::cell::RefCell;
use std::rc::Rc;

use crate::canvas::{
  DropShadowOptions, StrokeOptions,
  effects::{Shadow, Stroke},
  layer::Layer,
  layer_inner::LayerInner,
};

/// A proxy for applying effects to a layer.
/// This type owns the Rc<RefCell<LayerInner>> and can be used to chain transform operations.
///
/// All transformation logic is delegated to the underlying image, keeping the implementation
/// simple and ensuring that all resize/crop logic lives in one place.
pub struct LayerEffects {
  pub(super) layer: Rc<RefCell<LayerInner>>,
}

impl LayerEffects {
  /// Creates a new LayerEffects from an Rc<RefCell<LayerInner>>
  pub(super) fn new(layer: Rc<RefCell<LayerInner>>) -> Self {
    LayerEffects { layer }
  }
}

impl Shadow for LayerEffects {
  fn drop_shadow(&self, options: DropShadowOptions) {
    let layer = Layer::from_inner(self.layer.clone());
    super::drop_shadow::drop_shadow(layer, options);
  }
}

impl Stroke for LayerEffects {
  /// Applies a stroke effect to the layer.
  fn stroke(&self, options: StrokeOptions) {
    let layer = Layer::from_inner(self.layer.clone());
    super::stroke::stroke(layer, options);
  }
}
