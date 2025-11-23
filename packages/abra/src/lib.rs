//! The Abra image processing library.
//! It provides core functionalities for image manipulation
//! and a plugin system for extending its capabilities.

pub mod ffi;
/// Plugin module
pub mod plugin;

pub use adjustments;
pub use canvas;
pub use core::*;
pub use drawing;
pub use filters;
pub use mask;
pub use options;
