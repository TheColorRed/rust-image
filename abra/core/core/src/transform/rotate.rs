use crate::{Channels, Image, IntoNumber, Size};

use super::{
  TransformAlgorithm, TransformFit,
  interpolation::{EdgeMode, Interpolation, remap},
  resize::get_resize_algorithm,
};

/// The part of the rotated canvas that becomes the output, and the size it is drawn at.
struct RotateWindow {
  /// The size of the canvas the whole rotated image fits on. The image is rotated around its center.
  canvas_width: u32,
  canvas_height: u32,
  /// The area of the canvas to keep, in canvas pixels.
  left: f64,
  top: f64,
  width: f64,
  height: f64,
  /// The size of the output image. It differs from the kept area only for [`TransformFit::Fill`].
  output_width: u32,
  output_height: u32,
}

impl RotateWindow {
  fn new(p_width: u32, p_height: u32, p_degrees: f32, p_fit: TransformFit) -> RotateWindow {
    let (canvas_width, canvas_height) = Size::new(p_width, p_height).rotated_bounds(p_degrees).to_tuple::<u32>();
    let (canvas_w, canvas_h) = (canvas_width as f64, canvas_height as f64);
    let (center_x, center_y) = (canvas_w / 2.0, canvas_h / 2.0);
    let source = Size::new(p_width, p_height);

    // Resampling a rotated edge also blends in the empty area beside it, so the kept area stays one more pixel
    // away from any edge that is not axis-aligned.
    let (sin, cos) = (p_degrees as f64).to_radians().sin_cos();
    let margin = if (sin * cos).abs() > 1e-4 { 1.0 } else { 0.0 };

    let window = |p_left: f64, p_top: f64, p_width: f64, p_height: f64, p_output: (u32, u32)| RotateWindow {
      canvas_width,
      canvas_height,
      left: p_left,
      top: p_top,
      width: p_width,
      height: p_height,
      output_width: p_output.0.max(1),
      output_height: p_output.1.max(1),
    };

    match p_fit {
      TransformFit::Expand => window(0.0, 0.0, canvas_w, canvas_h, (canvas_width, canvas_height)),
      TransformFit::Crop => {
        let inscribed = source.inscribed_after_rotation(p_degrees, None);
        let (half_width, half_height) = (inscribed.width as f64 / 2.0, inscribed.height as f64 / 2.0);
        // Round inward to whole pixels so partially covered edge pixels are not kept. The tolerance stops float
        // error, like 49.9999 for 50, from costing a whole pixel.
        const EPSILON: f64 = 1e-3;
        let left = (center_x - half_width - EPSILON).ceil().max(0.0) + margin;
        let top = (center_y - half_height - EPSILON).ceil().max(0.0) + margin;
        let right = (center_x + half_width + EPSILON).floor().min(canvas_w) - margin;
        let bottom = (center_y + half_height + EPSILON).floor().min(canvas_h) - margin;
        let (width, height) = ((right - left).max(1.0), (bottom - top).max(1.0));
        window(left, top, width, height, (width as u32, height as u32))
      }
      TransformFit::Fill => {
        // The output is resampled from the kept area anyway, so it does not need whole pixels. Shrinking it evenly
        // for the margin keeps the aspect ratio exact, so the image is never stretched.
        let inscribed = source.inscribed_after_rotation(p_degrees, source.aspect_ratio());
        let (width, height) = (inscribed.width as f64, inscribed.height as f64);
        let shrink = (1.0 - 2.0 * margin / width.min(height)).max(0.0);
        let (width, height) = (width * shrink, height * shrink);
        window(center_x - width / 2.0, center_y - height / 2.0, width, height, (p_width, p_height))
      }
    }
  }
}

/// Applies the rotation to the image by sampling, for each output pixel, the source position that rotates onto it.
/// Only the part of the rotated canvas inside `p_window` is drawn, scaled to the window's output size.
/// * `p_image` - The image to rotate.
/// * `p_degrees` - The degrees to rotate the image.
/// * `p_window` - The area of the rotated canvas to keep and the size to draw it at.
/// * `p_interpolation` - The interpolation used while sampling source pixels.
fn apply_rotation(p_image: &mut Image, p_degrees: f32, p_window: &RotateWindow, p_interpolation: Interpolation) {
  let (source_width, source_height) = p_image.dimensions::<u32>();
  let (source_center_x, source_center_y) = (source_width as f64 / 2.0, source_height as f64 / 2.0);
  let (canvas_center_x, canvas_center_y) = (p_window.canvas_width as f64 / 2.0, p_window.canvas_height as f64 / 2.0);
  let scale_x = p_window.width / p_window.output_width as f64;
  let scale_y = p_window.height / p_window.output_height as f64;
  let (sin, cos) = (p_degrees as f64).to_radians().sin_cos();

  let pixels =
    remap(p_image, p_window.output_width, p_window.output_height, p_interpolation, EdgeMode::Transparent, |x, y| {
      // The output pixel's position on the rotated canvas, relative to the canvas center, turned back around the
      // image center into the source.
      let dx = p_window.left + x * scale_x - canvas_center_x;
      let dy = p_window.top + y * scale_y - canvas_center_y;
      Some((dx * cos + dy * sin + source_center_x, -dx * sin + dy * cos + source_center_y))
    });
  p_image.set_pixels(p_window.output_width, p_window.output_height, pixels, Channels::RGBA);
}

/// A rotation that has been described but not yet run. Create one with [`rotate`], optionally configure it with
/// [`RotateImage::with_fit`] and [`RotateImage::with_algorithm`], then run it with [`RotateImage::apply`].
pub struct RotateImage {
  pub degrees: f32,
  pub fit: TransformFit,
  pub algorithm: Option<TransformAlgorithm>,
}

impl RotateImage {
  /// Sets how the canvas is sized after rotating. Defaults to [`TransformFit::Expand`].
  pub fn with_fit(mut self, p_fit: TransformFit) -> Self {
    self.fit = p_fit;
    self
  }

  /// Sets the interpolation algorithm. When `None` (the default), the best algorithm is selected automatically.
  pub fn with_algorithm(mut self, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> Self {
    self.algorithm = p_algorithm.into();
    self
  }

  /// Rotates the image in place.
  pub fn apply(&self, p_image: &mut Image) {
    let (width, height) = p_image.dimensions::<u32>();
    let window = RotateWindow::new(width, height, self.degrees, self.fit);
    // The kept area is what gets resampled, so the automatic algorithm is picked from how much it is scaled.
    let algorithm = get_resize_algorithm(
      self.algorithm,
      window.width as u32,
      window.height as u32,
      window.output_width,
      window.output_height,
    );
    apply_rotation(p_image, self.degrees, &window, algorithm.interpolation());
  }
}

/// Rotates the image around its center by the given number of degrees. By default the canvas grows to fit the
/// rotated image; use [`RotateImage::with_fit`] to crop away the transparent corners instead.
/// # Arguments
/// - `p_degrees`: The number of degrees to rotate the image. Positive values rotate clockwise, negative values rotate counter-clockwise.
///
/// The interpolation algorithm is chosen automatically unless set with [`RotateImage::with_algorithm`].
pub fn rotate(p_degrees: impl IntoNumber) -> RotateImage {
  RotateImage {
    degrees: p_degrees.into::<f32>(),
    fit: TransformFit::Expand,
    algorithm: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::crop;

  const FITS: [TransformFit; 3] = [TransformFit::Expand, TransformFit::Crop, TransformFit::Fill];

  fn solid_image(p_width: u32, p_height: u32) -> Image {
    Image::new_from_pixels(p_width, p_height, &[200, 30, 30, 255].repeat((p_width * p_height) as usize), Channels::RGBA)
  }

  /// How many pixels are visibly transparent. Resampling can leave a few edge pixels a touch under fully opaque,
  /// so only ones that visibly show through count.
  fn transparent_pixels(p_image: &Image) -> usize {
    p_image.rgba().chunks(4).filter(|pixel| pixel[3] < 240).count()
  }

  #[test]
  fn rotating_by_nothing_changes_nothing() {
    let bytes: Vec<u8> = (0..16 * 12).flat_map(|i| [(i % 16 * 16) as u8, (i / 16 * 20) as u8, 128, 255]).collect();
    for algorithm in [
      TransformAlgorithm::NearestNeighbor,
      TransformAlgorithm::Bilinear,
      TransformAlgorithm::Bicubic,
      TransformAlgorithm::Lanczos,
      TransformAlgorithm::Auto,
    ] {
      for fit in FITS {
        let mut image = Image::new_from_pixels(16, 12, &bytes, Channels::RGBA);
        rotate(0.0).with_fit(fit).with_algorithm(algorithm).apply(&mut image);
        assert_eq!(image.dimensions::<u32>(), (16, 12), "{algorithm} {fit:?}");
        assert_eq!(image.rgba(), bytes.as_slice(), "{algorithm} {fit:?}");
      }
    }
  }

  #[test]
  fn a_quarter_turn_swaps_the_width_and_height() {
    for algorithm in [
      TransformAlgorithm::NearestNeighbor,
      TransformAlgorithm::Bilinear,
      TransformAlgorithm::Lanczos,
    ] {
      let mut image = Image::new_from_pixels(6, 4, &[9, 9, 9, 255].repeat(24), Channels::RGBA);
      rotate(90.0).with_algorithm(algorithm).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), (4, 6), "{algorithm}");
    }
  }

  #[test]
  fn a_quarter_turn_crop_keeps_the_whole_rotated_image() {
    let bytes: Vec<u8> = (0..6 * 4).flat_map(|i| [(i * 10) as u8, 0, 0, 255]).collect();
    let mut expanded = Image::new_from_pixels(6, 4, &bytes, Channels::RGBA);
    rotate(90.0).with_algorithm(TransformAlgorithm::NearestNeighbor).apply(&mut expanded);
    let mut cropped = Image::new_from_pixels(6, 4, &bytes, Channels::RGBA);
    rotate(90.0).with_fit(TransformFit::Crop).with_algorithm(TransformAlgorithm::NearestNeighbor).apply(&mut cropped);
    assert_eq!(cropped.dimensions::<u32>(), (4, 6));
    assert_eq!(cropped.rgba(), expanded.rgba());
  }

  #[test]
  fn crop_matches_expanding_then_cropping() {
    // Drawing only the kept window must give the same pixels as drawing the whole canvas and cutting it out.
    let bytes: Vec<u8> = (0..40 * 30).flat_map(|i| [(i % 40 * 6) as u8, (i / 40 * 8) as u8, 90, 255]).collect();
    for degrees in [7.0f32, -20.0, 33.0] {
      let window = RotateWindow::new(40, 30, degrees, TransformFit::Crop);
      let mut expected = Image::new_from_pixels(40, 30, &bytes, Channels::RGBA);
      rotate(degrees).with_algorithm(TransformAlgorithm::Bilinear).apply(&mut expected);
      crop(window.left, window.top, window.output_width, window.output_height).apply(&mut expected);

      let mut image = Image::new_from_pixels(40, 30, &bytes, Channels::RGBA);
      rotate(degrees).with_fit(TransformFit::Crop).with_algorithm(TransformAlgorithm::Bilinear).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), expected.dimensions::<u32>(), "{degrees}");
      assert_eq!(image.rgba(), expected.rgba(), "{degrees}");
    }
  }

  #[test]
  fn crop_and_fill_leave_no_transparent_corners() {
    for degrees in [5.7f32, -12.0, 30.0, 45.0, 100.0] {
      let mut expanded = solid_image(200, 100);
      rotate(degrees).apply(&mut expanded);
      assert!(transparent_pixels(&expanded) > 0, "{degrees}: expected transparent corners");

      let mut cropped = solid_image(200, 100);
      rotate(degrees).with_fit(TransformFit::Crop).with_algorithm(TransformAlgorithm::Lanczos).apply(&mut cropped);
      let (width, height) = cropped.dimensions::<u32>();
      assert!(width < 200 && height < 200, "{degrees}: expected a smaller image, got {width}x{height}");
      assert_eq!(transparent_pixels(&cropped), 0, "{degrees}: crop {width}x{height}");

      let mut filled = solid_image(200, 100);
      rotate(degrees).with_fit(TransformFit::Fill).with_algorithm(TransformAlgorithm::Lanczos).apply(&mut filled);
      assert_eq!(filled.dimensions::<u32>(), (200, 100), "{degrees}");
      assert_eq!(transparent_pixels(&filled), 0, "{degrees}: fill");
    }
  }

  #[test]
  fn fill_keeps_the_center_in_place() {
    // A dark dot at the center stays at the center after filling, since the image turns around it.
    let (width, height) = (120u32, 80u32);
    let mut bytes = [255u8, 255, 255, 255].repeat((width * height) as usize);
    for y in 38..42 {
      for x in 58..62 {
        let index = ((y * width + x) * 4) as usize;
        bytes[index..index + 3].copy_from_slice(&[0, 0, 0]);
      }
    }
    let mut image = Image::new_from_pixels(width, height, &bytes, Channels::RGBA);
    rotate(15.0).with_fit(TransformFit::Fill).with_algorithm(TransformAlgorithm::Bilinear).apply(&mut image);
    let (mut sum_x, mut sum_y, mut count) = (0.0, 0.0, 0.0);
    for (index, pixel) in image.rgba().chunks(4).enumerate() {
      if pixel[0] < 128 {
        sum_x += (index as u32 % width) as f32;
        sum_y += (index as u32 / width) as f32;
        count += 1.0;
      }
    }
    assert!(count > 0.0, "the dot was lost");
    let (center_x, center_y) = (sum_x / count, sum_y / count);
    assert!((center_x - 59.5).abs() < 1.0 && (center_y - 39.5).abs() < 1.0, "dot at {center_x}, {center_y}");
  }
}
