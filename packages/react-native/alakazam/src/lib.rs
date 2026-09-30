//! React Native bindings for the Abra image library.
//!
//! Exposed to JavaScript through uniffi-bindgen-react-native (`jsi2` flavour):
//! this crate builds `libalakazam_mobile.so`, which the `@ubjs/react-native`
//! player loads at runtime. Keep this crate a thin wrapper — reusable logic
//! belongs in abra.

uniffi::setup_scaffolding!();

pub mod adjustments;
pub mod filters;
pub mod history;
pub mod image;
pub mod live;
pub mod surface;
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
}

impl std::fmt::Display for AbraError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AbraError::Io { message } => write!(f, "IO error: {message}"),
      AbraError::InvalidPixels { message } => write!(f, "Invalid pixels: {message}"),
      AbraError::Render { message } => write!(f, "Render error: {message}"),
    }
  }
}

impl std::error::Error for AbraError {}
