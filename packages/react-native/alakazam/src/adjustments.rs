use abra::adjustments::prelude::*;

use crate::AbraImage;

/// Preset color treatments for the mobile editor.
#[derive(uniffi::Enum)]
pub enum PhotoFilterPreset {
  Warming,
  Cooling,
  Sepia,
  Underwater,
  DeepEmerald,
  CoolingDark,
  Magenta,
}

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

  /// Converts the image to grayscale.
  pub fn grayscale(&self) {
    self.with_image_mut(|img| color::grayscale().apply(img));
  }

  /// Inverts the colors of the image.
  pub fn invert(&self) {
    self.with_image_mut(|img| color::invert().apply(img));
  }

  /// Adjusts exposure in stops.
  pub fn exposure(&self, value: f64) {
    self.with_image_mut(|img| levels::exposure(value).apply(img));
  }

  /// Adjusts vibrance (-100 to 100).
  pub fn vibrance(&self, value: f64) {
    self.with_image_mut(|img| levels::vibrance(value).apply(img));
  }

  pub fn threshold(&self, value: u8) {
    self.with_image_mut(|img| color::threshold(value).apply(img));
  }

  pub fn posterize(&self, levels: u8) {
    self.with_image_mut(|img| color::posterize(levels).apply(img));
  }

  pub fn photo_filter(&self, preset: PhotoFilterPreset, density: f64) {
    let preset = match preset {
      PhotoFilterPreset::Warming => FilterType::WarmingLight,
      PhotoFilterPreset::Cooling => FilterType::CoolingLight,
      PhotoFilterPreset::Sepia => FilterType::Sepia,
      PhotoFilterPreset::Underwater => FilterType::Underwater,
      PhotoFilterPreset::DeepEmerald => FilterType::DeepEmerald,
      PhotoFilterPreset::CoolingDark => FilterType::CoolingDark,
      PhotoFilterPreset::Magenta => FilterType::Magenta,
    };
    self
      .with_image_mut(|img| levels::photo_filter(levels::PhotoFilter::Preset(preset)).with_density(density).apply(img));
  }
}
