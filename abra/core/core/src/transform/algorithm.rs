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
  /// `Auto` uses bicubic. The edge-directed algorithms only exist for resizing, so wherever samples are taken at
  /// arbitrary positions, such as when warping, they use Lanczos.
  pub fn interpolation(&self) -> Interpolation {
    match self {
      TransformAlgorithm::NearestNeighbor => Interpolation::Nearest,
      TransformAlgorithm::Bilinear => Interpolation::Bilinear,
      TransformAlgorithm::Bicubic | TransformAlgorithm::Auto => Interpolation::Bicubic,
      TransformAlgorithm::Lanczos | TransformAlgorithm::EdgeDirectNEDI | TransformAlgorithm::EdgeDirectEDI => {
        Interpolation::Lanczos
      }
    }
  }
}

/// Displays the name of the resize algorithm that is being used.
impl Display for TransformAlgorithm {
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TransformAlgorithm::NearestNeighbor => write!(p_f, "NearestNeighbor"),
      TransformAlgorithm::Bilinear => write!(p_f, "Bilinear"),
      TransformAlgorithm::Bicubic => write!(p_f, "Bicubic"),
      TransformAlgorithm::Lanczos => write!(p_f, "Lanczos"),
      TransformAlgorithm::EdgeDirectNEDI => write!(p_f, "EdgeDirectNEDI"),
      TransformAlgorithm::EdgeDirectEDI => write!(p_f, "EdgeDirectEDI"),
      TransformAlgorithm::Auto => write!(p_f, "Auto"),
    }
  }
}
