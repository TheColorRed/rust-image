use crate::Crop;
use crate::Image;
use crate::IntoNumber;
use crate::PointF;
use crate::Resize;
use crate::ResizeTarget;
use crate::TransformAlgorithm;
use primitives::Image as PrimitiveImage;

/// A zoom that has been described but not yet run. Create one with [`zoom`], optionally set the algorithm
/// with [`ZoomImage::with_algorithm`], then run it with [`ZoomImage::apply`].
pub struct ZoomImage {
  pub anchor: PointF,
  pub factor: f32,
  pub algorithm: Option<TransformAlgorithm>,
}

impl ZoomImage {
  /// Sets the resizing algorithm. When `None` (the default), the best algorithm is selected automatically.
  pub fn with_algorithm(mut self, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> Self {
    self.algorithm = p_algorithm.into();
    self
  }

  /// Zooms the image in place. Nothing happens when the factor is `<= 1.0`.
  pub fn apply(&self, p_image: &mut Image) {
    let factor = self.factor;
    // Also rejects NaN.
    if !(factor > 1.0) {
      return;
    }

    let (width, height) = p_image.dimensions::<u32>();
    let anchor = self.anchor;

    let crop_width = ((width as f32 / factor).round() as u32).clamp(1, width);
    let crop_height = ((height as f32 / factor).round() as u32).clamp(1, height);
    // Place the viewport so the anchor sits at the same relative position inside it as it does in the full image.
    // An anchor at 0 puts the viewport at the left/top edge, and an anchor at the far edge puts it at the right/bottom.
    let max_x = (width - crop_width) as f32;
    let max_y = (height - crop_height) as f32;
    let x = (anchor.x / width as f32 * max_x).round().clamp(0.0, max_x) as u32;
    let y = (anchor.y / height as f32 * max_y).round().clamp(0.0, max_y) as u32;

    p_image.crop(x, y, crop_width, crop_height);
    p_image.resize(ResizeTarget::Exact((width, height).into()), self.algorithm);
  }
}

/// Zooms into the image toward an anchor point, keeping the original dimensions.
/// The anchor stays at the same position in the output (like zooming toward a cursor), and the visible region always
/// stays within the image bounds. Anchors outside the image are clamped to its edges.
/// # Arguments
/// - `p_anchor`: The point to zoom toward, in source image pixel coordinates.
/// - `p_factor`: The magnification factor (e.g. `2.0` = 2×). Values `<= 1.0` leave the image unchanged.
///
/// The resizing algorithm is chosen automatically unless set with [`ZoomImage::with_algorithm`].
pub fn zoom(p_anchor: impl Into<PointF>, p_factor: impl IntoNumber) -> ZoomImage {
  ZoomImage {
    anchor: p_anchor.into(),
    factor: p_factor.into::<f32>(),
    algorithm: None,
  }
}

/// Trait for zooming functionality.
pub trait Zoom {
  /// Zoom the image according to the supplied target.
  /// - `p_anchor`: The point to zoom toward, in source image pixel coordinates.
  /// - `p_factor`: The magnification factor (e.g. `2.0` = 2×). Values `<= 1.0` leave the image unchanged.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn zoom(
    &mut self, p_anchor: impl Into<PointF>, p_factor: impl IntoNumber,
    p_algorithm: impl Into<Option<TransformAlgorithm>>,
  );
}

impl Zoom for PrimitiveImage {
  /// Zoom the image according to the supplied target.
  /// - `p_anchor`: The point to zoom toward, in source image pixel coordinates.
  /// - `p_factor`: The magnification factor (e.g. `2.0` = 2×). Values `<= 1.0` leave the image unchanged.
  /// - `p_algorithm`: The resizing algorithm to use. If None, the best algorithm will be selected automatically.
  fn zoom(
    &mut self, p_anchor: impl Into<PointF>, p_factor: impl IntoNumber,
    p_algorithm: impl Into<Option<TransformAlgorithm>>,
  ) {
    crate::transform::zoom(p_anchor, p_factor).with_algorithm(p_algorithm).apply(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Builds an image where each pixel's red channel is its x coordinate and green is its y coordinate.
  fn coordinate_image(p_width: u32, p_height: u32) -> Image {
    let mut pixels = Vec::with_capacity((p_width * p_height * 4) as usize);
    for y in 0..p_height {
      for x in 0..p_width {
        pixels.extend_from_slice(&[x as u8, y as u8, 0, 255]);
      }
    }
    let mut image = Image::new(p_width, p_height);
    image.set_new_pixels(&pixels, p_width, p_height);
    image
  }

  #[test]
  fn keeps_dimensions() {
    let mut image = coordinate_image(40, 20);
    zoom((20.0, 10.0), 2.0).with_algorithm(TransformAlgorithm::NearestNeighbor).apply(&mut image);
    assert_eq!(image.dimensions::<u32>(), (40, 20));
  }

  #[test]
  fn factor_at_or_below_one_is_noop() {
    for factor in [1.0, 0.5, 0.0, -2.0, f32::NAN] {
      let mut image = coordinate_image(8, 8);
      let before = image.rgba().to_vec();
      zoom((4.0, 4.0), factor).apply(&mut image);
      assert_eq!(image.rgba(), &before[..], "factor {factor}");
    }
  }

  /// Reads the red and green channels (source x and y) of the pixel at the given position.
  fn source_coordinate(p_image: &Image, p_x: u32, p_y: u32) -> (u8, u8) {
    let (width, _) = p_image.dimensions::<u32>();
    let index = ((p_y * width + p_x) * 4) as usize;
    let pixels = p_image.rgba();
    (pixels[index], pixels[index + 1])
  }

  #[test]
  fn centered_anchor_shows_middle() {
    let mut image = coordinate_image(40, 40);
    zoom((20.0, 20.0), 2.0).with_algorithm(TransformAlgorithm::NearestNeighbor).apply(&mut image);
    // 2× toward (20, 20) shows source region (10, 10)..(30, 30).
    assert_eq!(source_coordinate(&image, 0, 0), (10, 10));
  }

  #[test]
  fn anchor_stays_in_place() {
    for anchor in [(0u32, 0u32), (8, 8), (20, 30), (32, 16), (39, 39)] {
      let mut image = coordinate_image(40, 40);
      zoom((anchor.0 as f32, anchor.1 as f32), 2.0).with_algorithm(TransformAlgorithm::NearestNeighbor).apply(&mut image);
      let (x, y) = source_coordinate(&image, anchor.0, anchor.1);
      // Allow one pixel of rounding.
      assert!(x.abs_diff(anchor.0 as u8) <= 1 && y.abs_diff(anchor.1 as u8) <= 1, "anchor {anchor:?} -> ({x}, {y})");
    }
  }

  #[test]
  fn viewport_stays_within_image() {
    for anchor in [(0.0, 0.0), (39.0, 0.0), (0.0, 39.0), (39.0, 39.0), (-100.0, 500.0)] {
      let mut image = coordinate_image(40, 40);
      zoom(anchor, 4.0).with_algorithm(TransformAlgorithm::NearestNeighbor).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), (40, 40), "anchor {anchor:?}");
    }

    // Anchoring at the top-left corner shows the first 10×10 region.
    let mut image = coordinate_image(40, 40);
    zoom((0.0, 0.0), 4.0).with_algorithm(TransformAlgorithm::NearestNeighbor).apply(&mut image);
    assert_eq!(source_coordinate(&image, 0, 0), (0, 0));

    // Anchoring at the bottom-right corner shows the last 10×10 region.
    let mut image = coordinate_image(40, 40);
    zoom((40.0, 40.0), 4.0).with_algorithm(TransformAlgorithm::NearestNeighbor).apply(&mut image);
    assert_eq!(source_coordinate(&image, 0, 0), (30, 30));
  }
}
