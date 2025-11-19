//! The Abra image processing library.
//! It provides core functionalities for image manipulation
//! and a plugin system for extending its capabilities.

/// Plugin module
pub mod plugin;

pub use adjustments;
pub use core::*;
pub use drawing;
pub use filters;
pub use image;
