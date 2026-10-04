use abra::filters::prelude::*;

use crate::AbraImage;

#[uniffi::export]
impl AbraImage {
  /// Applies a box blur with the given radius in pixels.
  pub fn box_blur(&self, radius: f64) {
    self.with_image_mut(|img| blur::box_blur(radius).apply(img));
  }

  /// Sharpens the image.
  pub fn sharpen(&self) {
    self.with_image_mut(|img| sharpen::sharpen().apply(img));
  }

  pub fn smooth(&self) {
    self.with_image_mut(|img| smooth::smooth().apply(img));
  }

  pub fn median(&self, radius: f64) {
    self.with_image_mut(|img| noise::median(radius as f32).apply(img));
  }

  pub fn despeckle(&self) {
    self.with_image_mut(|img| noise::despeckle(1.0, 13.0).apply(img));
  }
}
