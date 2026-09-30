use crate::transform::TransformAlgorithm;
use crate::transform::interpolation;
use crate::{Channels, Image, Size};

/// Describes how an image should be resized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResizeTarget {
  /// Resize to exact dimensions without preserving aspect ratio.
  Exact(Size),
  /// Resize to a width while preserving aspect ratio.
  FitWidth(u32),
  /// Resize to a height while preserving aspect ratio.
  FitHeight(u32),
  /// Resize to the largest size that fits inside the given size while preserving aspect ratio.
  Fit(Size),
  /// Change the width by a number of pixels while preserving aspect ratio.
  RelativeWidth(i32),
  /// Change the height by a number of pixels while preserving aspect ratio.
  RelativeHeight(i32),
  /// Resize both dimensions by a positive scale factor.
  Scale(f32),
}

/// Determine the best resize algorithm based on the original and target dimensions.
/// If no algorithm is specified, this function selects an appropriate algorithm:
/// - If the target size is less than half the original size, Lanczos is chosen for high quality downscaling.
/// - If the target size is larger than the original size, Bicubic is chosen for quality upscaling.
/// - If the target size is smaller than the original size but not less than half, Bilinear is chosen for a good balance.
/// - If the target size is the same as the original size, Bilinear is used as a default.
pub(crate) fn get_resize_algorithm(
  p_algorithm: impl Into<Option<TransformAlgorithm>>, p_old_width: u32, p_old_height: u32, p_width: u32, p_height: u32,
) -> TransformAlgorithm {
  match p_algorithm.into() {
    Some(TransformAlgorithm::Auto) | None => {
      // Uses the Lanczos algorithm when downscaling more than half for best quality.
      if p_width < p_old_width / 2 || p_height < p_old_height / 2 {
        TransformAlgorithm::Lanczos
      }
      // Uses Bicubic when upscaling for better quality.
      else if p_width > p_old_width || p_height > p_old_height {
        TransformAlgorithm::Bicubic
      }
      // Uses Bilinear for moderate downscaling.
      else if p_width < p_old_width || p_height < p_old_height {
        TransformAlgorithm::Bilinear
      } else {
        TransformAlgorithm::Bicubic
      }
    }
    Some(algo) => algo,
  }
}

/// Resolves an image resize target to exact pixel dimensions.
fn target_dimensions(p_image: &Image, p_target: ResizeTarget) -> Option<(u32, u32)> {
  let (old_width, old_height) = p_image.dimensions::<u32>();
  match p_target {
    ResizeTarget::Exact(size) => Some((size.width.max(0.0) as u32, size.height.max(0.0) as u32)),
    ResizeTarget::FitWidth(width) => {
      let height = ((old_height as f32 / old_width as f32 * width as f32) as u32).max(1);
      Some((width, height))
    }
    ResizeTarget::FitHeight(height) => {
      let width = (old_width as f32 / old_height as f32 * height as f32) as u32;
      Some((width, height))
    }
    ResizeTarget::Fit(size) => {
      let scale = (size.width / old_width as f32).min(size.height / old_height as f32);
      target_dimensions(p_image, ResizeTarget::Scale(scale))
    }
    ResizeTarget::RelativeWidth(amount) => {
      let width = (old_width as i32 + amount).max(1) as u32;
      target_dimensions(p_image, ResizeTarget::FitWidth(width))
    }
    ResizeTarget::RelativeHeight(amount) => {
      let height = (old_height as i32 + amount).max(1) as u32;
      target_dimensions(p_image, ResizeTarget::FitHeight(height))
    }
    ResizeTarget::Scale(scale) if scale > 0.0 => {
      Some(((old_width as f32 * scale).max(1.0) as u32, (old_height as f32 * scale).max(1.0) as u32))
    }
    ResizeTarget::Scale(_) => None,
  }
}

/// A resize that has been described but not yet run. Create one with [`resize`], optionally set the algorithm
/// with [`ResizeImage::with_algorithm`], then run it with [`ResizeImage::apply`].
pub struct ResizeImage {
  pub target: ResizeTarget,
  pub algorithm: Option<TransformAlgorithm>,
}

impl ResizeImage {
  /// Sets the resizing algorithm. When `None` (the default), the best algorithm is selected automatically.
  pub fn with_algorithm(mut self, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> Self {
    self.algorithm = p_algorithm.into();
    self
  }

  /// Resizes the image. Nothing happens when the resolved dimensions are the same as the image's.
  pub fn apply(&self, p_image: &mut Image) {
    let (old_width, old_height) = p_image.dimensions::<u32>();
    let Some((width, height)) = target_dimensions(p_image, self.target) else {
      return;
    };
    if width == old_width && height == old_height {
      return;
    }

    let algorithm = get_resize_algorithm(self.algorithm, old_width, old_height, width, height);
    let pixels = interpolation::resample(p_image, width, height, algorithm.interpolation());
    p_image.set_pixels(width, height, pixels, Channels::RGBA);
  }

  /// Returns a resized copy and leaves the source untouched. Only the output is allocated, so this is cheaper
  /// than cloning and then calling [`ResizeImage::apply`].
  pub fn resized(&self, p_image: &Image) -> Image {
    let (old_width, old_height) = p_image.dimensions::<u32>();
    let Some((width, height)) = target_dimensions(p_image, self.target) else {
      return p_image.clone();
    };
    if width == old_width && height == old_height {
      return p_image.clone();
    }

    let algorithm = get_resize_algorithm(self.algorithm, old_width, old_height, width, height);
    let pixels = interpolation::resample(p_image, width, height, algorithm.interpolation());
    Image::new_from_pixels(width, height, pixels, Channels::RGBA)
  }
}

/// Resizes an image according to an exact or aspect-preserving target.
/// # Arguments
/// - `p_target`: The desired dimensions or resizing strategy.
///
/// The resizing algorithm is chosen automatically unless set with [`ResizeImage::with_algorithm`].
pub fn resize(p_target: ResizeTarget) -> ResizeImage {
  ResizeImage {
    target: p_target,
    algorithm: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn image(p_width: u32, p_height: u32) -> Image {
    Image::new_from_pixels(p_width, p_height, &[10, 20, 30, 255].repeat((p_width * p_height) as usize), Channels::RGBA)
  }

  #[test]
  fn builder_resizes_to_the_target() {
    let mut img = image(20, 10);
    resize(ResizeTarget::Exact(Size::new(8, 4))).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (8, 4));

    resize(ResizeTarget::FitWidth(16)).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (16, 8));
  }

  #[test]
  fn resized_returns_a_copy_and_leaves_the_source() {
    let img = image(40, 20);
    let copy = resize(ResizeTarget::Fit(Size::new(10, 10))).resized(&img);
    assert_eq!(copy.dimensions::<u32>(), (10, 5));
    assert_eq!(img.dimensions::<u32>(), (40, 20));
    // An interior pixel: resampling blends edge pixels with transparent samples outside the image.
    let interior = (2 * 10 + 5) * 4;
    assert_eq!(&copy.rgba()[interior..interior + 4], &[10, 20, 30, 255]);
  }

  #[test]
  fn fit_preserves_aspect_ratio_inside_the_box() {
    let mut img = image(40, 20);
    resize(ResizeTarget::Fit(Size::new(10, 10))).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (10, 5));

    let mut img = image(20, 40);
    resize(ResizeTarget::Fit(Size::new(10, 10))).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (5, 10));
  }

  #[test]
  fn builder_accepts_an_algorithm() {
    for algorithm in [TransformAlgorithm::NearestNeighbor, TransformAlgorithm::Lanczos] {
      let mut img = image(20, 10);
      resize(ResizeTarget::Scale(2.0)).with_algorithm(algorithm).apply(&mut img);
      assert_eq!(img.dimensions::<u32>(), (40, 20));
    }
  }

  #[test]
  fn builder_leaves_an_image_alone_when_the_size_is_unchanged() {
    let mut img = image(6, 3);
    let before = img.rgba().to_vec();
    resize(ResizeTarget::Exact(Size::new(6, 3))).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (6, 3));
    assert_eq!(img.rgba(), before.as_slice());
  }

  #[test]
  fn trait_method_still_works() {
    let mut img = image(10, 10);
    crate::Transform::resize(&mut img, ResizeTarget::Exact(Size::new(5, 5)), None);
    assert_eq!(img.dimensions::<u32>(), (5, 5));
  }

  #[test]
  fn edge_directed_algorithms_resize() {
    for algorithm in [TransformAlgorithm::EdgeDirectNEDI, TransformAlgorithm::EdgeDirectEDI] {
      let mut img = image(10, 6);
      resize(ResizeTarget::Scale(2.0)).with_algorithm(algorithm).apply(&mut img);
      assert_eq!(img.dimensions::<u32>(), (20, 12), "{algorithm}");
      // A flat color stays flat away from the transparent border.
      assert_eq!(img.get_pixel(10, 6), Some((10, 20, 30, 255)), "{algorithm}");
    }
  }
}
