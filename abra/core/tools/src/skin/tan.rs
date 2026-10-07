use crate::{Tool, skin::scale_mask};
use abra_core::{Color, Image, ImageRef, IntoNumber};
use adjustments::color::color_multiply;
use humanoid::{skin_mask, soften_skin_mask};
use mask::{IntoMaskCow, Mask};
use options::{ApplyOptions, Effect, EffectSink};
use std::borrow::Cow;

/// Tans skin by composing the detected skin mask with the generic color multiplication adjustment.
#[derive(Clone)]
pub struct SkinTan<'mask> {
  color: Color,
  feather: f32,
  mask: Option<Cow<'mask, Mask>>,
}

impl SkinTan<'_> {
  /// Replaces color-based skin detection with a supplied grayscale mask.
  pub fn with_mask<'mask>(self, p_mask: impl IntoMaskCow<'mask>) -> SkinTan<'mask> {
    SkinTan {
      color: self.color,
      feather: self.feather,
      mask: Some(p_mask.into_mask_cow()),
    }
  }

  /// Sets the edge fade as a multiple of the standard skin fade. Defaults to `3.0`.
  pub fn with_feather(mut self, p_feather: impl IntoNumber) -> Self {
    self.feather = p_feather.into::<f32>().max(0.0);
    self
  }

  fn mask_for(&self, p_source: Option<Image>) -> Option<Mask> {
    let source = p_source?;
    let mask = match &self.mask {
      Some(mask) => soften_skin_mask(&source, mask, self.feather),
      None => skin_mask(&source, self.feather),
    };
    Some(scale_mask(&mask, self.color.rgba().3 as f32 / 255.0))
  }

  fn effect(&self, p_mask: Option<Mask>) -> adjustments::color::ColorMultiply {
    let (red, green, blue, _) = self.color.rgba();
    let effect = color_multiply(Color::from_rgb(red, green, blue));
    match p_mask {
      Some(mask) => effect.with_options(ApplyOptions::new().with_mask(mask)),
      None => effect,
    }
  }

  /// Applies this tool to an image or live preview.
  pub fn apply_to(&self, p_target: impl EffectSink) {
    let mask = self.mask_for(p_target.source_image());
    p_target.accept(self.effect(mask));
  }
}

impl Tool for SkinTan<'_> {
  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    self.apply_to(p_image.into());
  }
}

/// Tans skin with `p_color`. White leaves the skin unchanged; the color alpha controls how much of the tan shows.
pub fn skin_tan(p_color: Color) -> SkinTan<'static> {
  SkinTan {
    color: p_color,
    feather: 0.0,
    mask: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tan_uses_color_multiplication_through_a_supplied_mask() {
    let mut image = Image::new_from_color(16, 16, Color::from_rgba(200, 150, 100, 71));
    let mask = Mask::from_image(Image::new_from_color(16, 16, Color::white()));
    skin_tan(Color::from_rgb(200, 140, 90)).with_mask(mask).apply_to(&mut image);
    assert_eq!(image.get_pixel(8, 8), Some((157, 82, 35, 71)));
  }
}
