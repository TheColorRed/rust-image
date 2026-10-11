//! Image transformation functions.
//!
//! Each transform is a builder: create it with its function ([`crop`], [`resize`], [`rotate`], [`zoom`], [`flip`],
//! [`warp`]), configure it, then run it with `apply`. The [`Transform`] trait adds one-call shortcuts on [`Image`].

mod algorithm;
mod crop;
mod fit;
mod flip;
mod interpolation;
mod resize;
mod rotate;
mod warp;
mod zoom;

pub use algorithm::*;
pub use crop::*;
pub use fit::*;
pub use flip::*;
pub use interpolation::*;
pub use resize::*;
pub use rotate::*;
pub use warp::*;
pub use zoom::*;

use crate::{Image, IntoNumber};

/// Crop, resize, rotate, and flip, implemented by [`Image`] and by the layer and canvas transform proxies in the
/// `canvas` crate. On an image these are one-call shortcuts for the builders; use the builders directly for options
/// such as [`RotateImage::with_fit`].
pub trait Transform {
  /// What [`Transform::resize`] accepts: [`ResizeTarget`] for images and layers.
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

impl Transform for Image {
  type Target = ResizeTarget;

  fn crop(&mut self, p_x: impl IntoNumber, p_y: impl IntoNumber, p_width: impl IntoNumber, p_height: impl IntoNumber) {
    crop(p_x, p_y, p_width, p_height).apply(self);
  }

  fn resize(&mut self, p_target: ResizeTarget, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
    resize(p_target).with_algorithm(p_algorithm).apply(self);
  }

  fn rotate(&mut self, p_degrees: impl IntoNumber, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
    rotate(p_degrees).with_algorithm(p_algorithm).apply(self);
  }

  fn flip(&mut self, p_axis: FlipAxis) {
    flip(p_axis).apply(self);
  }
}
