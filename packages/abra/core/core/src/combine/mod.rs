//! Combine multiple images into one.

/// Blends two images using a blend mode
pub mod blend;

pub use blend::{BLEND_MODES_WGSL, BlendMode};

/// The steps that stacking layers needs: limiting a picture to a mask, adjustment layers, and a layer's live effects as steps
pub mod layers;
