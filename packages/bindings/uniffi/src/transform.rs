//! Rotating, flipping and resizing an image, from the `transform` module of `abra-core`.
//!
//! A transform is made by its function and then applied to an image, as in the library: `rotate(90).apply(image)`. The
//! library changes the image in place, so `apply` does too.
//!
//! Not exported: `crop`, `zoom` and `warp`, and the `Transform` trait of one-call shortcuts on `Image`.

use std::sync::Arc;

use abra::abra_core::image::gpu::Hardware;
use abra::abra_core::{ResizeTarget, TransformAlgorithm};
use abra::transform::prelude::{self as library, FlipAxis, TransformFit};

use crate::Image;

// Which hardware does the work: the CPU, the GPU (the CPU when there is none, or it fails), or `Auto`, which follows the `gpu.enabled` setting.
#[uniffi::remote(Enum)]
pub enum Hardware {
  Auto,
  Cpu,
  Gpu,
}

// How the canvas is sized after a transform that leaves areas the source image does not cover.
#[uniffi::remote(Enum)]
pub enum TransformFit {
  Expand,
  Crop,
  Fill,
}

// The axis an image is mirrored across.
#[uniffi::remote(Enum)]
pub enum FlipAxis {
  Horizontal,
  Vertical,
}

/// A rotation that has been described but not yet run. Made by [`rotate`].
#[derive(uniffi::Object)]
pub struct RotateImage(library::RotateImage);

/// Rotates the image around its center by this many degrees. Positive values rotate clockwise.
#[uniffi::export]
pub fn rotate(degrees: f64) -> Arc<RotateImage> {
  Arc::new(RotateImage(library::rotate(degrees)))
}

#[uniffi::export]
impl RotateImage {
  /// A copy of the rotation that sizes the canvas as `fit` says. The library defaults to `Expand`.
  pub fn with_fit(&self, fit: TransformFit) -> Arc<RotateImage> {
    Arc::new(RotateImage(
      library::rotate(self.0.degrees).with_fit(fit).with_algorithm(self.0.algorithm).with_hardware(self.0.hardware),
    ))
  }

  /// A copy of the rotation that samples with this algorithm. Without one, the best is picked automatically.
  pub fn with_algorithm(&self, algorithm: Option<TransformAlgorithm>) -> Arc<RotateImage> {
    Arc::new(RotateImage(
      library::rotate(self.0.degrees).with_fit(self.0.fit).with_algorithm(algorithm).with_hardware(self.0.hardware),
    ))
  }

  /// A copy of the rotation that runs on this hardware. The library defaults to `Auto`.
  pub fn with_hardware(&self, hardware: Hardware) -> Arc<RotateImage> {
    Arc::new(RotateImage(
      library::rotate(self.0.degrees).with_fit(self.0.fit).with_algorithm(self.0.algorithm).with_hardware(hardware),
    ))
  }

  /// Rotates the image in place.
  pub fn apply(&self, image: Arc<Image>) {
    image.edit(|image| self.0.apply(image));
  }
}

/// A flip that has been described but not yet run. Made by [`flip`].
#[derive(uniffi::Object)]
pub struct FlipImage(library::FlipImage);

/// Flips an image along the requested axis.
#[uniffi::export]
pub fn flip(axis: FlipAxis) -> Arc<FlipImage> {
  Arc::new(FlipImage(library::flip(axis)))
}

#[uniffi::export]
impl FlipImage {
  /// Flips the image in place.
  pub fn apply(&self, image: Arc<Image>) {
    image.edit(|image| self.0.apply(image));
  }
}

/// A resize that has been described but not yet run. Made by [`resize`].
#[derive(uniffi::Object)]
pub struct ResizeImage(library::ResizeImage);

/// Resizes the image to the target. `Fit` keeps the proportions and stays inside the size, so it also enlarges a smaller image.
#[uniffi::export]
pub fn resize(target: ResizeTarget) -> Arc<ResizeImage> {
  Arc::new(ResizeImage(library::resize(target)))
}

#[uniffi::export]
impl ResizeImage {
  /// A copy of the resize that samples with this algorithm. Without one, the best is picked automatically.
  pub fn with_algorithm(&self, algorithm: Option<TransformAlgorithm>) -> Arc<ResizeImage> {
    Arc::new(ResizeImage(library::resize(self.0.target).with_algorithm(algorithm).with_hardware(self.0.hardware)))
  }

  /// A copy of the resize that runs on this hardware. The library defaults to `Auto`.
  pub fn with_hardware(&self, hardware: Hardware) -> Arc<ResizeImage> {
    Arc::new(ResizeImage(library::resize(self.0.target).with_algorithm(self.0.algorithm).with_hardware(hardware)))
  }

  /// Resizes the image in place.
  pub fn apply(&self, image: Arc<Image>) {
    image.edit(|image| self.0.apply(image));
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra::abra_core::{Channels, Image as CoreImage};

  fn gradient(width: u32, height: u32) -> (Arc<Image>, CoreImage) {
    let pixels: Vec<u8> = (0..width * height).flat_map(|i| [(i % 251) as u8, (i / 7 % 251) as u8, 40, 255]).collect();
    let core = CoreImage::new_from_pixels(width, height, pixels, Channels::RGBA);
    (Arc::new(Image::from_core(core.clone())), core)
  }

  #[test]
  fn rotate_matches_the_library() {
    for (degrees, fit) in [
      (90.0, TransformFit::Expand),
      (-90.0, TransformFit::Fill),
      (33.0, TransformFit::Crop),
    ] {
      let (handle, mut expected) = gradient(24, 16);
      rotate(degrees).with_fit(fit).apply(handle.clone());
      library::rotate(degrees).with_fit(fit).apply(&mut expected);
      assert_eq!(handle.core().dimensions::<u32>(), expected.dimensions::<u32>(), "{degrees} {fit:?}");
      assert_eq!(handle.core().rgba(), expected.rgba(), "{degrees} {fit:?}");
    }
  }

  #[test]
  fn with_algorithm_keeps_the_fit_and_with_fit_keeps_the_algorithm() {
    let (handle, mut expected) = gradient(24, 16);
    rotate(20.0).with_algorithm(Some(TransformAlgorithm::Bilinear)).with_fit(TransformFit::Fill).apply(handle.clone());
    library::rotate(20.0)
      .with_fit(TransformFit::Fill)
      .with_algorithm(TransformAlgorithm::Bilinear)
      .apply(&mut expected);
    assert_eq!(handle.core().rgba(), expected.rgba());
  }

  #[test]
  fn resize_matches_the_library() {
    let (handle, mut expected) = gradient(40, 20);
    let target = ResizeTarget::Fit(abra::abra_core::Size { width: 10.0, height: 10.0 });
    resize(target).apply(handle.clone());
    library::resize(target).apply(&mut expected);
    assert_eq!(handle.core().dimensions::<u32>(), (10, 5));
    assert_eq!(handle.core().rgba(), expected.rgba());
  }

  #[test]
  fn the_hardware_survives_the_other_builders() {
    let rotation = rotate(20.0).with_hardware(Hardware::Cpu).with_algorithm(None).with_fit(TransformFit::Fill);
    assert_eq!(rotation.0.hardware, Hardware::Cpu);
    let resizing = resize(ResizeTarget::Scale(2.0)).with_hardware(Hardware::Gpu).with_algorithm(None);
    assert_eq!(resizing.0.hardware, Hardware::Gpu);
  }

  #[test]
  fn flip_matches_the_library() {
    for axis in [FlipAxis::Horizontal, FlipAxis::Vertical] {
      let (handle, mut expected) = gradient(9, 5);
      flip(axis).apply(handle.clone());
      library::flip(axis).apply(&mut expected);
      assert_eq!(handle.core().rgba(), expected.rgba());
      assert_ne!(handle.core().rgba(), gradient(9, 5).1.rgba(), "the flip changed the pixels");
    }
  }
}
