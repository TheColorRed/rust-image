//! Abra prelude — a small convenience module that re-exports commonly used types and traits.
// Re-export the Abra wrapper types

// Re-export selected core types & traits under `abra::prelude` for ergonomic use by consumers
pub use crate::abra_core::Image;
pub use crate::abra_core::ImageLoader;
pub use crate::abra_core::LoadMode;
pub use crate::abra_core::LoadedCollection;
pub use crate::abra_core::LoadedImages;
pub use crate::abra_core::Settings;
pub use crate::abra_core::WriterOptions;
pub use crate::abra_core::image::image_ext::*;
pub use crate::abra_core::{BlendMode, Color, ColorStat, Harmony};
pub use crate::abra_core::{Channel, Channels};

// Commonly used transform traits (brought into prelude for ergonomics)
pub use crate::abra_core::Transform;

// Common geometry and path helpers
pub use crate::abra_core::Area;
pub use crate::abra_core::AspectRatio;
pub use crate::abra_core::Fill;
pub use crate::abra_core::LineJoin;
pub use crate::abra_core::LineSegment;
pub use crate::abra_core::Orientation;
pub use crate::abra_core::Path;
pub use crate::abra_core::PointF;
pub use crate::abra_core::Rect;
pub use crate::abra_core::Shape;
pub use crate::abra_core::Size;

// Gradient and drawing helpers
pub use crate::abra_core::{ColorStop, Gradient};
// pub use crate::drawing::fill;

// Transform options
pub use crate::abra_core::TransformAlgorithm;
pub use crate::abra_core::TransformFit;

// Plugins
pub use crate::plugin::*;
