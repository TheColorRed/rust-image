//! Applying effects described as plain data.
//!
//! An [`EffectSink`] is something an effect can be applied to: an image, or anything else a binding implements it for,
//! such as a live preview. A binding that describes its effects as plain data builds the effect for those parameters and
//! hands it to a sink, so one description works for a one-shot edit and for a live preview.

use abra_core::{Image, ImageRef};
use options::Effect;


/// Any effect that can be handed to an [`EffectSink`]. Every effect qualifies, so this only names the bounds once.
pub trait SpecEffect: Effect + Clone + 'static {}
impl<E: Effect + Clone + 'static> SpecEffect for E {}

/// Something an effect can be applied to: an image or a [`LiveImage`]. It takes whichever effect it is given, so a new
/// effect needs no bound here.
pub trait EffectSink {
  /// Applies `p_effect` to this target, as `p_effect.apply(target)` would.
  fn accept<E: SpecEffect>(self, p_effect: E);
}

impl EffectSink for &mut Image {
  fn accept<E: SpecEffect>(self, p_effect: E) {
    p_effect.apply(self);
  }
}

impl EffectSink for ImageRef<'_> {
  fn accept<E: SpecEffect>(self, p_effect: E) {
    p_effect.apply(self);
  }
}

/// An [`EffectSink`] that only records whether the effect it is given has a shader, in the `bool` it holds. It applies
/// nothing. A binding uses it to ask whether an effect described as data runs on the GPU.
pub struct GpuProbe<'a>(pub &'a mut bool);

impl EffectSink for GpuProbe<'_> {
  fn accept<E: SpecEffect>(self, p_effect: E) {
    *self.0 = p_effect.gpu_processor().is_some();
  }
}
