//! Color management utilities.
// Re-export color primitives from `primitives` crate and provide gradient/fill in core
mod fill;
mod gradient;
mod histogram;

pub use fill::Fill;
pub use gradient::Gradient;
pub use histogram::Histogram;
pub use primitives::color::*;
