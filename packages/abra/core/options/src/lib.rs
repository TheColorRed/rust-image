//! Options and traits for applying effects. They live in `abra_core::image::effect`; this crate re-exports them so
//! effects and callers can keep importing from `options`.

pub use abra_core::image::effect::{ApplyOptions, ApplyTarget, Effect, Options, get_ctx};
pub use abra_core::image::gpu::Hardware;

use abra_core::{Image, ImageRef};

/// Any effect that can be handed to an [`EffectSink`].
pub trait SpecEffect: Effect + Clone + 'static {}
impl<E: Effect + Clone + 'static> SpecEffect for E {}

/// Receives an effect for either an immediate edit or a live preview.
pub trait EffectSink {
  /// A copy of the unedited source image, when the target has one. Skin tools use it to build their detected mask.
  fn source_image(&self) -> Option<Image> {
    None
  }

  /// Applies `p_effect` to this target.
  fn accept<E: SpecEffect>(self, p_effect: E);
}

impl EffectSink for &mut Image {
  fn source_image(&self) -> Option<Image> {
    Some((*self).clone())
  }

  fn accept<E: SpecEffect>(self, p_effect: E) {
    p_effect.apply(self);
  }
}

impl EffectSink for ImageRef<'_> {
  fn source_image(&self) -> Option<Image> {
    Some((**self).clone())
  }

  fn accept<E: SpecEffect>(self, p_effect: E) {
    p_effect.apply(self);
  }
}

/// An [`EffectSink`] that records whether an effect has a GPU implementation without applying it.
pub struct GpuProbe<'a>(pub &'a mut bool);

impl EffectSink for GpuProbe<'_> {
  fn accept<E: SpecEffect>(self, p_effect: E) {
    *self.0 = p_effect.gpu_processor().is_some();
  }
}
