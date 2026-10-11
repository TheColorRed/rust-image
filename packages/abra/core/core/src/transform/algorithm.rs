use std::fmt::Display;

use crate::Interpolation;

#[derive(Clone, Copy, Debug, PartialEq)]
/// Algorithms for transforming images such as resizing or rotating.
/// Each algorithm offers a different balance between performance and quality.
pub enum TransformAlgorithm {
  /// Nearest neighbor interpolation. Fast but low quality.
  NearestNeighbor,
  /// Blends 4 neighboring pixels. Good balance between quality and performance.
  Bilinear,
  /// Uses a cubic kernel over 16 pixels (4x4 neighborhood). Better quality than bilinear, noticeable improvement for downscaling.
  Bicubic,
  /// Uses Lanczos-3 kernel over 36 pixels (6x6 neighborhood). Highest quality, best edge preservation, but most computationally expensive.
  Lanczos,
  /// Edge-Directed NEDI algorithm for high-quality resizing with edge preservation.
  /// Slower than Edge-Directed EDI.
  EdgeDirectNEDI,
  /// Edge-Directed EDI algorithm for high-quality resizing with edge preservation.
  /// Faster than Edge-Directed NEDI.
  EdgeDirectEDI,
  /// Automatically selects the best algorithm based on the image and target size.
  Auto,
}

impl TransformAlgorithm {
  /// The interpolation to sample pixels with for this algorithm.
  ///
  /// `Auto` uses bicubic.
  pub fn interpolation(&self) -> Interpolation {
    match self {
      TransformAlgorithm::NearestNeighbor => Interpolation::Nearest,
      TransformAlgorithm::Bilinear => Interpolation::Bilinear,
      TransformAlgorithm::Bicubic | TransformAlgorithm::Auto => Interpolation::Bicubic,
      TransformAlgorithm::Lanczos => Interpolation::Lanczos,
      TransformAlgorithm::EdgeDirectNEDI => Interpolation::EdgeDirectedNedi,
      TransformAlgorithm::EdgeDirectEDI => Interpolation::EdgeDirectedEdi,
    }
  }
}

/// Displays the name of the algorithm, such as `Bicubic`.
impl Display for TransformAlgorithm {
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Debug::fmt(self, p_f)
  }
}
