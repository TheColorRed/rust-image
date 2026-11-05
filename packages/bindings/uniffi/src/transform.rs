//! Cropping, resizing, rotating and flipping an image, from the `transform` module of `abra-core`.
//!
//! One transform does all of it, as in the library: `transform().with_scale(2).with_rotation(90).apply(image)`. The library picks
//! the order the pieces run in, and changes the image in place, so `apply` does too.
//!
//! Not exported: `zoom` and `warp`, and the `Transform` trait of shortcuts on layers and canvases.

use std::sync::Arc;

use abra::abra_core::image::gpu::Hardware;
use abra::abra_core::{Rect, ResizeTarget, TransformAlgorithm};
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

/// A transform that has been described but not yet run. Made by [`transform`].
#[derive(uniffi::Object)]
pub struct TransformImage(library::TransformImage);

/// Starts a transform that does nothing yet. Add a crop, a size or scale, a rotation or flips, then `apply` it to an image.
#[uniffi::export]
pub fn transform() -> Arc<TransformImage> {
  Arc::new(TransformImage(library::transform()))
}

#[uniffi::export]
impl TransformImage {
  /// A copy of the transform that keeps only this area, clipped to the image.
  pub fn with_crop(&self, area: Rect) -> Arc<TransformImage> {
    Arc::new(TransformImage(self.0.clone().with_crop(area)))
  }

  /// A copy of the transform that resizes to the target, after the crop and before the rotation. `Fit` keeps the proportions and stays
  /// inside the size, so it also enlarges a smaller image.
  pub fn with_size(&self, target: ResizeTarget) -> Arc<TransformImage> {
    Arc::new(TransformImage(self.0.clone().with_size(target)))
  }

  /// A copy of the transform that scales both dimensions by this factor.
  pub fn with_scale(&self, scale: f32) -> Arc<TransformImage> {
    Arc::new(TransformImage(self.0.clone().with_scale(scale)))
  }

  /// A copy of the transform that rotates the image around its center by this many degrees. Positive values rotate clockwise.
  pub fn with_rotation(&self, degrees: f64) -> Arc<TransformImage> {
    Arc::new(TransformImage(self.0.clone().with_rotation(degrees)))
  }

  /// A copy of the transform that also mirrors the image across the axis. Flipping the same axis twice puts the image back.
  pub fn with_flip(&self, axis: FlipAxis) -> Arc<TransformImage> {
    Arc::new(TransformImage(self.0.clone().with_flip(axis)))
  }

  /// A copy of the transform that sizes the canvas as `fit` says after a rotation. The library defaults to `Expand`.
  pub fn with_fit(&self, fit: TransformFit) -> Arc<TransformImage> {
    Arc::new(TransformImage(self.0.clone().with_fit(fit)))
  }

  /// A copy of the transform that samples with this algorithm. Without one, the best is picked automatically.
  pub fn with_algorithm(&self, algorithm: Option<TransformAlgorithm>) -> Arc<TransformImage> {
    Arc::new(TransformImage(self.0.clone().with_algorithm(algorithm)))
  }

  /// A copy of the transform that runs on this hardware. The library defaults to `Auto`.
  pub fn with_hardware(&self, hardware: Hardware) -> Arc<TransformImage> {
    Arc::new(TransformImage(self.0.clone().with_hardware(hardware)))
  }

  /// Runs the transform on the image in place.
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
  fn a_transform_matches_the_library() {
    for (degrees, fit) in [(90.0, TransformFit::Expand), (-90.0, TransformFit::Fill), (33.0, TransformFit::Crop)] {
      let (handle, mut expected) = gradient(24, 16);
      transform().with_rotation(degrees).with_fit(fit).apply(handle.clone());
      library::transform().with_rotation(degrees).with_fit(fit).apply(&mut expected);
      assert_eq!(handle.core().dimensions::<u32>(), expected.dimensions::<u32>(), "{degrees} {fit:?}");
      assert_eq!(handle.core().rgba(), expected.rgba(), "{degrees} {fit:?}");
    }
  }

  #[test]
  fn the_pieces_of_a_transform_are_kept_by_the_other_builders() {
    let (handle, mut expected) = gradient(40, 20);
    let area = Rect::new((4, 2), (30, 16));
    let size = ResizeTarget::Fit(abra::abra_core::Size { width: 10.0, height: 10.0 });
    transform()
      .with_crop(area)
      .with_size(size)
      .with_flip(FlipAxis::Horizontal)
      .with_rotation(20.0)
      .with_algorithm(Some(TransformAlgorithm::Bilinear))
      .with_hardware(Hardware::Cpu)
      .apply(handle.clone());
    library::transform()
      .with_crop(area)
      .with_size(size)
      .with_flip(FlipAxis::Horizontal)
      .with_rotation(20.0)
      .with_algorithm(TransformAlgorithm::Bilinear)
      .with_hardware(Hardware::Cpu)
      .apply(&mut expected);
    assert_eq!(handle.core().dimensions::<u32>(), expected.dimensions::<u32>());
    assert_eq!(handle.core().rgba(), expected.rgba());
  }

  #[test]
  fn flip_changes_the_pixels() {
    for axis in [FlipAxis::Horizontal, FlipAxis::Vertical] {
      let (handle, mut expected) = gradient(9, 5);
      transform().with_flip(axis).apply(handle.clone());
      library::transform().with_flip(axis).apply(&mut expected);
      assert_eq!(handle.core().rgba(), expected.rgba());
      assert_ne!(handle.core().rgba(), gradient(9, 5).1.rgba(), "the flip changed the pixels");
    }
  }
}
