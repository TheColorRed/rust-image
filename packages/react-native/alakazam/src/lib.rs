//! React Native bindings for the Abra image library.
//!
//! Exposed to JavaScript through uniffi-bindgen-react-native (`jsi2` flavour):
//! this crate builds `libalakazam_mobile.so`, which the `@ubjs/react-native`
//! player loads at runtime. Keep this crate a thin wrapper — reusable logic
//! belongs in abra.

uniffi::setup_scaffolding!();

pub mod adjustments;
pub mod components;
pub mod effect_spec;
pub mod filters;
pub mod history;
pub mod image;
pub mod live_effects;
pub mod live_image;
pub mod tools;
pub mod transforms;

pub use history::AbraImageHistory;
pub use image::AbraImage;

/// Errors surfaced to JavaScript as thrown exceptions.
#[derive(Debug, uniffi::Error)]
pub enum AbraError {
  /// Reading or writing an image file failed.
  Io { message: String },
  /// The supplied pixel buffer does not match the requested dimensions.
  InvalidPixels { message: String },
  /// Rendering a live preview failed.
  Render { message: String },
  /// Running person detection or skin segmentation failed.
  Ai { message: String },
  /// The requested person index is not present in the image.
  InvalidPersonSelection { index: u32 },
}

impl std::fmt::Display for AbraError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AbraError::Io { message } => write!(f, "IO error: {message}"),
      AbraError::InvalidPixels { message } => write!(f, "Invalid pixels: {message}"),
      AbraError::Render { message } => write!(f, "Render error: {message}"),
      AbraError::Ai { message } => write!(f, "AI error: {message}"),
      AbraError::InvalidPersonSelection { index } => write!(f, "No detected person at index {index}"),
    }
  }
}

impl std::error::Error for AbraError {}
