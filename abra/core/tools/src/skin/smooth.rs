use crate::{Tool, skin::scale_mask};
use abra_core::{Image, ImageRef, IntoNumber};
use filters::blur::{SurfaceBlur, surface_blur};
use humanoid::{skin_mask, soften_skin_mask};
use mask::{IntoMaskCow, Mask};
use options::{ApplyOptions, Effect, EffectSink};
use std::borrow::Cow;

const RADIUS_FRACTION: f32 = 0.005;
const RADIUS_MIN_PX: u32 = 2;
const SAMPLE_RADIUS_MAX: u32 = 10;
const SURFACE_THRESHOLD: u8 = 30;
const MAX_AMOUNT: f32 = 3.0;
const BOOSTED_THRESHOLD_MAX: u32 = 90;

/// Smooths skin by composing a skin mask with the surface blur filter.
#[derive(Clone)]
pub struct SkinSmooth<'mask> {
  amount: f32,
  feather: f32,
  mask: Option<Cow<'mask, Mask>>,
}

impl SkinSmooth<'_> {
  /// Replaces color-based skin detection with a supplied grayscale mask.
  pub fn with_mask<'mask>(self, p_mask: impl IntoMaskCow<'mask>) -> SkinSmooth<'mask> {
    SkinSmooth {
      amount: self.amount,
      feather: self.feather,
      mask: Some(p_mask.into_mask_cow()),
    }
  }

  /// Sets the edge fade as a multiple of the standard skin fade. Defaults to `10.0` for supplied masks.
  pub fn with_feather(mut self, p_feather: impl IntoNumber) -> Self {
    self.feather = p_feather.into::<f32>().max(0.0);
    self
  }

  fn params(&self, p_source: Option<&Image>) -> (u32, u8) {
    let long_side = p_source.map(|image| image.dimensions::<u32>().0.max(image.dimensions::<u32>().1)).unwrap_or(400);
    let boost = self.amount.max(1.0);
    let radius =
      (((long_side as f32 * RADIUS_FRACTION).round() as u32).max(RADIUS_MIN_PX) as f32 * boost).round() as u32;
    let step = radius.div_ceil(SAMPLE_RADIUS_MAX);
    let threshold = ((SURFACE_THRESHOLD as f32 * boost).round() as u32).min(BOOSTED_THRESHOLD_MAX) as u8;
    (radius.div_ceil(step), threshold)
  }

  fn mask_for(&self, p_source: Option<&Image>) -> Option<Mask> {
    let source = p_source?;
    let mask = match &self.mask {
      Some(mask) => soften_skin_mask(source, mask, self.feather),
      None => skin_mask(source, 1.0),
    };
    Some(scale_mask(&mask, self.amount.min(1.0)))
  }

  fn effect(&self, p_source: Option<&Image>, p_mask: Option<Mask>) -> SurfaceBlur {
    let (radius, threshold) = self.params(p_source);
    let effect = surface_blur(radius, threshold);
    match p_mask {
      Some(mask) => effect.with_options(ApplyOptions::new().with_mask(mask)),
      None => effect,
    }
  }

  /// Applies this tool to an image or live preview.
  pub fn apply_to(&self, p_target: impl EffectSink) {
    if self.amount <= 0.0 {
      return;
    }
    let source = p_target.source_image();
    let mask = self.mask_for(source.as_ref());
    p_target.accept(self.effect(source.as_ref(), mask));
  }
}

impl Tool for SkinSmooth<'_> {
  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    self.apply_to(p_image.into());
  }
}

/// Smooths skin and leaves image features sharp. Values above `1.0` increase the surface blur itself, up to `3.0`.
pub fn skin_smooth(p_amount: impl IntoNumber) -> SkinSmooth<'static> {
  SkinSmooth {
    amount: p_amount.into::<f64>().clamp(0.0, MAX_AMOUNT as f64) as f32,
    feather: 0.0,
    mask: None,
  }
}
