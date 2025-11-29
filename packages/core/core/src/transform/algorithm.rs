use std::fmt::Display;

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

/// Displays the name of the resize algorithm that is being used.
impl Display for TransformAlgorithm {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TransformAlgorithm::NearestNeighbor => write!(f, "NearestNeighbor"),
      TransformAlgorithm::Bilinear => write!(f, "Bilinear"),
      TransformAlgorithm::Bicubic => write!(f, "Bicubic"),
      TransformAlgorithm::Lanczos => write!(f, "Lanczos"),
      TransformAlgorithm::EdgeDirectNEDI => write!(f, "EdgeDirectNEDI"),
      TransformAlgorithm::EdgeDirectEDI => write!(f, "EdgeDirectEDI"),
      TransformAlgorithm::Auto => write!(f, "Auto"),
    }
  }
}
