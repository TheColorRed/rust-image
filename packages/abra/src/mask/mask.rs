use std::sync::{Arc, Mutex};

use crate::{
  Image,
  color::Color,
  combine::blend,
  geometry::{Area, PointF, Size},
};

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
pub struct Mask {
  /// The image representation of the mask.
  image_mask: Image,
}

impl Mask {
  /// Creates a new empty Mask from an existing Image.
  /// - `p_image`: The Image to create the mask from.
  pub fn new_from_image(p_src_image: &Image) -> Mask {
    let Size { width, height } = p_src_image.size();
    let image = Image::new_from_color(width as u32, height as u32, Color::from_rgba(255, 255, 255, 255));
    Mask { image_mask: image }
  }

  /// Draws a filled area onto the mask with the specified color.
  /// - `p_area`: The Area to draw.
  /// - `p_color`: The Color to use for the area.
  /// - `p_at`: Optional position as a tuple, PointF, or None. Defaults to (0, 0) if not provided.
  pub fn draw_area(&mut self, p_area: &Area, p_color: Color, p_at: impl IntoOptionalPointF) {
    let color = self.to_color(p_color);
    let position = p_at.into_optional_point_f().unwrap_or(PointF::new(0, 0));
    let filled_image = p_area.fill(color);
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

  fn to_color(&self, color: Color) -> Color {
    let c = ((color.r as u16 + color.g as u16 + color.b as u16) / 3) as u8;
    Color::from_rgba(c, c, c, color.a)
  }
}

// use rayon::prelude::*;

// use crate::Image;

// /// Converts a grayscale mask value to an alpha value where:
// /// - 255 (white) => 0 alpha (fully transparent)
// /// - 0 (black) => 255 alpha (fully opaque)
// /// - 127 (gray) => ~128 alpha (~50% opaque)
// pub fn mask_value_to_alpha(p_value: u8) -> u8 {
//   255u8.saturating_sub(p_value)
// }

// /// Computes a grayscale mask value from an RGBA pixel using a standard
// /// luma approximation and ignores the input alpha channel.
// #[inline]
// fn rgba_to_gray(p_rgba: &[u8]) -> u8 {
//   // ITU-R BT.601 luma transform (approximation with integer math)
//   // gray = 0.299 R + 0.587 G + 0.114 B
//   let r = p_rgba[0] as u16;
//   let g = p_rgba[1] as u16;
//   let b = p_rgba[2] as u16;
//   (((299 * r + 587 * g + 114 * b) + 500) / 1000) as u8
// }

// /// Applies a mask to an image by setting the image's alpha channel from the provided mask data.
// ///
// /// Semantics:
// /// - White (255) in the mask becomes fully transparent (alpha 0)
// /// - Black (0) in the mask becomes fully opaque (alpha 255)
// /// - Gray values in-between map linearly (e.g. 127 ~ 50% transparency)
// ///
// /// Mask input may be one of:
// /// - Grayscale: length = width * height
// /// - RGBA: length = width * height * 4 (converted to grayscale)
// pub fn apply_mask_to_image(p_image: &mut Image, p_mask: &[u8]) {
//   let (width, height) = p_image.dimensions::<usize>();
//   let px_count = width * height;

//   let mask_gray: Vec<u8> = match p_mask.len() {
//     len if len == px_count => p_mask.to_vec(),
//     len if len == px_count * 4 => p_mask.par_chunks(4).map(|px| rgba_to_gray(px)).collect(),
//     other => panic!("Invalid mask size: expected {} (gray) or {} (rgba) but got {}", px_count, px_count * 4, other),
//   };

//   let alphas: Vec<u8> = mask_gray.into_par_iter().map(mask_value_to_alpha).collect();

//   // Write alphas into the image buffer
//   let colors = p_image.colors.to_vec();
//   let mut out: Vec<u8> = Vec::with_capacity(px_count * 4);
//   out.par_extend(
//     colors
//       .par_chunks(4)
//       .zip(alphas.par_iter())
//       .flat_map_iter(|(rgba, &a)| [rgba[0], rgba[1], rgba[2], a]),
//   );
//   p_image.set_rgba(out);
// }

// /// Applies a mask directly to an RGBA pixel slice by setting its alpha channel.
// ///
// /// See `apply_mask_to_image` for mask semantics and accepted formats.
// pub fn apply_mask_to_pixels_rgba(p_pixels: &mut [u8], p_mask: &[u8]) {
//   assert!(p_pixels.len() % 4 == 0, "pixels must be RGBA (len divisible by 4)");
//   let px_count = p_pixels.len() / 4;

//   let mask_gray: Vec<u8> = match p_mask.len() {
//     len if len == px_count => p_mask.to_vec(),
//     len if len == px_count * 4 => p_mask.par_chunks(4).map(|px| rgba_to_gray(px)).collect(),
//     other => panic!("Invalid mask size: expected {} (gray) or {} (rgba) but got {}", px_count, px_count * 4, other),
//   };

//   // Apply in-place
//   p_pixels
//     .par_chunks_mut(4)
//     .zip(mask_gray.par_iter())
//     .for_each(|(rgba, &m)| {
//       rgba[3] = mask_value_to_alpha(m);
//     });
// }
