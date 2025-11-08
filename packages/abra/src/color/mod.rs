//! Color management utilities.

mod color;
/// Tools for creating gradients
pub mod gradient;
/// Convert to an HSL color
pub mod to_hsl;
/// Convert to an HSV color
pub mod to_hsv;
/// Convert to an RGB color
pub mod to_rgb;
/// Manage a single color
pub use color::Color;
