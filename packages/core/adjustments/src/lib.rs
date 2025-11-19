mod brightness;
mod contrast;
mod hue;
mod saturation;

/// Adjustments that affect an image's color.
pub mod color;
pub use brightness::brightness;
pub use contrast::contrast;
pub use hue::hue;
pub use saturation::saturation;
