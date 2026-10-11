//! Color management utilities: fills and gradients, plus the color types re-exported from `primitives`.
mod fill;
mod gradient;

pub use fill::Fill;
pub use gradient::{ColorStop, Gradient};
pub use primitives::color::*;
