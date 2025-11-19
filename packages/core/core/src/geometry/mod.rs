//! Geometry module

mod area;
mod line;
mod path;
mod point;
mod pointf;
mod shapes;
mod size;
mod stroke;
mod viewbox;

pub use area::Area;
pub use line::{bresenham, bresenham_from_points};
pub use path::{Path, Segment};
pub use point::Point;
pub use pointf::PointF;
pub use shapes::*;
pub use size::Size;
pub use stroke::{LineCap, LineJoin};
pub use viewbox::{Alignment, AspectRatio, PreserveAspectRatio, ViewBox};
