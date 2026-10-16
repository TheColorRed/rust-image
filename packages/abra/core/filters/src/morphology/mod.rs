//! Growing and shrinking the bright parts of an image (morphology).
//!
//! [`dilate`] makes the bright parts grow and [`erode`] makes them shrink, each color channel on its own. On a mask picture
//! (white where it reaches) they move the edge of the region outward or inward, which is what `Mask::dilated` and `Mask::eroded` do
//! to a mask directly. These are effects, so they also run on the GPU, with the same pixels.

mod window;

pub use window::{Morphology, MorphologyKind, dilate, erode};
