use super::{Crop, Resize};
use crate::transform::ResizeAlgorithm;

/// A generic handler for chaining transform operations on any type implementing Resize and Crop traits.
/// Provides a fluent API for applying resize and crop transformations.
/// Works with Image, LayerInner, and any other type that implements the transform traits.
pub struct TransformHandler<'a, T: Resize + Crop> {
  target: &'a mut T,
}

impl<'a, T: Resize + Crop> TransformHandler<'a, T> {
  /// Create a new TransformHandler for the given target.
  pub fn new(target: &'a mut T) -> Self {
    TransformHandler { target }
  }
}

impl<'a, T: Resize + Crop> Resize for TransformHandler<'a, T> {
  fn resize(&mut self, p_width: u32, p_height: u32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.target.resize(p_width, p_height, algorithm);
    self
  }

  fn resize_percentage(&mut self, percentage: f32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.target.resize_percentage(percentage, algorithm);
    self
  }

  fn resize_width(&mut self, p_width: u32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.target.resize_width(p_width, algorithm);
    self
  }

  fn resize_height(&mut self, p_height: u32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.target.resize_height(p_height, algorithm);
    self
  }

  fn resize_width_relative(&mut self, p_width: i32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.target.resize_width_relative(p_width, algorithm);
    self
  }

  fn resize_height_relative(&mut self, p_height: i32, algorithm: Option<ResizeAlgorithm>) -> &mut Self {
    self.target.resize_height_relative(p_height, algorithm);
    self
  }
}
