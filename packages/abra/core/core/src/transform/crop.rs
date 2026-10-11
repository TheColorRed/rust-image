use rayon::prelude::*;

use crate::{Channels, Image, IntoNumber, Rect};

/// A crop that has been described but not yet run. Create one with [`crop`], then run it with [`CropImage::apply`].
pub struct CropImage {
  x: u32,
  y: u32,
  width: u32,
  height: u32,
}

impl CropImage {
  /// Crops the image in place. The rectangle is clipped to the image; when nothing of it overlaps the image, the
  /// image is left unchanged.
  pub fn apply(&self, p_image: &mut Image) {
    let (image_width, image_height) = p_image.dimensions::<u32>();
    let keep =
      Rect::new((self.x, self.y), (self.width, self.height)).intersect(Rect::new((0, 0), (image_width, image_height)));
    if keep.is_empty() {
      return;
    }
    let (left, top, right, bottom) = keep.edges::<u32>();
    let (width, height) = (right - left, bottom - top);
    if (width, height) == (image_width, image_height) {
      return;
    }

    let source = p_image.rgba();
    let (source_stride, row_bytes) = (image_width as usize * 4, width as usize * 4);
    let mut pixels = vec![0u8; row_bytes * height as usize];
    pixels.par_chunks_exact_mut(row_bytes).enumerate().for_each(|(y, row)| {
      let start = (top as usize + y) * source_stride + left as usize * 4;
      row.copy_from_slice(&source[start..start + row_bytes]);
    });
    p_image.set_pixels(width, height, pixels, Channels::RGBA);
  }
}

/// Crop the image to the given rectangle.
/// - `p_x`, `p_y`: The top-left corner of the rectangle to keep.
/// - `p_width`, `p_height`: The size of the rectangle to keep.
pub fn crop(
  p_x: impl IntoNumber, p_y: impl IntoNumber, p_width: impl IntoNumber, p_height: impl IntoNumber,
) -> CropImage {
  CropImage {
    x: p_x.into::<u32>(),
    y: p_y.into::<u32>(),
    width: p_width.into::<u32>(),
    height: p_height.into::<u32>(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn crop_keeps_the_rectangle_and_clips_to_the_image() {
    let pixels: Vec<u8> = (0..16u8).flat_map(|i| [i, 0, 0, 255]).collect();
    let mut image = Image::new_from_pixels(4, 4, pixels, Channels::RGBA);
    crop(1, 2, 10, 10).apply(&mut image);
    assert_eq!(image.dimensions::<u32>(), (3, 2));
    assert_eq!(image.get_pixel(0, 0), Some((9, 0, 0, 255)));
    assert_eq!(image.get_pixel(2, 1), Some((15, 0, 0, 255)));

    crop(50, 50, 5, 5).apply(&mut image);
    assert_eq!(image.dimensions::<u32>(), (3, 2));
  }
}
