//! Geometry module

mod area;
mod boolean;
mod curve;
mod homography;
mod line;
mod line_segment;
mod outline;
mod path;
mod pointf;
mod quad;
mod rect;
mod shape;
mod size;
mod stroke;
mod stroke_opacity;
mod stroke_profile;
mod stroke_width;

pub use area::{Area, polygon_contains};
pub use boolean::{polygon_difference, polygon_union};
pub use curve::Curve;
pub use homography::Homography;
pub use line::bresenham;
pub use line_segment::{LineSegment, Orientation};
pub use outline::{simplify_points, smoothed_area};
pub use path::{ClosestQuery, Path, Segment};
pub use pointf::PointF;
pub use quad::Quad;
pub use rect::{Alignment, AspectRatio, PreserveAspectRatio, Rect};
pub use shape::Shape;
pub use size::Size;
pub use stroke::{AreaStroke, LineCap, LineJoin, PathStroke};
pub use stroke_opacity::{OpacityStop, StrokeOpacity};
pub use stroke_profile::StrokeEasing;
pub use stroke_width::{StrokeWidth, WidthStop};
