use abra_core::{LineSegment, Orientation, PointF, ResizeTarget, Size, TransformAlgorithm, crop, resize, rotate};

use crate::tool::Tool;

/// The direction the line is straightened to.
pub enum Axis {
  /// Make the line horizontal.
  Horizontal,
  /// Make the line vertical.
  Vertical,
  /// Whichever of horizontal or vertical the line is closer to, so the image turns as little as possible. Default.
  Auto,
}

pub struct Straighten {
  start: PointF,
  end: PointF,
  axis: Axis,
  crop: bool,
  resize: bool,
  algorithm: TransformAlgorithm,
}

/// Straighten an image by specifying two points on a line that should be level or plumb.
///
/// The image is rotated around its center so the line becomes horizontal or vertical, then the empty corners the
/// rotation leaves are cropped away. Both steps can be adjusted:
/// - [`Straighten::with_axis`]: which direction to straighten to. Defaults to [`Axis::Auto`].
/// - [`Straighten::with_crop`]: whether to crop away the empty corners. Defaults to `true`.
/// - [`Straighten::with_resize`]: whether to scale the cropped image back to the original size. Defaults to `false`.
/// - [`Straighten::with_algorithm`]: the interpolation algorithm used to rotate. Defaults to Lanczos.
///
/// A line with no length (both points the same) leaves the image unchanged.
/// # Arguments
/// - `p_start`: The starting point of the line to straighten.
/// - `p_end`: The ending point of the line to straighten.
pub fn straighten(p_start: PointF, p_end: PointF) -> Straighten {
  Straighten {
    start: p_start,
    end: p_end,
    axis: Axis::Auto,
    crop: true,
    resize: false,
    algorithm: TransformAlgorithm::Lanczos,
  }
}

impl Straighten {
  /// Sets which direction the line is straightened to. Defaults to [`Axis::Auto`].
  pub fn with_axis(mut self, p_axis: Axis) -> Self {
    self.axis = p_axis;
    self
  }

  /// Sets whether to crop away the empty corners the rotation leaves. Defaults to `true`.
  ///
  /// When `false` the whole rotated image is kept, on a larger canvas with transparent corners.
  pub fn with_crop(mut self, p_crop: bool) -> Self {
    self.crop = p_crop;
    self
  }

  /// Sets whether to scale the cropped image back to the size of the original. Defaults to `false`, so the
  /// result is the largest area without empty corners, which is smaller than the original.
  ///
  /// This only applies when cropping. Without a crop the canvas has a different shape from the original, and
  /// scaling it to the original size would stretch the image.
  pub fn with_resize(mut self, p_resize: bool) -> Self {
    self.resize = p_resize;
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
    if self.start == self.end {
      return;
    }

    let mut image = p_image.into();
    let (width, height): (u64, u64) = image.dimensions();

    let axis = match self.axis {
      Axis::Horizontal => Orientation::Horizontal,
      Axis::Vertical => Orientation::Vertical,
      Axis::Auto => {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        if dx.abs() > dy.abs() { Orientation::Horizontal } else { Orientation::Vertical }
      }
    };

    // Take the current start/end and calculate the number of degrees to make the y-axis aligned
    let angle = LineSegment::new(self.start, self.end).correction_to(axis);

    // Rotate the image around its center by the calculated angle.
    rotate(angle).with_algorithm(self.algorithm).apply(&mut image);

    // Rotating grows the canvas and leaves transparent corners; crop them away.
    let (canvas_width, canvas_height): (u32, u32) = image.dimensions();
    let size = Size::new(width, height);
    let canvas_size = Size::new(canvas_width, canvas_height);

    if self.crop {
      // Scaling back to the original size needs the original aspect ratio, or the image would be stretched.
      // Without it, the largest area of any shape is kept.
      let (x, y, crop_width, crop_height) = straightened_area(size, canvas_size, angle as f32, self.resize);
      crop(x, y, crop_width, crop_height).apply(&mut image);

      // Scale back up so the result is the same size as the original image.
      if self.resize {
        resize(ResizeTarget::Exact((width as u32, height as u32).into()))
          .with_algorithm(self.algorithm)
          .apply(&mut image);
      }
    }
  }
}

/// The area to keep after rotating an image, as `(x, y, width, height)` within the rotated canvas.
///
/// This is a rectangle that fits entirely inside the rotated image, so no transparent corners remain. It is
/// centered on the canvas, where rotating around the image center puts the original image's center.
/// - `p_source`: The size of the image before rotating.
/// - `p_canvas`: The size of the canvas after rotating.
/// - `p_degrees`: The rotation that was applied, clockwise.
/// - `p_keep_aspect`: Keep the original aspect ratio. Otherwise the rectangle with the largest area is used.
fn straightened_area(p_source: Size, p_canvas: Size, p_degrees: f32, p_keep_aspect: bool) -> (u32, u32, u32, u32) {
  let (source_width, source_height) = (p_source.width as f64, p_source.height as f64);
  let (canvas_width, canvas_height) = (p_canvas.width as f64, p_canvas.height as f64);
  let (sin, cos) = (p_degrees as f64).to_radians().sin_cos();
  let (sin, cos) = (sin.abs(), cos.abs());

  let (crop_width, crop_height) = if p_keep_aspect {
    // A centered rectangle of size (a, b) fits inside the rotated image when both
    // a*|cos| + b*|sin| <= width and a*|sin| + b*|cos| <= height. Scale the original size until they hold.
    let scale = (source_width / (source_width * cos + source_height * sin))
      .min(source_height / (source_width * sin + source_height * cos))
      .min(1.0);
    (source_width * scale, source_height * scale)
  } else {
    largest_inscribed_rectangle(source_width, source_height, sin, cos)
  };

  let (center_x, center_y) = (canvas_width / 2.0, canvas_height / 2.0);

  // Round inward so partially covered edge pixels are not kept. Resampling a rotated edge also blends in the
  // empty area beside it, so stay one more pixel away from any edge that is not axis-aligned.
  let margin = if (sin * cos).abs() > 1e-4 { 1.0 } else { 0.0 };
  // The tolerance stops float error, like 49.999999999999994 for 50, from costing a whole pixel.
  const EPSILON: f64 = 1e-6;
  let left = (center_x - crop_width / 2.0 - EPSILON).ceil().max(0.0) + margin;
  let top = (center_y - crop_height / 2.0 - EPSILON).ceil().max(0.0) + margin;
  let right = (center_x + crop_width / 2.0 + EPSILON).floor().min(canvas_width) - margin;
  let bottom = (center_y + crop_height / 2.0 + EPSILON).floor().min(canvas_height) - margin;

  (left as u32, top as u32, (right - left).max(1.0) as u32, (bottom - top).max(1.0) as u32)
}

/// The size of the largest axis-aligned rectangle, of any aspect ratio, that fits centered inside a
/// `p_width` x `p_height` rectangle rotated by an angle with the given absolute sine and cosine.
fn largest_inscribed_rectangle(p_width: f64, p_height: f64, p_sin: f64, p_cos: f64) -> (f64, f64) {
  let width_is_longer = p_width >= p_height;
  let (long_side, short_side) = if width_is_longer { (p_width, p_height) } else { (p_height, p_width) };

  if short_side <= 2.0 * p_sin * p_cos * long_side || (p_sin - p_cos).abs() < 1e-9 {
    // Half constrained: two corners of the rectangle touch the longer side of the rotated image, and the other
    // two sit on the line down its middle.
    let half = 0.5 * short_side;
    if width_is_longer { (half / p_sin, half / p_cos) } else { (half / p_cos, half / p_sin) }
  } else {
    // Fully constrained: the rectangle touches all four sides of the rotated image.
    let cos_2a = p_cos * p_cos - p_sin * p_sin;
    ((p_width * p_cos - p_height * p_sin) / cos_2a, (p_height * p_cos - p_width * p_sin) / cos_2a)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_rotation_keeps_the_whole_image() {
    for keep_aspect in [true, false] {
      assert_eq!(straightened_area(Size::new(100, 50), Size::new(100, 50), 0.0, keep_aspect), (0, 0, 100, 50));
    }
  }

  #[test]
  fn crop_stays_inside_the_rotated_image() {
    // Canvas sizes are what `rotate` produces for a center rotation: the bounding box of the rotated image.
    for (source, degrees) in [
      (Size::new(100u32, 50u32), 10.0f32),
      (Size::new(100, 50), -10.0),
      (Size::new(50, 100), 25.0),
      (Size::new(200, 200), 45.0),
    ] {
      let radians = degrees.to_radians();
      let (sin, cos) = (radians.sin().abs(), radians.cos().abs());
      let canvas =
        Size::new((source.width * cos + source.height * sin) as u32, (source.width * sin + source.height * cos) as u32);
      let (x, y, w, h) = straightened_area(source, canvas, degrees, true);

      // Rotate each crop corner back into the source frame; it must land inside the source image.
      let (sin, cos) = radians.sin_cos();
      let (cx, cy) = (canvas.width / 2.0, canvas.height / 2.0);
      for (px, py) in [(x, y), (x + w, y), (x, y + h), (x + w, y + h)] {
        let (dx, dy) = (px as f32 - cx, py as f32 - cy);
        let back_x = dx * cos + dy * sin;
        let back_y = -dx * sin + dy * cos;
        assert!(back_x.abs() <= source.width / 2.0 + 0.01, "{source:?} {degrees}: x {back_x}");
        assert!(back_y.abs() <= source.height / 2.0 + 0.01, "{source:?} {degrees}: y {back_y}");
      }
      // And it should be centered and keep the original aspect ratio (within rounding).
      assert!((x as f32 + w as f32 / 2.0 - cx).abs() <= 1.0);
      assert!((y as f32 + h as f32 / 2.0 - cy).abs() <= 1.0);
      // Each edge can lose up to 2px to rounding plus 1px of margin, so the two scale factors may differ by that much per source pixel.
      let (scale_x, scale_y) = (w as f32 / source.width, h as f32 / source.height);
      let tolerance = 6.0 / source.width.min(source.height) as f32;
      assert!((scale_x - scale_y).abs() <= tolerance, "{source:?} {degrees}: {w}x{h}");
    }
  }

  fn solid_image(p_width: u32, p_height: u32) -> abra_core::Image {
    abra_core::Image::from_rgba_bytes(p_width, p_height, &[200, 30, 30, 255].repeat((p_width * p_height) as usize))
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
    abra_core::Image::from_rgba_bytes(width, height, &bytes)
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
    let tool = straighten(PointF::new(start.0, start.1), PointF::new(end.0, end.1)).with_crop(false);
    let (red, blue) = markers_after(tool, start, end);
    assert!((red.1 - blue.1).abs() < 1.0, "not level: red {red:?} blue {blue:?}");
    assert!(blue.0 > red.0 + 100.0, "the image was flipped: red {red:?} blue {blue:?}");
  }

  #[test]
  fn the_line_ends_up_plumb() {
    // Nearly vertical, so `Auto` makes it vertical.
    let (start, end) = ((100, 20), (110, 80));
    let tool = straighten(PointF::new(start.0, start.1), PointF::new(end.0, end.1)).with_crop(false);
    let (red, blue) = markers_after(tool, start, end);
    assert!((red.0 - blue.0).abs() < 1.0, "not plumb: red {red:?} blue {blue:?}");
    assert!(blue.1 > red.1 + 40.0, "the image was flipped: red {red:?} blue {blue:?}");
  }

  #[test]
  fn a_forced_axis_overrides_the_closer_one() {
    // The same nearly vertical line, forced to horizontal.
    let (start, end) = ((100, 20), (110, 80));
    let tool =
      straighten(PointF::new(start.0, start.1), PointF::new(end.0, end.1)).with_axis(Axis::Horizontal).with_crop(false);
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
  fn largest_area_crop_stays_inside_and_beats_the_aspect_locked_one() {
    for (source, degrees) in [
      (Size::new(100u32, 50u32), 10.0f32),
      (Size::new(100, 50), -10.0),
      (Size::new(50, 100), 25.0),
      (Size::new(300, 100), 30.0),
      (Size::new(200, 200), 45.0),
      (Size::new(100, 50), 90.0),
    ] {
      let radians = degrees.to_radians();
      let (abs_sin, abs_cos) = (radians.sin().abs(), radians.cos().abs());
      let canvas = Size::new(
        (source.width * abs_cos + source.height * abs_sin) as u32,
        (source.width * abs_sin + source.height * abs_cos) as u32,
      );
      let (x, y, w, h) = straightened_area(source, canvas, degrees, false);

      // Every corner of the crop, rotated back into the source frame, must land inside the source image.
      let (sin, cos) = radians.sin_cos();
      let (cx, cy) = (canvas.width / 2.0, canvas.height / 2.0);
      for (px, py) in [(x, y), (x + w, y), (x, y + h), (x + w, y + h)] {
        let (dx, dy) = (px as f32 - cx, py as f32 - cy);
        let back_x = dx * cos + dy * sin;
        let back_y = -dx * sin + dy * cos;
        // A pixel of slack covers the canvas size being truncated to whole pixels.
        assert!(back_x.abs() <= source.width / 2.0 + 1.0, "{source:?} {degrees}: x {back_x}");
        assert!(back_y.abs() <= source.height / 2.0 + 1.0, "{source:?} {degrees}: y {back_y}");
      }

      // It keeps at least as much as the aspect-locked crop, apart from a few pixels of rounding.
      let (_, _, locked_w, locked_h) = straightened_area(source, canvas, degrees, true);
      let slack = (w + h) as f32 * 2.0;
      assert!(
        (w * h) as f32 + slack >= (locked_w * locked_h) as f32,
        "{source:?} {degrees}: {w}x{h} vs {locked_w}x{locked_h}"
      );
    }
  }

  #[test]
  fn a_quarter_turn_keeps_the_whole_rotated_image() {
    assert_eq!(straightened_area(Size::new(100, 50), Size::new(50, 100), 90.0, false), (0, 0, 50, 100));
  }

  #[test]
  fn resizing_restores_the_original_size_after_cropping() {
    let mut image = solid_image(200, 100);
    straighten(PointF::new(0, 0), PointF::new(100, 10)).with_resize(true).apply(&mut image);
    assert_eq!(image.dimensions::<u32>(), (200, 100));
  }

  #[test]
  fn resizing_is_ignored_without_a_crop() {
    // Scaling an uncropped canvas to the original size would stretch it, so it is left at the rotated size.
    let mut plain = solid_image(200, 100);
    straighten(PointF::new(0, 0), PointF::new(100, 10)).with_crop(false).apply(&mut plain);
    let mut resized = solid_image(200, 100);
    straighten(PointF::new(0, 0), PointF::new(100, 10)).with_crop(false).with_resize(true).apply(&mut resized);
    assert_eq!(plain.dimensions::<u32>(), resized.dimensions::<u32>());
    assert!(plain.dimensions::<u32>().0 > 200, "expected a larger canvas");
  }
}
