//! Geometry module

mod area;
mod line;
mod line_segment;
mod path;
mod point;
mod pointf;
mod shapes;
mod size;
mod stroke;
mod viewbox;

pub use area::Area;
pub use line::{bresenham, bresenham_from_points};
pub use line_segment::{LineSegment, Orientation};
pub use path::{Path, Segment};
pub use point::Point;
pub use pointf::PointF;
pub use shapes::*;
pub use size::Size;
pub use stroke::{AreaStroke, LineCap, LineJoin, PathStroke};
pub use viewbox::{Alignment, AspectRatio, PreserveAspectRatio, ViewBox};
