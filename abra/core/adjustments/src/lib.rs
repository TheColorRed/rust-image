// The scaffolding other languages bind to; the items that carry `uniffi` attributes export through it.
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub mod levels;
pub use levels::FilterType;
pub use options::Effect;

/// Adjustments that affect an image's color.
pub mod color;

mod lut;
