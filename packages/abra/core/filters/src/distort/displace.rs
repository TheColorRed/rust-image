use crate::common::*;
use abra_core::Channels;
use abra_core::transform::*;
use rayon::prelude::*;

/// How bright a pixel of a displacement map is, from `0.0` for black to `1.0` for white, using the Rec. 709 weights.
fn brightness(p_pixel: [u8; 4]) -> f32 {
  (0.2126 * p_pixel[0] as f32 + 0.7152 * p_pixel[1] as f32 + 0.0722 * p_pixel[2] as f32) / 255.0
}

fn apply_displace(p_image: &mut Image, p_map: &Image, p_amount_x: f32, p_amount_y: f32) {
  if p_amount_x == 0.0 && p_amount_y == 0.0 {
    return;
  }
  let (width, height) = p_image.dimensions();
  let (map_width, map_height) = p_map.dimensions::<u32>();
  if width == 0 || height == 0 || map_width == 0 || map_height == 0 {
    return;
  }
  // A map of another size is stretched over the image.
  let scale_x = map_width as f32 / width as f32;
  let scale_y = map_height as f32 / height as f32;

  let original_image = p_image.clone();
  let mut new_pixels = vec![0u8; (width * height * 4) as usize];

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = (i as u32 % width) as f32;
    let y = (i as u32 / width) as f32;

    // Mid gray moves nothing, white moves a pixel by the full amount one way and black by the full amount the other way.
    let map_pixel = sample(p_map, (x + 0.5) * scale_x - 0.5, (y + 0.5) * scale_y - 0.5, Interpolation::Bilinear, EdgeMode::Clamp);
    let push = (brightness(map_pixel) - 0.5) * 2.0;

    let src_x = x - push * p_amount_x;
    let src_y = y - push * p_amount_y;
    chunk.copy_from_slice(&sample(&original_image, src_x, src_y, Interpolation::Bicubic, EdgeMode::Clamp));
  });

  p_image.set_pixels(width, height, new_pixels, Channels::RGBA);
}

/// A displacement of an image by the brightness of another image. Create one with [`displace`].
#[derive(Clone)]
pub struct Displace {
  map: Image,
  amount_x: f32,
  amount_y: f32,
  options: Options,
}
impl Effect for Displace {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    self.amount_x.abs().max(self.amount_y.abs()).ceil() as i32 + 2
  }

  fn positional(&self) -> bool {
    true
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_displace(p_image, &self.map, self.amount_x, self.amount_y);
  }
}

/// Moves the pixels of the image by the brightness of another image, the way Photoshop's Displace filter does. Where the map is
/// mid gray nothing moves, where it is white the content moves by the full amount along each axis, and where it is black it
/// moves the same distance the other way, so a height map bends whatever is drawn on the image to follow it. A map of a
/// different size is stretched over the image, and pixels pulled in from past an edge take the nearest edge pixel.
/// - `p_map`: The image whose brightness says how far each pixel moves.
/// - `p_amount_x`: The most a pixel moves sideways, in pixels. Positive moves content to the right where the map is bright.
/// - `p_amount_y`: The most a pixel moves up or down, in pixels. Positive moves content down where the map is bright.
pub fn displace(p_map: &Image, p_amount_x: f32, p_amount_y: f32) -> Displace {
  Displace {
    map: p_map.clone(),
    amount_x: p_amount_x,
    amount_y: p_amount_y,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn gray(p_width: u32, p_height: u32, p_value: u8) -> Image {
    Image::new_from_pixels(p_width, p_height, &[p_value, p_value, p_value, 255].repeat((p_width * p_height) as usize), Channels::RGBA)
  }

  /// A row of pixels whose red channel is the pixel's column, so a move shows as a change of value.
  fn ramp(p_width: u32, p_height: u32) -> Image {
    let bytes: Vec<u8> = (0..p_width * p_height).flat_map(|i| [(i % p_width * 10) as u8, 0, 0, 255]).collect();
    Image::new_from_pixels(p_width, p_height, &bytes, Channels::RGBA)
  }

  #[test]
  fn mid_gray_moves_nothing() {
    let mut image = ramp(20, 4);
    let before = image.rgba().to_vec();
    displace(&gray(20, 4, 128), 5.0, 5.0).cpu_processor(&mut image);
    let worst = image.rgba().iter().zip(&before).map(|(&a, &b)| (a as i32 - b as i32).abs()).max().unwrap();
    assert!(worst <= 1, "moved by {worst}");
  }

  #[test]
  fn white_moves_content_right_and_black_moves_it_left() {
    let mut right = ramp(20, 4);
    displace(&gray(20, 4, 255), 3.0, 0.0).cpu_processor(&mut right);
    // Content moved 3 to the right, so column 10 now holds what was in column 7.
    assert_eq!(right.rgba()[10 * 4], 70);

    let mut left = ramp(20, 4);
    displace(&gray(20, 4, 0), 3.0, 0.0).cpu_processor(&mut left);
    assert_eq!(left.rgba()[10 * 4], 130);
  }

  #[test]
  fn a_map_of_another_size_is_stretched_over_the_image() {
    let mut image = ramp(20, 4);
    displace(&gray(2, 2, 255), 3.0, 0.0).cpu_processor(&mut image);
    assert_eq!(image.rgba()[10 * 4], 70);
  }
}
