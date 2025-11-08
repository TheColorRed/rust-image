//! Transform operations for layers.
//!
//! Since layers are just wrappers around images, LayerTransform simply delegates all
//! transformation operations to the underlying image. This keeps the logic centralized
//! in the Image type while providing a convenient fluent API for the layer.

use std::cell::RefCell;
use std::rc::Rc;

use crate::transform::{Crop, Resize, ResizeAlgorithm};

use super::layer_inner::LayerInner;

/// A proxy for applying transform operations to a layer.
/// This type owns the Rc<RefCell<LayerInner>> and can be used to chain transform operations.
///
/// All transformation logic is delegated to the underlying image, keeping the implementation
/// simple and ensuring that all resize/crop logic lives in one place.
pub struct LayerTransform {
  pub(super) layer: Rc<RefCell<LayerInner>>,
}

impl LayerTransform {
  /// Creates a new LayerTransform from an Rc<RefCell<LayerInner>>
  pub(super) fn new(layer: Rc<RefCell<LayerInner>>) -> Self {
    LayerTransform { layer }
  }
}

impl Resize for LayerTransform {
  fn resize(&mut self, p_width: u32, p_height: u32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.layer.borrow_mut().image_mut().resize(p_width, p_height, algorithm);
    self
  }

  fn resize_percentage(&mut self, percentage: f32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.layer.borrow_mut().image_mut().resize_percentage(percentage, algorithm);
    self
  }

  fn resize_width(&mut self, width: u32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.layer.borrow_mut().image_mut().resize_width(width, algorithm);
    self
  }

  fn resize_height(&mut self, height: u32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.layer.borrow_mut().image_mut().resize_height(height, algorithm);
    self
  }

  fn resize_width_relative(&mut self, width: i32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.layer.borrow_mut().image_mut().resize_width_relative(width, algorithm);
    self
  }

  fn resize_height_relative(&mut self, height: i32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.layer.borrow_mut().image_mut().resize_height_relative(height, algorithm);
    self
  }
}

impl Crop for LayerTransform {
  fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> &mut Self {
    self.layer.borrow_mut().image_mut().crop(x, y, width, height);
    self
  }
}
