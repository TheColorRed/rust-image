//! Geometry module

mod area;
mod homography;
mod line;
mod line_segment;
mod path;
mod pointf;
mod quad;
mod rect;
mod shape;
mod size;
mod stroke;

pub use area::{Area, polygon_contains};
pub use homography::Homography;
pub use line::bresenham;
pub use line_segment::{LineSegment, Orientation};
pub use path::{Path, Segment};
pub use pointf::PointF;
pub use quad::Quad;
pub use rect::{Alignment, AspectRatio, PreserveAspectRatio, Rect};
pub use shape::Shape;
pub use size::Size;
pub use stroke::{AreaStroke, LineCap, LineJoin, PathStroke};
