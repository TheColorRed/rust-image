use abra_core::{LineSegment, Orientation, PointF, TransformAlgorithm, TransformFit, rotate};

use crate::tool::Tool;

pub struct Straighten {
  line: LineSegment,
  orientation: Orientation,
  fit: TransformFit,
  algorithm: TransformAlgorithm,
}

/// Straighten an image by specifying two points on a line that should be level or plumb.
///
/// The image is rotated around its center so the line becomes horizontal or vertical, then the empty corners the
/// rotation leaves are cropped away. Both steps can be adjusted:
/// - [`Straighten::with_orientation`]: which direction to straighten to. Defaults to [`Orientation::Nearest`].
/// - [`Straighten::with_fit`]: how the canvas is sized after rotating. Defaults to [`TransformFit::Crop`].
/// - [`Straighten::with_algorithm`]: the interpolation algorithm used to rotate. Defaults to Lanczos.
///
/// A line with no length (both points the same) leaves the image unchanged.
/// # Arguments
/// - `p_start`: The starting point of the line to straighten.
/// - `p_end`: The ending point of the line to straighten.
pub fn straighten(p_start: PointF, p_end: PointF) -> Straighten {
  Straighten {
    line: LineSegment::new(p_start, p_end),
    orientation: Orientation::Nearest,
    fit: TransformFit::Crop,
    algorithm: TransformAlgorithm::Lanczos,
  }
}

impl Straighten {
  /// Sets which direction the line is straightened to. Defaults to [`Orientation::Nearest`], whichever of
  /// horizontal or vertical the line is closer to, so the image turns as little as possible. Use
  /// [`Orientation::Angle`] to turn the line to any other angle.
  pub fn with_orientation(mut self, p_orientation: Orientation) -> Self {
    self.orientation = p_orientation;
    self
  }

  /// Sets how the canvas is sized after rotating. Defaults to [`TransformFit::Crop`], the largest area without empty
  /// corners, which is smaller than the original.
  /// - [`TransformFit::Fill`] crops to the original aspect ratio and scales back up to the original size.
  /// - [`TransformFit::Expand`] keeps the whole rotated image, on a larger canvas with transparent corners.
  pub fn with_fit(mut self, p_fit: TransformFit) -> Self {
    self.fit = p_fit;
    self
  }

  /// Sets the interpolation algorithm used to rotate the image. Defaults to `TransformAlgorithm::Lanczos`,
  /// the highest quality and the slowest.
  pub fn with_algorithm(mut self, p_algorithm: TransformAlgorithm) -> Self {
    self.algorithm = p_algorithm;
    self
  }
}

impl Tool for Straighten {
  fn apply<'a>(&self, p_image: impl Into<abra_core::ImageRef<'a>>) {
    // A line with no length has no direction to straighten to.
    if self.line.is_zero_length() {
      return;
    }

    let mut image = p_image.into();
    rotate(self.line.correction_to(self.orientation))
      .with_fit(self.fit)
      .with_algorithm(self.algorithm)
      .apply(&mut image);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;

  fn solid_image(p_width: u32, p_height: u32) -> abra_core::Image {
    abra_core::Image::new_from_pixels(
      p_width,
      p_height,
      &[200, 30, 30, 255].repeat((p_width * p_height) as usize),
      Channels::RGBA,
    )
  }

  /// Straightens a solid 200x100 image and returns its size and how many pixels are visibly transparent.
  fn straighten_solid() -> ((u32, u32), usize) {
    let mut image = solid_image(200, 100);
    straighten(PointF::new(0, 0), PointF::new(100, 10)).apply(&mut image);
    let size: (u32, u32) = image.dimensions();
    // Resampling can leave a few edge pixels a touch under fully opaque; only count ones that visibly show through.
    (size, image.rgba().chunks(4).filter(|pixel| pixel[3] < 240).count())
  }

  #[test]
  fn cropped_result_has_no_transparent_pixels() {
    let (size, transparent) = straighten_solid();
    assert!(size.0 < 200 && size.1 < 100, "expected a smaller image, got {size:?}");
    assert_eq!(transparent, 0, "{transparent} transparent pixels in {size:?}");
  }

  const BACKGROUND: [u8; 4] = [128, 128, 128, 255];
  const RED: [u8; 4] = [255, 0, 0, 255];
  const BLUE: [u8; 4] = [0, 0, 255, 255];

  /// A gray 200x100 image with a 5x5 red square centered on `p_start` and a blue one on `p_end`.
  fn image_with_markers(p_start: (u32, u32), p_end: (u32, u32)) -> abra_core::Image {
    let (width, height) = (200u32, 100u32);
    let mut bytes = BACKGROUND.repeat((width * height) as usize);
    for (center, color) in [(p_start, RED), (p_end, BLUE)] {
      for y in center.1 - 2..=center.1 + 2 {
        for x in center.0 - 2..=center.0 + 2 {
          let index = ((y * width + x) * 4) as usize;
          bytes[index..index + 4].copy_from_slice(&color);
        }
      }
    }
    abra_core::Image::new_from_pixels(width, height, &bytes, Channels::RGBA)
  }

  /// The center of the pixels that look like `p_color`, or `None` if there are none.
  fn find_marker(p_image: &abra_core::Image, p_color: [u8; 4]) -> Option<(f32, f32)> {
    let (width, _): (u32, u32) = p_image.dimensions();
    let (mut sum_x, mut sum_y, mut count) = (0.0, 0.0, 0.0);
    for (index, pixel) in p_image.rgba().chunks(4).enumerate() {
      // Resampling blends colors a little, so match by which channels dominate rather than exact values.
      let close = pixel.iter().zip(p_color).take(3).all(|(&a, b)| (a as i32 - b as i32).abs() < 90) && pixel[3] > 200;
      if close {
        sum_x += (index as u32 % width) as f32;
        sum_y += (index as u32 / width) as f32;
        count += 1.0;
      }
    }
    (count > 0.0).then(|| (sum_x / count, sum_y / count))
  }

  fn markers_after(p_tool: Straighten, p_start: (u32, u32), p_end: (u32, u32)) -> ((f32, f32), (f32, f32)) {
    let mut image = image_with_markers(p_start, p_end);
    p_tool.apply(&mut image);
    (find_marker(&image, RED).expect("red marker lost"), find_marker(&image, BLUE).expect("blue marker lost"))
  }

  #[test]
  fn the_line_ends_up_level() {
    let (start, end) = ((40, 30), (160, 50));
    let tool = straighten(PointF::new(start.0, start.1), PointF::new(end.0, end.1)).with_fit(TransformFit::Expand);
    let (red, blue) = markers_after(tool, start, end);
    assert!((red.1 - blue.1).abs() < 1.0, "not level: red {red:?} blue {blue:?}");
    assert!(blue.0 > red.0 + 100.0, "the image was flipped: red {red:?} blue {blue:?}");
  }

  #[test]
  fn the_line_ends_up_plumb() {
    // Nearly vertical, so `Nearest` makes it vertical.
    let (start, end) = ((100, 20), (110, 80));
    let tool = straighten(PointF::new(start.0, start.1), PointF::new(end.0, end.1)).with_fit(TransformFit::Expand);
    let (red, blue) = markers_after(tool, start, end);
    assert!((red.0 - blue.0).abs() < 1.0, "not plumb: red {red:?} blue {blue:?}");
    assert!(blue.1 > red.1 + 40.0, "the image was flipped: red {red:?} blue {blue:?}");
  }

  #[test]
  fn a_forced_axis_overrides_the_closer_one() {
    // The same nearly vertical line, forced to horizontal.
    let (start, end) = ((100, 20), (110, 80));
    let tool = straighten(PointF::new(start.0, start.1), PointF::new(end.0, end.1))
      .with_orientation(Orientation::Horizontal)
      .with_fit(TransformFit::Expand);
    let (red, blue) = markers_after(tool, start, end);
    assert!((red.1 - blue.1).abs() < 1.0, "not level: red {red:?} blue {blue:?}");
    assert!((red.0 - blue.0).abs() > 40.0, "still vertical: red {red:?} blue {blue:?}");
  }

  #[test]
  fn a_line_with_no_length_leaves_the_image_alone() {
    let mut image = image_with_markers((40, 30), (160, 50));
    let before = image.rgba().to_vec();
    straighten(PointF::new(50, 50), PointF::new(50, 50)).apply(&mut image);
    assert_eq!(image.dimensions::<u32>(), (200, 100));
    assert_eq!(image.rgba(), before.as_slice());
  }

  #[test]
  fn fill_restores_the_original_size() {
    let mut image = solid_image(200, 100);
    straighten(PointF::new(0, 0), PointF::new(100, 10)).with_fit(TransformFit::Fill).apply(&mut image);
    assert_eq!(image.dimensions::<u32>(), (200, 100));
    assert_eq!(image.rgba().chunks(4).filter(|pixel| pixel[3] < 240).count(), 0);
  }

  #[test]
  fn expand_keeps_the_whole_rotated_image() {
    let mut image = solid_image(200, 100);
    straighten(PointF::new(0, 0), PointF::new(100, 10)).with_fit(TransformFit::Expand).apply(&mut image);
    let (width, height) = image.dimensions::<u32>();
    assert!(width > 200 && height > 100, "expected a larger canvas, got {width}x{height}");
  }
}
