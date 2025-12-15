//! Transform operations for layers.
//!
//! Since layers are just wrappers around images, LayerTransform simply delegates all
//! transformation operations to the underlying image. This keeps the logic centralized
//! in the Image type while providing a convenient fluent API for the layer.

use std::sync::Arc;
use std::sync::Mutex;

use abra_core::Rotate;
use abra_core::{Crop, Resize, TransformAlgorithm};

use super::layer_inner::LayerInner;

/// A proxy for applying transform operations to a layer.
/// This type owns the Arc<Mutex<LayerInner>> and can be used to chain transform operations.
///
/// All transformation logic is delegated to the underlying image, keeping the implementation
/// simple and ensuring that all resize/crop logic lives in one place.
pub struct LayerTransform {
  pub(super) layer: Arc<Mutex<LayerInner>>,
}

impl LayerTransform {
  /// Creates a new LayerTransform from an Arc<Mutex<LayerInner>>
  pub(super) fn new(layer: Arc<Mutex<LayerInner>>) -> Self {
    LayerTransform { layer }
  }
}

impl Resize for LayerTransform {
  fn resize(&mut self, p_width: u32, p_height: u32, algorithm: impl Into<Option<TransformAlgorithm>>) {
    self
      .layer
      .lock()
      .unwrap()
      .image_mut()
      .resize(p_width, p_height, algorithm);
    self.layer.lock().unwrap().mark_dirty();
  }

  fn resize_percentage(&mut self, percentage: f32, algorithm: impl Into<Option<TransformAlgorithm>>) {
    self
      .layer
      .lock()
      .unwrap()
      .image_mut()
      .resize_percentage(percentage, algorithm);
    self.layer.lock().unwrap().mark_dirty();
  }

  fn resize_width(&mut self, width: u32, algorithm: impl Into<Option<TransformAlgorithm>>) {
    self.layer.lock().unwrap().image_mut().resize_width(width, algorithm);
    self.layer.lock().unwrap().mark_dirty();
  }

  fn resize_height(&mut self, height: u32, algorithm: impl Into<Option<TransformAlgorithm>>) {
    self.layer.lock().unwrap().image_mut().resize_height(height, algorithm);
    self.layer.lock().unwrap().mark_dirty();
  }

  fn resize_width_relative(&mut self, width: i32, algorithm: impl Into<Option<TransformAlgorithm>>) {
    self
      .layer
      .lock()
      .unwrap()
      .image_mut()
      .resize_width_relative(width, algorithm);
    self.layer.lock().unwrap().mark_dirty();
  }

  fn resize_height_relative(&mut self, height: i32, algorithm: impl Into<Option<TransformAlgorithm>>) {
    self
      .layer
      .lock()
      .unwrap()
      .image_mut()
      .resize_height_relative(height, algorithm);
    self.layer.lock().unwrap().mark_dirty();
  }
}

impl Crop for LayerTransform {
  fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) {
    self.layer.lock().unwrap().image_mut().crop(x, y, width, height);
    self.layer.lock().unwrap().mark_dirty();
  }
}

impl Rotate for LayerTransform {
  fn rotate(&mut self, angle_in_degrees: impl Into<f64>, algorithm: impl Into<Option<TransformAlgorithm>>) {
    self
      .layer
      .lock()
      .unwrap()
      .image_mut()
      .rotate(angle_in_degrees, algorithm);
    self.layer.lock().unwrap().mark_dirty();
  }
  fn flip_horizontal(&mut self) {
    self.layer.lock().unwrap().image_mut().flip_horizontal();
    self.layer.lock().unwrap().mark_dirty();
  }

  fn flip_vertical(&mut self) {
    self.layer.lock().unwrap().image_mut().flip_vertical();
    self.layer.lock().unwrap().mark_dirty();
  }
}
