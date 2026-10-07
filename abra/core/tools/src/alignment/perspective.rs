use abra_core::{Area, Quad, TransformAlgorithm, TransformFit, warp};

use crate::Tool;

pub struct Perspective {
  area: Area,
  algorithm: TransformAlgorithm,
  fit: TransformFit,
}

impl Perspective {
  /// Sets the interpolation algorithm used to resample the image. Defaults to `TransformAlgorithm::Lanczos`,
  /// the highest quality and the slowest.
  ///
  /// The edge-directed resize algorithms have no meaning here and use Lanczos.
  pub fn with_algorithm(mut self, p_algorithm: TransformAlgorithm) -> Self {
    self.algorithm = p_algorithm;
    self
  }

  /// Sets how the result is sized. Defaults to [`TransformFit::Crop`].
  /// - [`TransformFit::Crop`]: only the flattened area is kept, at the size of the area.
  /// - [`TransformFit::Fill`]: the result stays the same size as the image and the area is stretched out in place to
  ///   its bounding box, zoomed in if needed so every pixel comes from the source: nothing is stretched from the
  ///   edges and no transparent gaps open up.
  /// - [`TransformFit::Expand`]: like `Fill`, but instead of zooming in, the canvas grows to show the whole corrected
  ///   image, with transparent areas where the source does not reach.
  pub fn with_fit(mut self, p_fit: TransformFit) -> Self {
    self.fit = p_fit;
    self
  }
}

impl Tool for Perspective {
  fn apply<'a>(&self, p_image: impl Into<abra_core::ImageRef<'a>>) {
    let Some(quad) = self.area.to_quad() else {
      return;
    };

    // Cropping flattens the area onto an upright rectangle shaped like the one in the photo. Otherwise the area is
    // stretched out in place over its bounding box: each corner only moves outwards, so the top lines up with the
    // higher top corner, the bottom with the lower bottom corner, and each side with the corner that sticks out
    // further.
    let target = match self.fit {
      TransformFit::Crop => Quad::rect((0, 0), quad.flattened_size()),
      TransformFit::Fill | TransformFit::Expand => quad.bounds().into(),
    };

    let mut image = p_image.into();
    warp(quad, target).with_fit(self.fit).with_algorithm(self.algorithm).apply(&mut image);
  }
}

/// Correct the perspective of an image, using an area that should be a rectangle but was photographed at an angle,
/// such as a building, a sign or a page.
///
/// The image is warped so the four corners of the area become the corners of a rectangle, sized by
/// [`Quad::flattened_size`] so the far end is not squashed. By default the result is cropped to that rectangle; see
/// [`Perspective::with_fit`] for keeping the whole image instead.
///
/// The area must be a polygon with exactly four corners that form a convex shape, like
/// `Area::from_points(&[[x, y], ...])`. The corners can be given in any order. Anything else, such as a
/// circle or a shape with curves, leaves the image unchanged.
pub fn perspective(p_area: impl Into<Area>) -> Perspective {
  Perspective {
    area: p_area.into(),
    algorithm: TransformAlgorithm::Lanczos,
    fit: TransformFit::Crop,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Channels, Homography, Image};

  const GRAY: [u8; 4] = [128, 128, 128, 255];
  const RED: [u8; 4] = [255, 0, 0, 255];

  /// A gray image with the given quad filled red.
  fn image_with_quad(p_width: u32, p_height: u32, p_quad: &[[f32; 2]]) -> Image {
    let area = Area::from_points(p_quad);
    let mut bytes = Vec::with_capacity((p_width * p_height * 4) as usize);
    for y in 0..p_height {
      for x in 0..p_width {
        bytes.extend_from_slice(if area.contains((x as f32 + 0.5, y as f32 + 0.5)) { &RED } else { &GRAY });
      }
    }
    Image::new_from_pixels(p_width, p_height, &bytes, Channels::RGBA)
  }

  fn share_of_red(p_image: &Image) -> f32 {
    let pixels = p_image.rgba().chunks(4);
    let total = pixels.len() as f32;
    pixels.filter(|pixel| pixel[0] > 200 && pixel[1] < 60 && pixel[2] < 60).count() as f32 / total
  }

  #[test]
  fn the_whole_image_as_the_area_changes_nothing() {
    let bytes: Vec<u8> =
      (0..40 * 30).flat_map(|i| [(i % 251) as u8, (i % 13 * 19) as u8, (i % 7 * 36) as u8, 255]).collect();
    let mut image = Image::new_from_pixels(40, 30, &bytes, Channels::RGBA);
    perspective(Area::rect((0, 0), (40, 30))).apply(&mut image);

    assert_eq!(image.dimensions::<u32>(), (40, 30));
    let worst = image.rgba().iter().zip(&bytes).map(|(&a, &b)| (a as i32 - b as i32).abs()).max().unwrap();
    assert!(worst <= 2, "pixels drifted by {worst}");
  }

  #[test]
  fn a_skewed_quad_becomes_a_rectangle() {
    let quad = [[50.0, 40.0], [150.0, 60.0], [140.0, 150.0], [60.0, 130.0]];
    let mut image = image_with_quad(200, 200, &quad);
    perspective(Area::from_points(&quad)).with_fit(TransformFit::Crop).apply(&mut image);

    // The longer of the top and bottom edges is 102px, and of the left and right edges 91px. The rectangle is at
    // least that big, grown in one direction to the shape that leaves the nearest corner undistorted.
    let (width, height): (u32, u32) = image.dimensions();
    assert!(width >= 102 && height >= 91, "{width}x{height}");
    let expected = Area::from_points(&quad).to_quad().unwrap().flattened_aspect().unwrap();
    assert!((width as f64 / height as f64 - expected).abs() < 0.02, "{width}x{height} is not {expected:.3} wide");
    // Only the pixels right along the border can pick up the gray outside.
    assert!(share_of_red(&image) > 0.9, "only {:.0}% of the result is red", share_of_red(&image) * 100.0);
  }

  #[test]
  fn corner_order_does_not_matter() {
    let quad = [[50.0, 40.0], [150.0, 60.0], [140.0, 150.0], [60.0, 130.0]];
    let shuffled = [[140.0, 150.0], [50.0, 40.0], [60.0, 130.0], [150.0, 60.0]];

    let mut a = image_with_quad(200, 200, &quad);
    let mut b = a.clone();
    perspective(Area::from_points(&quad)).apply(&mut a);
    perspective(Area::from_points(&shuffled)).apply(&mut b);
    assert_eq!(a.dimensions::<u32>(), b.dimensions::<u32>());
    assert_eq!(a.rgba(), b.rgba());
  }

  #[test]
  fn shapes_without_four_straight_convex_corners_change_nothing() {
    let shapes = [
      ("a circle", Area::circle((50, 50), 30)),
      ("a triangle", Area::from_points(&[[10.0, 10.0], [90.0, 20.0], [50.0, 80.0]])),
      ("a dented shape", Area::from_points(&[[10.0, 10.0], [90.0, 10.0], [50.0, 90.0], [50.0, 30.0]])),
      ("a line", Area::from_points(&[[10.0, 10.0], [20.0, 20.0], [30.0, 30.0], [40.0, 40.0]])),
    ];
    for (name, area) in shapes {
      let mut image = image_with_quad(100, 100, &[[20.0, 20.0], [80.0, 20.0], [80.0, 80.0], [20.0, 80.0]]);
      let before = image.rgba().to_vec();
      perspective(area).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), (100, 100), "{name}");
      assert_eq!(image.rgba(), before.as_slice(), "{name}");
    }
  }

  /// The smallest box around the red pixels, as `(left, top, width, height)`, and how many there are.
  fn red_box(p_image: &Image) -> ((u32, u32, u32, u32), usize) {
    let (width, _): (u32, u32) = p_image.dimensions();
    let (mut min, mut max, mut count) = ((u32::MAX, u32::MAX), (0, 0), 0);
    for (index, pixel) in p_image.rgba().chunks(4).enumerate() {
      if pixel[0] > 200 && pixel[1] < 60 && pixel[2] < 60 && pixel[3] > 200 {
        let (x, y) = (index as u32 % width, index as u32 / width);
        min = (min.0.min(x), min.1.min(y));
        max = (max.0.max(x), max.1.max(y));
        count += 1;
      }
    }
    ((min.0, min.1, max.0 - min.0 + 1, max.1 - min.1 + 1), count)
  }

  #[test]
  fn without_a_crop_the_area_becomes_an_upright_box_shaped_like_its_bounding_box() {
    // The first is lopsided, the second like a building photographed from below: the bottom is level and the
    // widest, the right side is upright and the tallest, and the top and left lean in.
    for quad in [
      [[50.0, 40.0], [150.0, 60.0], [140.0, 150.0], [60.0, 130.0]],
      [[50.0, 30.0], [160.0, 20.0], [160.0, 130.0], [30.0, 130.0]],
    ] {
      let mut image = image_with_quad(200, 170, &quad);
      perspective(Area::from_points(&quad)).with_fit(TransformFit::Fill).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), (200, 170));

      let ((_, _, red_width, red_height), red) = red_box(&image);
      let (red_width, red_height) = (red_width as f64, red_height as f64);
      assert!(red as f64 > 0.95 * red_width * red_height, "red fills only {red} of {red_width}x{red_height}");
      let [top_left, _, bottom_right, _] = Area::from_points(&quad).to_quad().unwrap().bounds().corners();
      let expected = ((bottom_right.x - top_left.x) / (bottom_right.y - top_left.y)) as f64;
      assert!((red_width / red_height - expected).abs() < 0.04, "red box is {red_width}x{red_height} for {quad:?}");
    }
  }

  #[test]
  fn without_a_crop_the_whole_image_as_the_area_changes_nothing() {
    let bytes: Vec<u8> =
      (0..40 * 30).flat_map(|i| [(i % 251) as u8, (i % 13 * 19) as u8, (i % 7 * 36) as u8, 255]).collect();
    let mut image = Image::new_from_pixels(40, 30, &bytes, Channels::RGBA);
    perspective(Area::rect((0, 0), (40, 30))).with_fit(TransformFit::Fill).apply(&mut image);

    assert_eq!(image.dimensions::<u32>(), (40, 30));
    let worst = image.rgba().iter().zip(&bytes).map(|(&a, &b)| (a as i32 - b as i32).abs()).max().unwrap();
    assert!(worst <= 2, "pixels drifted by {worst}");
  }

  #[test]
  fn without_a_crop_no_transparent_gaps_open_up() {
    for quad in [
      [[50.0, 40.0], [150.0, 60.0], [140.0, 150.0], [60.0, 130.0]],
      [[50.0, 30.0], [160.0, 20.0], [160.0, 130.0], [30.0, 130.0]],
      // So strong that without zooming in, the line where the warp goes to infinity would be inside the image.
      [[45.0, 5.0], [55.0, 5.0], [100.0, 35.0], [0.0, 35.0]],
    ] {
      let mut image = image_with_quad(200, 200, &quad);
      perspective(Area::from_points(&quad)).with_fit(TransformFit::Fill).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), (200, 200));
      let clear = image.rgba().chunks(4).filter(|pixel| pixel[3] < 255).count();
      assert_eq!(clear, 0, "{clear} see-through pixels for {quad:?}");
    }
  }

  #[test]
  fn every_quality_setting_produces_the_same_shape() {
    let quad = [[50.0, 40.0], [150.0, 60.0], [140.0, 150.0], [60.0, 130.0]];
    for quality in [
      TransformAlgorithm::NearestNeighbor,
      TransformAlgorithm::Bilinear,
      TransformAlgorithm::Bicubic,
      TransformAlgorithm::Lanczos,
      TransformAlgorithm::Auto,
    ] {
      let mut image = image_with_quad(200, 200, &quad);
      perspective(Area::from_points(&quad)).with_fit(TransformFit::Crop).with_algorithm(quality).apply(&mut image);
      assert_eq!(image.dimensions::<u32>(), (102, 108), "{quality:?}");
    }
  }

  /// A checkerboard of 20px squares on a 200x120 sheet, seen through a perspective projection.
  fn photographed_checkerboard(p_quad: [(f64, f64); 4], p_width: u32, p_height: u32) -> Image {
    let [top_left, top_right, bottom_right, bottom_left] = p_quad;
    let photo = Quad::new(top_left, top_right, bottom_right, bottom_left);
    let to_sheet = Homography::from_quads(&photo, &Quad::rect((0, 0), (200, 120))).unwrap();
    let mut bytes = Vec::new();
    for y in 0..p_height {
      for x in 0..p_width {
        let color = match to_sheet.map_finite(x as f64 + 0.5, y as f64 + 0.5) {
          Some((u, v)) if (0.0..200.0).contains(&u) && (0.0..120.0).contains(&v) => {
            if ((u / 20.0).floor() as i32 + (v / 20.0).floor() as i32) % 2 == 0 {
              [255, 255, 255, 255]
            } else {
              [0, 0, 0, 255]
            }
          }
          _ => [128, 128, 128, 255],
        };
        bytes.extend_from_slice(&color);
      }
    }
    Image::new_from_pixels(p_width, p_height, &bytes, Channels::RGBA)
  }

  #[test]
  fn a_photographed_checkerboard_is_flattened_to_an_even_grid() {
    let quad = [(60.0, 40.0), (170.0, 55.0), (190.0, 150.0), (30.0, 140.0)];
    let mut image = photographed_checkerboard(quad, 220, 180);
    perspective(Area::from_points(&quad.map(|(x, y)| [x as f32, y as f32])))
      .with_fit(TransformFit::Crop)
      .apply(&mut image);

    // The sheet is 10x6 squares, so the corrected image must be too, however its aspect ratio came out.
    let (width, height): (u32, u32) = image.dimensions();
    let pixels = image.rgba();
    let (mut wrong, mut checked) = (0, 0);
    for y in 0..height {
      for x in 0..width {
        let (column, row) = ((x as f64 + 0.5) / width as f64 * 10.0, (y as f64 + 0.5) / height as f64 * 6.0);
        // Skip pixels right at a square's edge, where a small offset legitimately flips the color.
        if (column - column.round()).abs() * (width as f64 / 10.0) < 2.0
          || (row - row.round()).abs() * (height as f64 / 6.0) < 2.0
        {
          continue;
        }
        let expected_white = (column.floor() as i32 + row.floor() as i32) % 2 == 0;
        checked += 1;
        if (pixels[((y * width + x) * 4) as usize] > 127) != expected_white {
          wrong += 1;
        }
      }
    }
    assert!(checked > 1000, "only {checked} pixels checked");
    assert!(wrong * 100 <= checked, "{wrong} of {checked} pixels are the wrong color in {width}x{height}");
  }

  #[test]
  fn expand_keeps_the_whole_corrected_image() {
    let quad = [[50.0, 40.0], [150.0, 60.0], [140.0, 150.0], [60.0, 130.0]];
    let mut image = image_with_quad(200, 200, &quad);
    perspective(Area::from_points(&quad)).with_fit(TransformFit::Expand).apply(&mut image);
    // Stretching the area out pulls the image's edges inward, leaving transparent gaps instead of zooming in.
    let (width, height) = image.dimensions::<u32>();
    assert!(width >= 200 && height >= 200, "{width}x{height}");
    assert!(image.rgba().chunks(4).any(|pixel| pixel[3] == 0), "expected transparent gaps");
  }
}
