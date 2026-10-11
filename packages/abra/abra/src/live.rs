//! Applying effects described as plain data.
//!
//! An [`EffectSink`] is something an effect can be applied to: an image, or anything else a binding implements it for,
//! such as a live preview. A binding that describes its effects as plain data builds the effect for those parameters and
//! hands it to a sink, so one description works for a one-shot edit and for a live preview.

pub use options::{EffectSink, GpuProbe, SpecEffect};
