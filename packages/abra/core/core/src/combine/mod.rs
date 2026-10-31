//! Combine multiple images into one.

/// Blends two images using a blend mode
pub mod blend;

pub use blend::{BLEND_MODES_WGSL, BlendMode};
