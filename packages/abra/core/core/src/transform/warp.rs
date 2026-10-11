//! Perspective warping: moving four points of an image onto four other points.
//!
//! [`warp`] bends the image with a perspective transform, so straight lines stay straight while the shape changes,
//! the way a flat surface changes when seen from another angle. It is the building block for perspective
//! correction, for placing an image onto a shape, and for any other four-point transform.

use super::{EdgeMode, TransformAlgorithm, TransformFit, remap, resize::get_resize_algorithm};
use crate::{Channels, Homography, Image, Quad};

/// How far past the original frame [`TransformFit::Expand`] may grow the canvas on each side, as a share of the
/// image's size. A warp can stretch parts of the image out to infinity, so the canvas needs a limit.
const EXPAND_LIMIT: f64 = 0.5;

/// A warp that has been described but not yet run. Create one with [`warp`], optionally configure it with
/// [`WarpImage::with_fit`] and [`WarpImage::with_algorithm`], then run it with [`WarpImage::apply`].
pub struct WarpImage {
  pub from: Quad,
  pub to: Quad,
  pub fit: TransformFit,
  pub algorithm: Option<TransformAlgorithm>,
}

impl WarpImage {
  /// Sets how the canvas is sized after warping. Defaults to [`TransformFit::Expand`].
  /// - [`TransformFit::Expand`]: the whole warped image, with transparent areas where the source does not reach.
  ///   The canvas grows at most half the image's size past each edge of the original frame, since a strong warp
  ///   can stretch parts of the image out to infinity.
  /// - [`TransformFit::Crop`]: only the upright box around the `to` quad, at its own size.
  /// - [`TransformFit::Fill`]: the original frame, zoomed in just enough that every pixel comes from the source.
  pub fn with_fit(mut self, p_fit: TransformFit) -> Self {
    self.fit = p_fit;
    self
  }

  /// Sets the interpolation algorithm. When `None` (the default), the best algorithm is selected automatically.
  /// The edge-directed resize algorithms have no meaning here and use Lanczos.
  pub fn with_algorithm(mut self, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> Self {
    self.algorithm = p_algorithm.into();
    self
  }

  /// Warps the image in place. Nothing happens when either quad is degenerate, for example with three corners on a
  /// line.
  pub fn apply(&self, p_image: &mut Image) {
    let (image_width, image_height) = p_image.dimensions::<u32>();
    if image_width == 0 || image_height == 0 {
      return;
    }
    // Takes a position in the result back to a position in the source image.
    let Some(to_source) = Homography::from_quads(&self.to, &self.from) else {
      return;
    };
    let Some(window) = WarpWindow::new(self, &to_source, image_width, image_height) else {
      return;
    };

    let algorithm = get_resize_algorithm(
      self.algorithm,
      (window.width as f64 * window.scale) as u32,
      (window.height as f64 * window.scale) as u32,
      window.width as u32,
      window.height as u32,
    );
    let interpolation = algorithm.interpolation();

    // Filling keeps every pixel inside the source, but rounding can still put the outermost ones a hair past its
    // edges. Those take the nearest edge pixel, and the sampler repeats the edges so the result's edges do not fade.
    let clamp = self.fit == TransformFit::Fill;
    let edge = if clamp { EdgeMode::Clamp } else { EdgeMode::Transparent };
    let (max_x, max_y) = (image_width as f64 - 0.5, image_height as f64 - 0.5);
    let WarpWindow {
      left,
      top,
      scale,
      width,
      height,
    } = window;

    // Each output pixel's center maps back into the source image. Past the line where the warp goes to infinity
    // there is no source pixel, so the pixel stays transparent.
    let pixels = remap(p_image, width as u32, height as u32, interpolation, edge, |x, y| {
      let (source_x, source_y) = to_source.map_finite(left + x * scale, top + y * scale)?;
      Some(if clamp { (source_x.clamp(0.5, max_x), source_y.clamp(0.5, max_y)) } else { (source_x, source_y) })
    });

    p_image.set_pixels(width as u32, height as u32, pixels, Channels::RGBA);
  }
}

/// The window onto the warped plane the result shows: result pixel `x` shows the plane at `left + x * scale`.
struct WarpWindow {
  left: f64,
  top: f64,
  scale: f64,
  width: usize,
  height: usize,
}

impl WarpWindow {
  fn new(p_warp: &WarpImage, p_to_source: &Homography, p_width: u32, p_height: u32) -> Option<WarpWindow> {
    let (width, height) = (p_width as f64, p_height as f64);
    let window = |p_left: f64, p_top: f64, p_right: f64, p_bottom: f64| WarpWindow {
      left: p_left,
      top: p_top,
      scale: 1.0,
      width: (p_right - p_left).round().max(1.0) as usize,
      height: (p_bottom - p_top).round().max(1.0) as usize,
    };

    Some(match p_warp.fit {
      TransformFit::Crop => {
        let (left, top, right, bottom) = p_warp.to.bounds().edges::<f64>();
        window(left, top, right, bottom)
      }
      TransformFit::Fill => {
        // The largest part of the original frame that the source fully covers, shown at the original size.
        let bounds = (0.0, 0.0, width, height);
        let (left, top, scale) = p_to_source.covered_window((width, height), (width, height), bounds);
        WarpWindow {
          left,
          top,
          scale,
          width: p_width as usize,
          height: p_height as usize,
        }
      }
      TransformFit::Expand => {
        // Where the source's corners land, clipped to the limit. A corner past the horizon means the image stretches
        // out to infinity, so the limit is used as it is.
        let to_result = Homography::from_quads(&p_warp.from, &p_warp.to)?;
        let (limit_x, limit_y) = (width * EXPAND_LIMIT, height * EXPAND_LIMIT);
        let limit = (-limit_x, -limit_y, width + limit_x, height + limit_y);
        let corners = [(0.0, 0.0), (width, 0.0), (width, height), (0.0, height)]
          .map(|(x, y)| to_result.map_finite(x, y))
          .into_iter()
          .collect::<Option<Vec<_>>>();
        let (left, top, right, bottom) = match corners {
          Some(corners) => (
            corners.iter().map(|corner| corner.0).fold(f64::INFINITY, f64::min).floor().max(limit.0),
            corners.iter().map(|corner| corner.1).fold(f64::INFINITY, f64::min).floor().max(limit.1),
            corners.iter().map(|corner| corner.0).fold(f64::NEG_INFINITY, f64::max).ceil().min(limit.2),
            corners.iter().map(|corner| corner.1).fold(f64::NEG_INFINITY, f64::max).ceil().min(limit.3),
          ),
          None => limit,
        };
        window(left, top, right.max(left), bottom.max(top))
      }
    })
  }
}

/// Warps the image so the four corners of `p_from` move onto the four corners of `p_to`, bending everything
/// between them with a perspective transform. Straight lines stay straight.
///
/// For example, flattening a photographed page is a warp from the page's corners in the photo to an upright
/// rectangle. Both quads are in image pixels, with the origin at the top-left corner.
///
/// By default the canvas grows to show the whole warped image; use [`WarpImage::with_fit`] to crop instead. The
/// interpolation algorithm is chosen automatically unless set with [`WarpImage::with_algorithm`].
/// # Arguments
/// - `p_from`: Four points in the image.
/// - `p_to`: Where each of those points ends up.
pub fn warp(p_from: impl Into<Quad>, p_to: impl Into<Quad>) -> WarpImage {
  WarpImage {
    from: p_from.into(),
    to: p_to.into(),
    fit: TransformFit::Expand,
    algorithm: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn gradient_image(p_width: u32, p_height: u32) -> (Image, Vec<u8>) {
    let bytes: Vec<u8> =
      (0..p_width * p_height).flat_map(|i| [(i % 251) as u8, (i % 13 * 19) as u8, (i % 7 * 36) as u8, 255]).collect();
    (Image::new_from_pixels(p_width, p_height, &bytes, Channels::RGBA), bytes)
  }

  #[test]
  fn warping_onto_the_same_points_changes_nothing() {
    for fit in [TransformFit::Expand, TransformFit::Crop, TransformFit::Fill] {
      let (mut image, bytes) = gradient_image(40, 30);
      let frame = Quad::rect((0, 0), (40, 30));
      warp(frame, frame).with_fit(fit).with_algorithm(TransformAlgorithm::Lanczos).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), (40, 30), "{fit:?}");
      let worst = image.rgba().iter().zip(&bytes).map(|(&a, &b)| (a as i32 - b as i32).abs()).max().unwrap();
      assert!(worst <= 2, "{fit:?}: pixels drifted by {worst}");
    }
  }

  #[test]
  fn crop_is_the_size_of_the_target() {
    let (mut image, _) = gradient_image(200, 200);
    let from = Quad::new((50, 40), (150, 60), (140, 150), (60, 130));
    warp(from, Quad::rect((0, 0), (90, 70))).with_fit(TransformFit::Crop).apply(&mut image);
    assert_eq!(image.dimensions::<u32>(), (90, 70));
  }

  #[test]
  fn fill_leaves_no_transparent_pixels() {
    let (mut image, _) = gradient_image(200, 200);
    // So strong that without zooming in, the line where the warp goes to infinity would be inside the image.
    let from = Quad::new((45, 5), (55, 5), (100, 35), (0, 35));
    warp(from, from.bounds()).with_fit(TransformFit::Fill).apply(&mut image);
    assert_eq!(image.dimensions::<u32>(), (200, 200));
    assert_eq!(image.rgba().chunks(4).filter(|pixel| pixel[3] < 255).count(), 0);
  }

  #[test]
  fn expand_shows_the_whole_warped_image() {
    // Shrinking the middle of the image pulls its edges inward, so nothing is lost and the gaps are transparent.
    let (mut image, _) = gradient_image(100, 100);
    let from = Quad::rect((0, 0), (100, 100));
    let to = Quad::new((10, 0), (90, 0), (100, 100), (0, 100));
    warp(from, to).apply(&mut image);
    assert_eq!(image.dimensions::<u32>(), (100, 100));
    let pixels = image.rgba();
    assert_eq!(pixels[3], 0, "the top-left corner should be uncovered");
    assert_eq!(pixels[((90 * 100 + 50) * 4 + 3) as usize], 255, "the bottom middle should be covered");
  }

  #[test]
  fn expand_is_limited_when_the_warp_runs_off_to_infinity() {
    let (mut image, _) = gradient_image(200, 200);
    let from = Quad::new((45, 5), (55, 5), (100, 35), (0, 35));
    warp(from, from.bounds()).apply(&mut image);
    let (width, height) = image.dimensions::<u32>();
    assert!(width <= 400 && height <= 400, "{width}x{height}");
  }
}
