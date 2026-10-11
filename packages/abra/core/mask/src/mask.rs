use abra_core::{Area, Color, GrayPlane, Image, PointF, ResizeTarget, Size, Transform, TransformAlgorithm, blend};
pub use abra_core::rgba_to_gray;
use std::borrow::Cow;

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

/// A mask says how much of an operation reaches each pixel of an image.
///
/// It holds one byte per pixel: `255` means fully applied, `0` means untouched, and values in between apply
/// partially. A mask is a quarter of the size of the same picture as an RGBA image. Clones share their bytes until one
/// of them changes.
#[derive(Clone, Debug)]
pub struct Mask {
  plane: GrayPlane,
}

/// Converts an owned or borrowed mask into a copy-on-write mask reference.
pub trait IntoMaskCow<'a> {
  /// Converts this value without cloning borrowed masks.
  fn into_mask_cow(self) -> Cow<'a, Mask>;
}

impl<'a> IntoMaskCow<'a> for &'a Mask {
  fn into_mask_cow(self) -> Cow<'a, Mask> {
    Cow::Borrowed(self)
  }
}

impl<'a> IntoMaskCow<'a> for Mask {
  fn into_mask_cow(self) -> Cow<'a, Mask> {
    Cow::Owned(self)
  }
}

impl Mask {
  /// Creates a mask the size of an existing image that lets everything through (all `255`).
  /// - `p_src_image`: The image whose size the mask takes.
  pub fn new_from_image(p_src_image: &Image) -> Mask {
    let (width, height) = p_src_image.dimensions::<u32>();
    Mask::from_values(width, height, vec![255; width as usize * height as usize])
  }

  /// Creates a mask from one value per pixel.
  /// - `p_width`, `p_height`: The size of the mask.
  /// - `p_values`: The values, row by row from the top left. Its length must be `p_width * p_height`.
  pub fn from_values(p_width: u32, p_height: u32, p_values: Vec<u8>) -> Mask {
    Mask {
      plane: GrayPlane::new(p_width, p_height, p_values),
    }
  }

  /// Creates a mask from the brightness of each pixel of an image: white is `255` and black is `0`. The image's
  /// alpha is ignored.
  pub fn from_image(p_img: Image) -> Mask {
    Mask {
      plane: GrayPlane::from_image(&p_img),
    }
  }

  /// The mask's values and size, which is what effects read.
  pub fn plane(&self) -> &GrayPlane {
    &self.plane
  }

  /// The width of the mask in pixels.
  pub fn width(&self) -> u32 {
    self.plane.width()
  }

  /// The height of the mask in pixels.
  pub fn height(&self) -> u32 {
    self.plane.height()
  }

  /// The size of the mask as a tuple of `T` (generic integer type).
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u64>,
    <T as TryFrom<u64>>::Error: std::fmt::Debug,
  {
    self.plane.dimensions()
  }

  /// The mask values, one per pixel, row by row from the top left.
  pub fn values(&self) -> &[u8] {
    self.plane.values()
  }

  /// The mask value at a pixel, or `None` outside the mask.
  pub fn get(&self, p_x: u32, p_y: u32) -> Option<u8> {
    self.plane.get(p_x, p_y)
  }

  /// The mask as a gray RGBA image, for drawing or saving.
  pub fn to_image(&self) -> Image {
    self.plane.to_image()
  }

  /// A copy of the mask scaled to a new size.
  /// - `p_width`, `p_height`: The new size.
  /// - `p_algorithm`: How values between pixels are worked out.
  pub fn resized(&self, p_width: u32, p_height: u32, p_algorithm: TransformAlgorithm) -> Mask {
    let mut image = self.to_image();
    image.resize(ResizeTarget::Exact(Size::new(p_width, p_height)), p_algorithm);
    Mask::from_image(image)
  }
}

impl From<Mask> for GrayPlane {
  fn from(p_mask: Mask) -> GrayPlane {
    p_mask.plane
  }
}

impl From<&Mask> for GrayPlane {
  fn from(p_mask: &Mask) -> GrayPlane {
    p_mask.plane.clone()
  }
}

impl From<Mask> for Image {
  fn from(p_mask: Mask) -> Image {
    p_mask.to_image()
  }
}

impl From<Image> for Mask {
  fn from(p_img: Image) -> Mask {
    Mask::from_image(p_img)
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
    let filled_image = fill(p_area, &color).to_image();
    let mut image = self.to_image();
    blend::blend(&filled_image).with_offset((position.x as i32, position.y as i32)).apply(&mut image);
    // A new buffer, so clones made earlier keep the old values.
    self.plane = Mask::from_image(image).plane;
  }

  /// Apply the mask to an image by adjusting the image's alpha channel.
  ///
  /// The mask value becomes the alpha:
  /// - 255 -> alpha 255 (fully opaque / visible)
  /// - 0 -> alpha 0 (fully transparent / hidden)
  ///
  /// The mask size must match the image size. If you need positioning, use `Image::set_from` or a temporary canvas.
  pub fn apply_to_image(&self, p_image: &mut Image) {
    if let Some(pixels) = p_image.colors().as_slice_mut() {
      apply_mask_to_pixels_rgba(pixels, self.values());
    }
  }

  fn to_color(&self, p_color: Color) -> Color {
    let c = ((p_color.r as u16 + p_color.g as u16 + p_color.b as u16) / 3) as u8;
    Color::from_rgba(c, c, c, p_color.a)
  }
}

/// Converts a grayscale mask value to an alpha value where:
/// - 255 (white) => 255 alpha (fully opaque/visible)
/// - 0 (black) => 0 alpha (fully transparent/hidden)
/// - 127 (gray) => ~127 alpha (~50% opacity)
pub fn mask_value_to_alpha(p_value: u8) -> u8 {
  p_value
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
  if let Some(pixels) = p_image.colors().as_slice_mut() {
    apply_mask_to_rgba_pixels(pixels, p_mask, width * height);
  }
}

/// Applies a mask directly to an RGBA pixel slice by setting its alpha channel.
///
/// See `apply_mask_to_image` for mask semantics and accepted formats.
pub fn apply_mask_to_pixels_rgba(p_pixels: &mut [u8], p_mask: &[u8]) {
  assert!(p_pixels.len() % 4 == 0, "pixels must be RGBA (len divisible by 4)");
  apply_mask_to_rgba_pixels(p_pixels, p_mask, p_pixels.len() / 4);
}

fn apply_mask_to_rgba_pixels(p_pixels: &mut [u8], p_mask: &[u8], p_px_count: usize) {
  // A grayscale mask is used as it is; only an RGBA mask needs converting.
  let mask_gray: Cow<[u8]> = match p_mask.len() {
    len if len == p_px_count => Cow::Borrowed(p_mask),
    len if len == p_px_count * 4 => Cow::Owned(p_mask.chunks(4).map(rgba_to_gray).collect()),
    other => panic!("Invalid mask size: expected {} (gray) or {} (rgba) but got {}", p_px_count, p_px_count * 4, other),
  };

  for (rgba, &m) in p_pixels.chunks_mut(4).zip(mask_gray.iter()) {
    rgba[3] = mask_value_to_alpha(m);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Area, Color, Image};

  #[test]
  fn borrowed_mask_input_keeps_the_original_reference() {
    let mask = Mask::from_image(Image::new(1, 1));
    let mask_ref = &mask;

    assert!(matches!(mask_ref.into_mask_cow(), Cow::Borrowed(borrowed) if std::ptr::eq(borrowed, mask_ref)));
  }

  #[test]
  fn a_mask_holds_one_byte_per_pixel() {
    let mask = Mask::from_image(Image::new_from_color(20, 10, Color::from_rgba(255, 255, 255, 255)));

    assert_eq!(mask.dimensions::<u32>(), (20, 10));
    assert_eq!(mask.values().len(), 200);
    assert_eq!(mask.get(19, 9), Some(255));
    assert_eq!(mask.get(20, 0), None);
  }

  #[test]
  fn an_image_round_trips_through_a_mask() {
    let mut image = Image::new_from_color(2, 1, Color::from_rgba(255, 255, 255, 255));
    image.set_pixel(0, 0, (40, 40, 40, 255));
    let mask = Mask::from_image(image);

    assert_eq!(mask.values(), &[40, 255]);
    assert_eq!(mask.to_image().rgba(), &[40, 40, 40, 255, 255, 255, 255, 255]);
  }

  #[test]
  fn resizing_keeps_a_uniform_mask_uniform() {
    let mask = Mask::from_values(4, 4, vec![200; 16]).resized(8, 6, TransformAlgorithm::Bilinear);

    assert_eq!(mask.dimensions::<u32>(), (8, 6));
    assert!(mask.values().iter().all(|value| *value == 200));
  }

  #[test]
  fn mask_clone_shares_buffer_and_cow_on_draw() {
    let img = Image::new_from_color(20, 20, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::from(img);
    let ptr1 = mask.values().as_ptr();
    let mask_clone = mask.clone();
    let ptr2 = mask_clone.values().as_ptr();
    assert_eq!(ptr1, ptr2, "Mask clones should share their values");

    // Drawing on the original must leave the clone alone.
    let area = Area::rect((1.0, 1.0), (2.0, 2.0));
    mask.draw_area(&area, Color::black(), None);
    assert_ne!(mask.values().as_ptr(), ptr2, "Drawing should have given the original mask its own values");
    assert_eq!(
      mask_clone.values().as_ptr(),
      ptr2,
      "Clone's buffer pointer should still be same after original mutated"
    );
    assert_eq!(mask_clone.get(1, 1), Some(255), "the clone is unchanged");
    assert_eq!(mask.get(1, 1), Some(0), "the original was drawn on");
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
    let before_ptr = mask.values().as_ptr();
    mask.apply_to_image(&mut img);
    let after_ptr = mask.values().as_ptr();
    assert_eq!(before_ptr, after_ptr, "apply_to_image should not mutate or clone the mask's internal buffer");
  }

  #[test]
  fn draw_area_respects_feathering() {
    let img = Image::new_from_color(10, 10, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::new_from_image(&img);
    let area = Area::rect((1.0, 1.0), (8.0, 8.0)).with_feather(2);
    mask.draw_area(&area, Color::black(), None);
    // center should be black (0) - fully drawn area
    let center_alpha = mask.get(5, 5).unwrap();
    assert_eq!(center_alpha, 0);
    // ensure we have at least one partially transparent pixel within the area (alpha not strictly 0 or 255)
    let mut found_partial = false;
    let min_x = 1usize;
    let min_y = 1usize;
    let max_x = 8usize;
    let max_y = 8usize;
    for y in min_y..=max_y {
      for x in min_x..=max_x {
        let alpha = mask.get(x as u32, y as u32).unwrap();
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
    use abra_core::{AspectRatio, Shape};
    let img = Image::new_from_color(200, 200, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::new_from_image(&img);
    // Create a star area sized to half the image and not positioned explicitly
    let area = Area::shape(Shape::Star).fit(img.size() / 2, AspectRatio::meet());
    mask.draw_area(&area, Color::black(), None);
    // compute topmost row with non-white pixel
    let mut topmost: Option<u32> = None;
    for y in 0..200u32 {
      for x in 0..200u32 {
        if mask.get(x, y).unwrap() != 255 {
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
    use abra_core::{AspectRatio, Shape};
    let img = Image::new_from_color(200, 200, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::new_from_image(&img);
    let area = Area::shape(Shape::Star).fit(img.size() / 2, AspectRatio::meet());
    // Draw with an explicit offset
    mask.draw_area(&area, Color::black(), (10.0, 20.0));
    // compute topmost row with non-white pixel
    let mut topmost: Option<u32> = None;
    for y in 0..200u32 {
      for x in 0..200u32 {
        if mask.get(x, y).unwrap() != 255 {
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
