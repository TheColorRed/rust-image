//! Abra prelude — a small convenience module that re-exports commonly used types and traits.
// Re-export the Abra wrapper types

// Re-export selected core types & traits under `abra::prelude` for ergonomic use by consumers
pub use crate::abra_core::Channels;
pub use crate::abra_core::Color;
pub use crate::abra_core::Image;
pub use crate::abra_core::ImageLoader;
pub use crate::abra_core::LoadedImages;
pub use crate::abra_core::Settings;
pub use crate::abra_core::WriterOptions;
pub use crate::abra_core::image::image_ext::*;

// Commonly used transform traits (brought into prelude for ergonomics)
pub use crate::abra_core::Crop;
pub use crate::abra_core::Resize;
pub use crate::abra_core::Rotate;

// Common geometry and path helpers
pub use crate::abra_core::Area;
pub use crate::abra_core::AspectRatio;
pub use crate::abra_core::Fill;
pub use crate::abra_core::LineJoin;
pub use crate::abra_core::Path;
pub use crate::abra_core::Point;
pub use crate::abra_core::PointF;
pub use crate::abra_core::Polygon;

// Gradient and drawing helpers
pub use crate::abra_core::Gradient;
// pub use crate::drawing::fill;

// TransformAlgorithm enum
pub use crate::abra_core::TransformAlgorithm;

// Plugins
pub use crate::plugin::*;
