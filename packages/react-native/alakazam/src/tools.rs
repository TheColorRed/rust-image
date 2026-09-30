use abra::abra_core::Units;
use abra::tools::prelude::*;

use crate::AbraImage;

#[uniffi::export]
impl AbraImage {
  /// Removes a blemish centered at `(x, y)` (image pixels), blending in texture borrowed from
  /// nearby. `radius` is the size of the healed spot itself; the tool samples a wider ring around
  /// it for context, scaled to the spot so larger blemishes still get an appropriately sized
  /// sample.
  pub fn remove_blemish(&self, x: f64, y: f64, radius: f64) {
    let distance = Units::Pixels((radius * 0.6).round().max(3.0) as u32);
    self.with_image_mut(|img| {
      remover(RemoverShape::circle(radius)).with_distance(distance).with_position((x, y)).apply(img);
    });
  }
}
