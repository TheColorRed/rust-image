use abra::transform::prelude::*;

use crate::AbraImage;

#[uniffi::export]
impl AbraImage {
  /// Rotates while retaining the source dimensions and avoiding transparent corners.
  pub fn rotate(&self, degrees: f64) {
    self.with_image_mut(|img| rotate(degrees).with_fit(TransformFit::Fill).apply(img));
  }

  pub fn flip_horizontal(&self) {
    self.with_image_mut(|img| flip(FlipAxis::Horizontal).apply(img));
  }

  pub fn flip_vertical(&self) {
    self.with_image_mut(|img| flip(FlipAxis::Vertical).apply(img));
  }
}
