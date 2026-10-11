//! How the canvas is sized after a transform that moves pixels off their grid, such as a rotation.
//!
//! Turning or warping an image leaves areas the source does not cover. [`TransformFit`] picks whether to keep
//! those areas as transparent pixels, crop them away, or crop them away and scale back up to the original size.

/// How the canvas is sized after a transform that leaves areas the source image does not cover, such as the
/// corners a rotation uncovers or the edges a perspective warp pulls in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformFit {
  /// Grow the canvas to show the whole transformed image. Areas the source does not cover are transparent. Default.
  #[default]
  Expand,
  /// Keep only an area the source fully covers, at its own size, so the result is usually smaller than the
  /// original. For a rotation this is the largest rectangle with no empty corners; for a warp it is the area the
  /// four points were moved to.
  Crop,
  /// Keep the original size, zoomed in just enough that the source covers every pixel. The transform and the
  /// zoom are done in one pass, so the image is only resampled once.
  Fill,
}
