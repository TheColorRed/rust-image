//! Color management utilities.

/// Color representation and manipulation
mod color;
mod colors_list;
/// Fill modes for colors and gradients
mod fill;
/// Tools for creating gradients
mod gradient;
/// Convert to an HSL color
mod to_hsl;
/// Convert to an HSV color
mod to_hsv;
/// Convert to an RGB color
mod to_rgb;

pub use color::Color;
pub use fill::Fill;
pub use gradient::Gradient;
pub use to_hsl::rgb_to_hsl;
pub use to_hsv::rgb_to_hsv;
pub use to_rgb::{hsl_to_rgb, hsv_to_rgb};
