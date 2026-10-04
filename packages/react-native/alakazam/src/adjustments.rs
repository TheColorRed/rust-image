use abra::adjustments::prelude::*;

use crate::AbraImage;

#[uniffi::export]
impl AbraImage {
  /// Automatically adjusts the colors of the image.
  pub fn auto_color(&self) {
    self.with_image_mut(|img| color::auto_color().apply(img));
  }

  /// Automatically adjusts the tone of the image.
  pub fn auto_tone(&self) {
    self.with_image_mut(|img| color::auto_tone().apply(img));
  }

  /// Inverts the colors of the image.
  pub fn invert(&self) {
    self.with_image_mut(|img| color::invert().apply(img));
  }

  pub fn posterize(&self, levels: u8) {
    self.with_image_mut(|img| color::posterize(levels).apply(img));
  }
}
