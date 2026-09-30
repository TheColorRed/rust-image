//! Transform operations for layers.
//!
//! Since layers are just wrappers around images, LayerTransform simply delegates all
//! transformation operations to the underlying image. This keeps the logic centralized
//! in the Image type while providing a convenient fluent API for the layer.

use std::sync::Arc;
use std::sync::Mutex;

use abra_core::{FlipAxis, IntoNumber, ResizeTarget, Transform, TransformAlgorithm};

use super::layer_inner::{LayerInner, LayerOperation};

/// A proxy for applying transform operations to a layer.
/// This type owns the Arc<Mutex<LayerInner>> and can be used to chain transform operations.
///
/// All transformation logic is delegated to the underlying image, keeping the implementation
/// simple and ensuring that all resize/crop logic lives in one place.
pub struct LayerTransform<'a> {
  pub(super) layer: Arc<Mutex<LayerInner<'a>>>,
}

impl<'a> LayerTransform<'a> {
  /// Creates a new LayerTransform from an Arc<Mutex<LayerInner>>
  pub(super) fn new(p_layer: Arc<Mutex<LayerInner<'a>>>) -> Self {
    LayerTransform { layer: p_layer }
  }
}

impl<'a> Transform for LayerTransform<'a> {
  type Target = ResizeTarget;

  fn resize(&mut self, p_target: ResizeTarget, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
    self.layer.lock().unwrap().queue_operation(LayerOperation::Resize(p_target, p_algorithm.into()));
    self.layer.lock().unwrap().mark_dirty();
  }

  fn crop(&mut self, p_x: impl IntoNumber, p_y: impl IntoNumber, p_width: impl IntoNumber, p_height: impl IntoNumber) {
    let (x, y, width, height) = (p_x.into::<u32>(), p_y.into::<u32>(), p_width.into::<u32>(), p_height.into::<u32>());
    self.layer.lock().unwrap().queue_operation(LayerOperation::Crop(x, y, width, height));
    self.layer.lock().unwrap().mark_dirty();
  }

  fn rotate(&mut self, p_angle_in_degrees: impl IntoNumber, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
    self.layer.lock().unwrap().queue_operation(LayerOperation::Rotate(p_angle_in_degrees.into(), p_algorithm.into()));
    self.layer.lock().unwrap().mark_dirty();
  }
  fn flip(&mut self, p_axis: FlipAxis) {
    self.layer.lock().unwrap().queue_operation(LayerOperation::Flip(p_axis));
    self.layer.lock().unwrap().mark_dirty();
  }
}
