use crate::common::*;

#[napi]
/// Generates a transparent checkerboard pattern image to be used as a background for empty layers.
/// @param width The width of the image.
/// @param height The height of the image.
/// @returns An ImageData object containing the checkerboard pattern.
pub fn get_empty_layer_background(width: u32, height: u32) -> ImageData {
  let img = alakazam::generate_image::transparent_pattern(width, height, 8);
  img.into()
}
