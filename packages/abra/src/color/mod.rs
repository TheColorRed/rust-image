//! Color management utilities.

/// Color representation and manipulation
pub mod color;
/// Fill modes for colors and gradients
pub(crate) mod fill;
/// Tools for creating gradients
pub(crate) mod gradient;
/// Convert to an HSL color
pub(crate) mod to_hsl;
/// Convert to an HSV color
pub(crate) mod to_hsv;
/// Convert to an RGB color
pub(crate) mod to_rgb;

pub use color::Color;
pub use fill::Fill;
pub use gradient::Gradient;
