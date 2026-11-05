//! Image transformation functions.
//!
//! Each transform is a builder: create it with its function ([`transform`], [`zoom`], [`warp`]), configure it, then run it with
//! `apply`. [`transform`] crops, resizes, rotates and flips in one go, and picks the order the pieces run in. The [`Transform`]
//! trait is what layers and canvases offer for the same operations on their contents.

mod algorithm;
mod crop;
mod fit;
mod flip;
mod interpolation;
mod resize;
mod rotate;
mod transform_image;
mod warp;
mod zoom;

pub use algorithm::*;
pub use fit::*;
pub use flip::FlipAxis;
pub use interpolation::*;
pub use resize::ResizeTarget;
pub use transform_image::*;
pub use warp::*;

pub(crate) use crop::crop;
pub(crate) use flip::flip;
pub use zoom::*;

use crate::IntoNumber;

/// Crop, resize, rotate, and flip, implemented by the layer and canvas transform proxies in the `canvas` crate. To change an
/// image's own pixels use [`transform`].
pub trait Transform {
  /// What [`Transform::resize`] accepts.
  type Target;

  /// Crops to the given rectangle. See [`crop`].
  fn crop(&mut self, p_x: impl IntoNumber, p_y: impl IntoNumber, p_width: impl IntoNumber, p_height: impl IntoNumber);
  /// Resizes to the target. `None` picks the algorithm automatically. See [`resize`].
  fn resize(&mut self, p_target: Self::Target, p_algorithm: impl Into<Option<TransformAlgorithm>>);
  /// Rotates clockwise by `p_degrees`, growing the canvas to fit. `None` picks the algorithm automatically. See
  /// [`rotate`].
  fn rotate(&mut self, p_degrees: impl IntoNumber, p_algorithm: impl Into<Option<TransformAlgorithm>>);
  /// Mirrors across the axis. See [`flip`].
  fn flip(&mut self, p_axis: FlipAxis);
}
