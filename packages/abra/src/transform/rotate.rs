use std::time::Instant;

use crate::{
  image::Image,
  utils::debug::{DebugInfo, RotateMessage},
};

use rayon::prelude::*;

/// Calculate the new size of the image after rotation.
/// This is to resize the new image to fit the rotated image without cropping.
/// * `image` - The source image.
/// * `degrees` - The degrees to rotate the image.
fn calc_image_new_size(image: &Image, degrees: f32) -> (u32, u32) {
  let (width, height) = image.dimensions::<u32>();
  let mut size = vec![width, height];
  let mut degrees = degrees % 180.0;
  if degrees < 0.0 {
    degrees += 180.0
  }
  if degrees >= 90.0 {
    size = vec![size[1], size[0]];
    degrees -= 90.0;
  }

  if degrees == 0.0 {
    return (size[0], size[1]);
  }

  let radians = degrees.to_radians();
  let width = (size[0] as f32 * radians.cos() + size[1] as f32 * radians.sin()).abs() as u32;
  let height = (size[0] as f32 * radians.sin() + size[1] as f32 * radians.cos()).abs() as u32;

  (width, height)
}

/// Applies the rotation to the image by copying the pixels from the source image to the destination image
/// at the proper rotated position.
/// * `src` - The source image.
/// * `dest` - The destination image.
/// * `degrees` - The degrees to rotate the image.
/// * `width` - The new width of the image after rotation.
/// * `height` - The new height of the image after rotation.
fn apply_rotation(src: &mut Image, degrees: f32, width: u32, height: u32) {
  let (src_width, src_height) = src.dimensions::<usize>();
  let radians = degrees.to_radians();
  // let (src_width, src_height) = (src_width, src_height);

  let src_center_x = src_width as f32 / 2.0;
  let src_center_y = src_height as f32 / 2.0;
  let dest_center_x = width as f32 / 2.0;
  let dest_center_y = height as f32 / 2.0;
  // If degrees is positive rotate clockwise, if negative rotate counter-clockwise.
  let src_pixels = src.rgba();
  let mut pixels = vec![0; (width * height * 4) as usize];

  pixels.par_chunks_mut(4).enumerate().for_each(|(index, pixel)| {
    let x = index as u32 % width;
    let y = index as u32 / width;
    let src_x = ((x as f32 - dest_center_x) * radians.cos() + (y as f32 - dest_center_y) * radians.sin() + src_center_x) as i32;
    let src_y = (-(x as f32 - dest_center_x) * radians.sin() + (y as f32 - dest_center_y) * radians.cos() + src_center_y) as i32;

    if src_x >= 0 && src_x < src_width as i32 && src_y >= 0 && src_y < src_height as i32 {
      let src_index = (src_y * src_width as i32 + src_x) as usize;
      pixel[0] = src_pixels[src_index * 4];
      pixel[1] = src_pixels[src_index * 4 + 1];
      pixel[2] = src_pixels[src_index * 4 + 2];
      pixel[3] = src_pixels[src_index * 4 + 3];
    }
  });

  src.set_new_pixels(pixels, width, height);
}

/// Rotates the image by the specified number of degrees.\
/// The image will be resized to fit the rotated image without cropping.\
/// * `image` - The image to rotate.
/// * `degrees` - The number of degrees to rotate the image. Positive values rotate clockwise, negative values rotate counter-clockwise.
pub fn rotate(image: &mut Image, degrees: f32) {
  let duration = Instant::now();
  let (width, height) = calc_image_new_size(image, degrees);
  apply_rotation(image, degrees, width, height);
  let (new_width, new_height) = image.dimensions::<u32>();
  DebugInfo::Rotate(degrees, width, height, new_width, new_height, duration.elapsed()).log();
}

/// Rotates the image 90 degrees clockwise.
/// * `image` - The image to rotate.
pub fn rotate_90(image: &mut Image) {
  let duration = Instant::now();
  let (width, height) = image.dimensions::<u32>();
  rotate(image, 90.0);
  let (new_width, new_height) = image.dimensions::<u32>();
  DebugInfo::Rotate(90.0, width, height, new_width, new_height, duration.elapsed()).log();
}

/// Rotates the image 90 degrees counter-clockwise.
/// * `image` - The image to rotate.
pub fn rotate_90_ccw(image: &mut Image) {
  let duration = Instant::now();
  let (width, height) = image.dimensions::<u32>();
  rotate(image, -90.0);
  let (new_width, new_height) = image.dimensions::<u32>();
  DebugInfo::Rotate(-90.0, width, height, new_width, new_height, duration.elapsed()).log();
}

/// Rotates the image 180 degrees.
/// * `image` - The image to rotate.
pub fn rotate_180(image: &mut Image) {
  let duration = Instant::now();
  let (width, height) = image.dimensions::<u32>();
  rotate(image, 180.0);
  let (new_width, new_height) = image.dimensions::<u32>();
  DebugInfo::Rotate(180.0, width, height, new_width, new_height, duration.elapsed()).log();
}
