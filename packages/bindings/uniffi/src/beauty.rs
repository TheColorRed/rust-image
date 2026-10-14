//! The beauty tools of the `tools` crate: the skin tools, which tan, tone and smooth the skin.
//!
//! A tool is made by its function and then applied to an image, as in the library:
//! `skinTan(color).withMask(skin).apply(photo)`. Without a mask they find the skin by its color. The `with_*` functions return a
//! new tool, as the library's builders take and return the tool by value, and a mask is copied in, so the tool does not borrow it.

use std::sync::Arc;

use abra::tools::prelude::{self as library, Tool};

use crate::{Color, Image, Mask};

/// Tans skin by multiplying it with a color. Made by [`skin_tan`].
#[derive(uniffi::Object)]
pub struct SkinTan(library::SkinTan<'static>);

/// A tool that tans skin with this color. White leaves the skin unchanged; the color's alpha controls how much of the tan shows.
#[uniffi::export]
pub fn skin_tan(color: Arc<Color>) -> Arc<SkinTan> {
  Arc::new(SkinTan(library::skin_tan(color.0)))
}

#[uniffi::export]
impl SkinTan {
  /// A copy of the tool that works on this mask of the skin, in place of finding the skin by its color.
  pub fn with_mask(&self, mask: Arc<Mask>) -> Arc<SkinTan> {
    Arc::new(SkinTan(self.0.clone().with_mask(mask.core())))
  }

  /// A copy of the tool whose edge fade is this multiple of the standard skin fade.
  pub fn with_feather(&self, feather: f64) -> Arc<SkinTan> {
    Arc::new(SkinTan(self.0.clone().with_feather(feather)))
  }

  /// Tans the skin of the image in place.
  pub fn apply(&self, image: Arc<Image>) {
    image.edit(|image| Tool::apply(&self.0, image));
  }
}

/// A mask of the skin made from a supplied mask: resized to the image and with its edge faded as the skin tools fade it.
/// `feather` is a multiple of the standard skin fade.
#[uniffi::export]
pub fn soften_skin_mask(image: Arc<Image>, mask: Arc<Mask>, feather: f64) -> Arc<Mask> {
  let softened = abra::humanoid::prelude::soften_skin_mask(&image.core(), &mask.core(), feather);
  Arc::new(Mask::from_core(softened))
}

/// A mask of the teeth-colored pixels of the image: bright, not very colorful, and more yellow than skin. A `limit` mask, such as
/// the inside of the mouth, restricts where teeth are looked for, and its edge fade is `feather` times the standard skin fade.
#[uniffi::export]
pub fn teeth_mask(image: Arc<Image>, limit: Option<Arc<Mask>>, feather: f64) -> Arc<Mask> {
  let limit = limit.map(|mask| mask.core());
  let mask = abra::humanoid::prelude::teeth_mask(&image.core(), limit.as_ref(), feather);
  Arc::new(Mask::from_core(mask))
}

/// Lightens or darkens skin relative to its own tone. Made by [`skin_tone`].
#[derive(uniffi::Object)]
pub struct SkinTone(library::SkinTone<'static>);

/// A tool that adjusts skin by this amount: negative lightens and positive darkens, and the library does not limit it.
#[uniffi::export]
pub fn skin_tone(amount: f64) -> Arc<SkinTone> {
  Arc::new(SkinTone(library::skin_tone(amount)))
}

#[uniffi::export]
impl SkinTone {
  /// A copy of the tool that works on this mask of the skin, in place of finding the skin by its color.
  pub fn with_mask(&self, mask: Arc<Mask>) -> Arc<SkinTone> {
    Arc::new(SkinTone(self.0.clone().with_mask(mask.core())))
  }

  /// A copy of the tool whose edge fade is this multiple of the standard skin fade.
  pub fn with_feather(&self, feather: f64) -> Arc<SkinTone> {
    Arc::new(SkinTone(self.0.clone().with_feather(feather)))
  }

  /// Changes the tone of the skin of the image in place.
  pub fn apply(&self, image: Arc<Image>) {
    image.edit(|image| Tool::apply(&self.0, image));
  }
}

/// Smooths skin without blurring its edges. Made by [`skin_smooth`].
#[derive(uniffi::Object)]
pub struct SkinSmooth(library::SkinSmooth<'static>);

/// A tool that smooths skin by this amount, from 0 (nothing) to 3; the library limits it to that.
#[uniffi::export]
pub fn skin_smooth(amount: f64) -> Arc<SkinSmooth> {
  Arc::new(SkinSmooth(library::skin_smooth(amount)))
}

#[uniffi::export]
impl SkinSmooth {
  /// A copy of the tool that works on this mask of the skin, in place of finding the skin by its color.
  pub fn with_mask(&self, mask: Arc<Mask>) -> Arc<SkinSmooth> {
    Arc::new(SkinSmooth(self.0.clone().with_mask(mask.core())))
  }

  /// A copy of the tool whose edge fade is this multiple of the standard skin fade.
  pub fn with_feather(&self, feather: f64) -> Arc<SkinSmooth> {
    Arc::new(SkinSmooth(self.0.clone().with_feather(feather)))
  }

  /// Smooths the skin of the image in place.
  pub fn apply(&self, image: Arc<Image>) {
    image.edit(|image| Tool::apply(&self.0, image));
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra::abra_core::{Channels, Image as CoreImage};
  use abra::mask::prelude::Mask as CoreMask;

  /// A skin-colored picture with a dark patch, and a mask that reaches the left half only.
  fn skin() -> (CoreImage, CoreMask) {
    let (width, height) = (16u32, 16u32);
    let mut pixels = Vec::new();
    for y in 0..height {
      for x in 0..width {
        let shade = 120 + ((x + y) % 7) as u8 * 4;
        pixels.extend_from_slice(&[200, shade, 90, 255]);
      }
    }
    let values = (0..width * height).map(|index| if index % width < width / 2 { 255 } else { 0 }).collect();
    (CoreImage::new_from_pixels(width, height, pixels, Channels::RGBA), CoreMask::from_values(width, height, values))
  }

  #[test]
  fn skin_tan_matches_the_library_and_only_changes_the_masked_skin() {
    let (image, mask) = skin();
    let color = abra::abra_core::Color::from_rgb(210, 140, 90);
    let handle = Arc::new(Image::from_core(image.clone()));
    skin_tan(Arc::new(Color(color))).with_mask(Arc::new(Mask::from_core(mask.clone()))).apply(handle.clone());

    let mut expected = image.clone();
    library::skin_tan(color).with_mask(mask).apply(&mut expected);

    assert_eq!(handle.core().rgba(), expected.rgba());
    assert_ne!(handle.core().rgba(), image.rgba(), "the skin was tanned");
    assert_eq!(handle.core().get_pixel(15, 0), image.get_pixel(15, 0), "outside the mask is untouched");
  }

  #[test]
  fn skin_tone_and_smooth_match_the_library() {
    let (image, mask) = skin();
    let handle = Arc::new(Image::from_core(image.clone()));
    skin_tone(-80.0).with_mask(Arc::new(Mask::from_core(mask.clone()))).apply(handle.clone());
    let mut expected = image.clone();
    library::skin_tone(-80.0).with_mask(mask.clone()).apply(&mut expected);
    assert_eq!(handle.core().rgba(), expected.rgba());
    assert_ne!(handle.core().rgba(), image.rgba(), "the tone changed");

    let handle = Arc::new(Image::from_core(image.clone()));
    skin_smooth(1.0).with_mask(Arc::new(Mask::from_core(mask.clone()))).apply(handle.clone());
    let mut expected = image.clone();
    library::skin_smooth(1.0).with_mask(mask).apply(&mut expected);
    assert_eq!(handle.core().rgba(), expected.rgba());
  }

  #[test]
  fn skin_smooth_with_no_amount_changes_nothing() {
    let (image, mask) = skin();
    let handle = Arc::new(Image::from_core(image.clone()));
    skin_smooth(0.0).with_mask(Arc::new(Mask::from_core(mask))).apply(handle.clone());
    assert_eq!(handle.core().rgba(), image.rgba());
  }
}
