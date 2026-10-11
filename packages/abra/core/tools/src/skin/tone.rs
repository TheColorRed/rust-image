use crate::Tool;
use abra_core::{ImageRef, IntoNumber};
use adjustments::levels::exposure;
use humanoid::{skin_mask, soften_skin_mask};
use mask::{IntoMaskCow, Mask};
use options::{ApplyOptions, Effect, EffectSink};
use std::borrow::Cow;

/// Lightens or darkens existing skin tones by composing a skin mask with the exposure adjustment.
#[derive(Clone)]
pub struct SkinTone<'mask> {
  amount: f64,
  feather: f32,
  mask: Option<Cow<'mask, Mask>>,
}

impl SkinTone<'_> {
  /// Replaces color-based skin detection with a supplied grayscale mask.
  pub fn with_mask<'mask>(self, p_mask: impl IntoMaskCow<'mask>) -> SkinTone<'mask> {
    SkinTone {
      amount: self.amount,
      feather: self.feather,
      mask: Some(p_mask.into_mask_cow()),
    }
  }

  /// Sets the edge fade as a multiple of the standard skin fade. Defaults to `3.0`.
  pub fn with_feather(mut self, p_feather: impl IntoNumber) -> Self {
    self.feather = p_feather.into::<f32>().max(0.0);
    self
  }

  fn effect(&self, p_mask: Option<Mask>) -> adjustments::levels::Exposure {
    let effect = exposure(0.0).with_gamma(1.4_f64.powf(-self.amount / 100.0));
    match p_mask {
      Some(mask) => effect.with_options(ApplyOptions::new().with_mask(mask)),
      None => effect,
    }
  }

  fn mask_for(&self, p_source: Option<abra_core::Image>) -> Option<Mask> {
    let source = p_source?;
    Some(match &self.mask {
      Some(mask) => soften_skin_mask(&source, mask, self.feather),
      None => skin_mask(&source, self.feather),
    })
  }

  /// Applies this tool to an image or live preview. A target without a source image receives the underlying exposure
  /// adjustment without a mask; this is useful for capability probes.
  pub fn apply_to(&self, p_target: impl EffectSink) {
    let mask = self.mask_for(p_target.source_image());
    p_target.accept(self.effect(mask));
  }
}

impl Tool for SkinTone<'_> {
  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    self.apply_to(p_image.into());
  }
}

/// Adjusts skin relative to its original tone. Negative amounts lighten and positive amounts darken; the core does not
/// limit the amount.
pub fn skin_tone(p_amount: impl IntoNumber) -> SkinTone<'static> {
  SkinTone {
    amount: p_amount.into::<f64>(),
    feather: 0.0,
    mask: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Color, Image};

  #[test]
  fn tone_uses_exposure_through_a_supplied_mask() {
    let mut image = Image::new_from_color(16, 16, Color::from_rgba(160, 120, 80, 71));
    let mask = Mask::from_image(Image::new_from_color(16, 16, Color::white()));
    skin_tone(-100).with_mask(mask).apply_to(&mut image);
    let pixel = image.get_pixel(8, 8).unwrap();
    assert!(pixel.0 > 160 && pixel.1 > 120 && pixel.2 > 80);
    assert_eq!(pixel.3, 71);
  }
}
