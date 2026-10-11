use primitives::{Channels, Image};
use rayon::prelude::*;
use std::sync::Arc;

/// One value per pixel, `0` to `255`, with no color: the shape of a mask. It is a quarter of the size of the same
/// picture as an RGBA [`Image`]. Clones share their bytes, so passing one around is cheap.
#[derive(Clone, Debug)]
pub struct GrayPlane {
  width: u32,
  height: u32,
  /// Row by row from the top left, `width * height` long.
  values: Arc<Vec<u8>>,
}

impl GrayPlane {
  /// Creates a plane from one value per pixel.
  /// - `p_width`, `p_height`: The size of the plane.
  /// - `p_values`: The values, row by row from the top left. Its length must be `p_width * p_height`.
  pub fn new(p_width: u32, p_height: u32, p_values: Vec<u8>) -> GrayPlane {
    assert_eq!(
      p_values.len(),
      p_width as usize * p_height as usize,
      "a plane needs one value per pixel"
    );
    GrayPlane {
      width: p_width,
      height: p_height,
      values: Arc::new(p_values),
    }
  }

  /// Creates a plane from the brightness of each pixel of an image: white is `255` and black is `0`. The image's alpha
  /// is ignored.
  pub fn from_image(p_image: &Image) -> GrayPlane {
    let (width, height) = p_image.dimensions::<u32>();
    GrayPlane::new(width, height, p_image.rgba().par_chunks_exact(4).map(rgba_to_gray).collect())
  }

  /// The width in pixels.
  pub fn width(&self) -> u32 {
    self.width
  }

  /// The height in pixels.
  pub fn height(&self) -> u32 {
    self.height
  }

  /// The size as a tuple of `T` (generic integer type).
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u64>,
    <T as TryFrom<u64>>::Error: std::fmt::Debug,
  {
    (T::try_from(self.width as u64).unwrap(), T::try_from(self.height as u64).unwrap())
  }

  /// The values, one per pixel, row by row from the top left.
  pub fn values(&self) -> &[u8] {
    &self.values
  }

  /// The value at a pixel, or `None` outside the plane.
  pub fn get(&self, p_x: u32, p_y: u32) -> Option<u8> {
    if p_x >= self.width || p_y >= self.height {
      return None;
    }
    self.values.get(p_y as usize * self.width as usize + p_x as usize).copied()
  }

  /// Whether `p_other` uses the very same bytes, as a clone does. Cheap, and `false` only means "may differ".
  pub fn shares_values_with(&self, p_other: &GrayPlane) -> bool {
    Arc::ptr_eq(&self.values, &p_other.values)
  }

  /// The plane as a gray RGBA image, for drawing or saving.
  pub fn to_image(&self) -> Image {
    let mut rgba = vec![255u8; self.values.len() * 4];
    rgba.par_chunks_exact_mut(4).zip(self.values.par_iter()).for_each(|(pixel, &value)| pixel[..3].fill(value));
    Image::new_from_pixels(self.width, self.height, rgba, Channels::RGBA)
  }
}

impl From<&Image> for GrayPlane {
  fn from(p_image: &Image) -> GrayPlane {
    GrayPlane::from_image(p_image)
  }
}

impl From<Image> for GrayPlane {
  fn from(p_image: Image) -> GrayPlane {
    GrayPlane::from_image(&p_image)
  }
}

/// The brightness of an RGBA pixel, ignoring its alpha, using the ITU-R BT.601 weights (`0.299 R + 0.587 G +
/// 0.114 B`) in integer math. White is `255` and black is `0`.
#[inline]
pub fn rgba_to_gray(p_rgba: &[u8]) -> u8 {
  let r = p_rgba[0] as u32;
  let g = p_rgba[1] as u32;
  let b = p_rgba[2] as u32;
  (((299 * r + 587 * g + 114 * b) + 500) / 1000) as u8
}

#[cfg(test)]
mod tests {
  use super::*;
  use primitives::Color;

  #[test]
  fn a_plane_holds_one_byte_per_pixel() {
    let plane = GrayPlane::from_image(&Image::new_from_color(20, 10, Color::from_rgba(255, 255, 255, 255)));

    assert_eq!(plane.dimensions::<u32>(), (20, 10));
    assert_eq!(plane.values().len(), 200);
    assert_eq!(plane.get(19, 9), Some(255));
    assert_eq!(plane.get(20, 0), None);
  }

  #[test]
  fn an_image_round_trips_through_a_plane() {
    let mut image = Image::new_from_color(2, 1, Color::from_rgba(255, 255, 255, 255));
    image.set_pixel(0, 0, (40, 40, 40, 255));
    let plane = GrayPlane::from_image(&image);

    assert_eq!(plane.values(), &[40, 255]);
    assert_eq!(plane.to_image().rgba(), &[40, 40, 40, 255, 255, 255, 255, 255]);
  }

  #[test]
  fn clones_share_their_values() {
    let plane = GrayPlane::new(2, 2, vec![1, 2, 3, 4]);

    assert!(plane.shares_values_with(&plane.clone()));
    assert!(!plane.shares_values_with(&GrayPlane::new(2, 2, vec![1, 2, 3, 4])));
  }
}
