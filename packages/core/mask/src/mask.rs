use abra_core::{Area, Color, Image, PointF, blend};

use drawing::fill;

/// Helper trait to convert various types into an optional PointF
pub trait IntoOptionalPointF {
  /// Converts the value into an optional PointF.
  fn into_optional_point_f(self) -> Option<PointF>;
}

impl IntoOptionalPointF for Option<PointF> {
  fn into_optional_point_f(self) -> Option<PointF> {
    self
  }
}

impl IntoOptionalPointF for PointF {
  fn into_optional_point_f(self) -> Option<PointF> {
    Some(self)
  }
}

impl IntoOptionalPointF for (f32, f32) {
  fn into_optional_point_f(self) -> Option<PointF> {
    Some(PointF::from(self))
  }
}

impl IntoOptionalPointF for (i32, i32) {
  fn into_optional_point_f(self) -> Option<PointF> {
    Some(PointF::from(self))
  }
}

impl IntoOptionalPointF for (u32, u32) {
  fn into_optional_point_f(self) -> Option<PointF> {
    Some(PointF::from(self))
  }
}

impl IntoOptionalPointF for (f64, f64) {
  fn into_optional_point_f(self) -> Option<PointF> {
    Some(PointF::from(self))
  }
}

/// A mask defines an area used for masking operations in image processing.
/// It encapsulates a geometric area that can be applied to images.
/// The mask can be created from various geometric shapes and used to
/// control the visibility of image regions.
///
/// A color of white (`255`) in the mask represents fully opaque areas,
/// a black color (`0`) represents fully transparent areas, and gray values in between
/// represent varying levels of transparency.

#[derive(Clone, Debug)]
pub struct Mask {
  /// The image representation of the mask.
  image_mask: Image,
}

impl Mask {
  /// Creates a new empty Mask from an existing Image.
  /// - `p_image`: The Image to create the mask from.
  pub fn new_from_image(p_src_image: &Image) -> Mask {
    let (width, height) = p_src_image.dimensions::<u32>();
    let image = Image::new_from_color(width, height, Color::from_rgba(255, 255, 255, 255));
    Mask { image_mask: image }
  }

  /// Create a mask by consuming an Image.
  pub fn from_image(img: Image) -> Mask {
    Mask { image_mask: img }
  }
}

impl From<Image> for Mask {
  fn from(img: Image) -> Mask {
    Mask::from_image(img)
  }
}

impl Mask {
  /// Draws a filled area onto the mask with the specified color.
  /// - `p_area`: The Area to draw.
  /// - `p_color`: The Color to use for the area.
  /// - `p_at`: Optional position as a tuple, PointF, or None. Defaults to (0, 0) if not provided.
  pub fn draw_area(&mut self, p_area: &Area, p_color: Color, p_at: impl IntoOptionalPointF) {
    let color = self.to_color(p_color);
    let position = p_at.into_optional_point_f().unwrap_or(PointF::new(0, 0));
    let filled_image = fill(p_area, color);
    blend::blend_images_at(
      &mut self.image_mask,
      &filled_image,
      0,
      0,
      position.x as i32,
      position.y as i32,
      blend::normal,
    );
  }

  /// The underlying mask image.
  pub fn image(&self) -> &Image {
    &self.image_mask
  }

  /// Apply the mask to an image by adjusting the image's alpha channel.
  ///
  /// The mask is interpreted as a grayscale image where:
  /// - 255 (white) -> alpha 255 (fully opaque / visible)
  /// - 0 (black) -> alpha 0 (fully transparent / hidden)
  ///
  /// The mask size must match the image size. If you need positioning, use `Image::set_from` or a temporary canvas.
  pub fn apply_to_image(&self, p_image: &mut Image) {
    let mask_bytes = self.image().rgba();
    if let Some(pixels) = p_image.colors().as_slice_mut() {
      apply_mask_to_pixels_rgba(pixels, &mask_bytes);
    }
  }

  fn to_color(&self, color: Color) -> Color {
    let c = ((color.r as u16 + color.g as u16 + color.b as u16) / 3) as u8;
    Color::from_rgba(c, c, c, color.a)
  }
}

/// Converts a grayscale mask value to an alpha value where:
/// - 255 (white) => 255 alpha (fully opaque/visible)
/// - 0 (black) => 0 alpha (fully transparent/hidden)
/// - 127 (gray) => ~127 alpha (~50% opacity)
pub fn mask_value_to_alpha(p_value: u8) -> u8 {
  p_value
}

/// Computes a grayscale mask value from an RGBA pixel using a standard
/// luma approximation and ignores the input alpha channel.
/// Computes grayscale (luma) from an RGBA pixel using the ITU-R BT.601
/// approximation. Exposed publicly so other crates can reuse the
/// standard luma transform to avoid duplicated code.
#[inline]
pub fn rgba_to_gray(p_rgba: &[u8]) -> u8 {
  // ITU-R BT.601 luma transform (approximation with integer math)
  // gray = 0.299 R + 0.587 G + 0.114 B
  let r = p_rgba[0] as u32;
  let g = p_rgba[1] as u32;
  let b = p_rgba[2] as u32;
  (((299 * r + 587 * g + 114 * b) + 500) / 1000) as u8
}

/// Applies a mask to an image by setting the image's alpha channel from the provided mask data.
///
/// Semantics:
/// - White (255) in the mask becomes fully opaque (alpha 255)
/// - Black (0) in the mask becomes fully transparent (alpha 0)
/// - Gray values map linearly between 0 and 255
///
/// Mask input may be one of:
/// - Grayscale: length = width * height
/// - RGBA: length = width * height * 4 (converted to grayscale)
pub fn apply_mask_to_image(p_image: &mut Image, p_mask: &[u8]) {
  let (width, height) = p_image.dimensions::<usize>();
  let px_count = width * height;

  let mask_gray: Vec<u8> = match p_mask.len() {
    len if len == px_count => p_mask.to_vec(),
    len if len == px_count * 4 => p_mask.chunks(4).map(|px| rgba_to_gray(px)).collect(),
    other => panic!("Invalid mask size: expected {} (gray) or {} (rgba) but got {}", px_count, px_count * 4, other),
  };

  let alphas: Vec<u8> = mask_gray.into_iter().map(mask_value_to_alpha).collect();

  // Write alphas into the image buffer
  if let Some(pixels) = p_image.colors().as_slice_mut() {
    // Write alphas directly into the image buffer
    for (rgba, &a) in pixels.chunks_mut(4).zip(alphas.iter()) {
      rgba[3] = a;
    }
  }
}

/// Applies a mask directly to an RGBA pixel slice by setting its alpha channel.
///
/// See `apply_mask_to_image` for mask semantics and accepted formats.
pub fn apply_mask_to_pixels_rgba(p_pixels: &mut [u8], p_mask: &[u8]) {
  assert!(p_pixels.len() % 4 == 0, "pixels must be RGBA (len divisible by 4)");
  let px_count = p_pixels.len() / 4;

  let mask_gray: Vec<u8> = match p_mask.len() {
    len if len == px_count => p_mask.to_vec(),
    len if len == px_count * 4 => p_mask.chunks(4).map(|px| rgba_to_gray(px)).collect(),
    other => panic!("Invalid mask size: expected {} (gray) or {} (rgba) but got {}", px_count, px_count * 4, other),
  };

  // Apply in-place
  for (rgba, &m) in p_pixels.chunks_mut(4).zip(mask_gray.iter()) {
    rgba[3] = mask_value_to_alpha(m);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Area, Color, Image};

  #[test]
  fn mask_clone_shares_buffer_and_cow_on_draw() {
    let img = Image::new_from_color(20, 20, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::from(img);
    let ptr1 = mask.image().rgba().as_ptr();
    let mask_clone = mask.clone();
    let ptr2 = mask_clone.image().rgba().as_ptr();
    assert_eq!(ptr1, ptr2, "Mask clones should share underlying Image buffer");

    // Mutate the original mask - should trigger copy-on-write in Image
    let area = Area::rect((1.0, 1.0), (2.0, 2.0));
    mask.draw_area(&area, Color::black(), None);
    assert_ne!(mask.image().rgba().as_ptr(), ptr2, "Mutation should have caused the original mask's image to COW");
    assert_eq!(
      mask_clone.image().rgba().as_ptr(),
      ptr2,
      "Clone's buffer pointer should still be same after original mutated"
    );
  }

  #[test]
  fn test_apply_mask_to_pixels_rgba() {
    // Two pixels: RGBA (red, green)
    let mut pixels: Vec<u8> = vec![255, 0, 0, 255, 0, 255, 0, 255];
    // mask: first pixel black (transparent), second pixel white (opaque)
    let mask: Vec<u8> = vec![0, 255];
    apply_mask_to_pixels_rgba(&mut pixels, &mask);
    assert_eq!(pixels[3], 0);
    assert_eq!(pixels[7], 255);
  }

  #[test]
  fn test_apply_mask_to_image_using_mask_struct() {
    let mut img = Image::new_from_color(2, 1, Color::from_rgba(255, 0, 0, 255));
    // create a mask image with first pixel black, second white
    let mut mask_img = Image::new_from_color(2, 1, Color::from_rgba(255, 255, 255, 255));
    // set first pixel to black on mask
    mask_img.set_pixel(0, 0, (0, 0, 0, 255));
    let mask = Mask::from(mask_img);
    mask.apply_to_image(&mut img);
    let rgba = img.to_rgba_vec();
    assert_eq!(rgba[3], 0);
    assert_eq!(rgba[7], 255);
  }

  #[test]
  fn apply_to_image_does_not_copy_mask() {
    let mut img = Image::new_from_color(2, 1, Color::from_rgba(255, 0, 0, 255));
    let mut mask_img = Image::new_from_color(2, 1, Color::from_rgba(255, 255, 255, 255));
    // set first pixel to black on mask
    mask_img.set_pixel(0, 0, (0, 0, 0, 255));
    let mask = Mask::from(mask_img);
    let before_ptr = mask.image().rgba().as_ptr();
    mask.apply_to_image(&mut img);
    let after_ptr = mask.image().rgba().as_ptr();
    assert_eq!(before_ptr, after_ptr, "apply_to_image should not mutate or clone the mask's internal buffer");
  }

  #[test]
  fn draw_area_respects_feathering() {
    let img = Image::new_from_color(10, 10, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::new_from_image(&img);
    let area = Area::rect((1.0, 1.0), (8.0, 8.0)).with_feather(2);
    mask.draw_area(&area, Color::black(), None);
    // center should be black (0) - fully drawn area
    let center_alpha = mask.image().get_pixel(5, 5).unwrap().0; // grayscale value in mask image
    assert_eq!(center_alpha, 0);
    // ensure we have at least one partially transparent pixel within the area (alpha not strictly 0 or 255)
    let mut found_partial = false;
    let min_x = 1usize;
    let min_y = 1usize;
    let max_x = 8usize;
    let max_y = 8usize;
    for y in min_y..=max_y {
      for x in min_x..=max_x {
        let alpha = mask.image().get_pixel(x as u32, y as u32).unwrap().0;
        if alpha > 0 && alpha < 255 {
          found_partial = true;
          break;
        }
      }
      if found_partial {
        break;
      }
    }
    assert!(found_partial, "Expected to find partial-coverage pixels for feathered area");
  }

  #[test]
  fn draw_star_area_offset_is_correct() {
    use abra_core::image::image_ext::*;
    use abra_core::{AspectRatio, Star};
    let img = Image::new_from_color(200, 200, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::new_from_image(&img);
    // Create a star area sized to half the image and not positioned explicitly
    let area = Star::new().fit_with_aspect(img.size() / 2, AspectRatio::meet());
    mask.draw_area(&area, Color::black(), None);
    // compute topmost row with non-white pixel
    let mut topmost: Option<u32> = None;
    for y in 0..200u32 {
      for x in 0..200u32 {
        if mask.image().get_pixel(x, y).unwrap().0 != 255 {
          topmost = Some(y);
          break;
        }
      }
      if topmost.is_some() {
        break;
      }
    }
    assert!(topmost.is_some());
    // If shape is anchored to 0, expect topmost to be 0
    assert_eq!(topmost.unwrap(), 0);
  }

  #[test]
  fn draw_star_area_offset_with_position() {
    use abra_core::image::image_ext::*;
    use abra_core::{AspectRatio, Star};
    let img = Image::new_from_color(200, 200, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::new_from_image(&img);
    let area = Star::new().fit_with_aspect(img.size() / 2, AspectRatio::meet());
    // Draw with an explicit offset
    mask.draw_area(&area, Color::black(), (10.0, 20.0));
    // compute topmost row with non-white pixel
    let mut topmost: Option<u32> = None;
    for y in 0..200u32 {
      for x in 0..200u32 {
        if mask.image().get_pixel(x, y).unwrap().0 != 255 {
          topmost = Some(y);
          break;
        }
      }
      if topmost.is_some() {
        break;
      }
    }
    assert!(topmost.is_some());
    // Expect topmost to be 20 due to the offset provided earlier
    assert_eq!(topmost.unwrap(), 20);
  }
}
