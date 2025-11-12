#![warn(missing_docs)]
#![allow(deprecated)]

//! This crate provides various image processing functionalities including adjustments, drawing, filtering, and more.

/// Adjustments module
pub mod adjustments;
/// Canvas module
pub mod canvas;
/// Color module
pub mod color;
/// Combine module
pub mod combine;
/// Draw module
pub mod draw;
/// Filters module
pub mod filters;
/// Geometry module
pub mod geometry;
/// Image module
pub mod image;
/// Mask module
pub mod mask;
/// Path module
pub mod path;
/// Plugin module
pub mod plugin;
/// Transform module
pub mod transform;
/// Utilities module
pub mod utils;

// pub use canvas::{Anchor, Canvas, Layer};
pub use canvas::{Anchor, Canvas, Layer, LayerSize, NewLayerOptions};
pub use image::Image;
pub use utils::loader::{ImageLoader, LoadedImages};

#[derive(Clone, Copy, Eq, PartialEq)]
/// The number of channels in an image
pub enum Channels {
  /// A three channel image (RGB)
  RGB = 3,
  /// A four channel image (RGBA)
  RGBA = 4,
}

// mod gpu;
