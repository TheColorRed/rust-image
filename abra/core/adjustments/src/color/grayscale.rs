use abra_core::{Image, ImageRef};
use options::Options;

use crate::apply_adjustment;

/// Converts an image to grayscale
fn apply_grayscale(image_ref: &mut Image) {
  image_ref.mut_pixels(|mut pixel| {
    let r = pixel[0] as f32;
    let g = pixel[1] as f32;
    let b = pixel[2] as f32;

    // Map the pixel to a grayscale value.
    let gray = (r * 0.299 + g * 0.587 + b * 0.114) as u8;

    // Set the pixel to the grayscale value.
    pixel[0] = gray;
    pixel[1] = gray;
    pixel[2] = gray;
  });
}

pub fn grayscale<'a>(image: impl Into<ImageRef<'a>>, p_options: impl Into<Options>) {
  let mut image_ref: ImageRef = image.into();
  let image = &mut image_ref as &mut Image;
  apply_adjustment!(apply_grayscale, image, p_options, 1);
}
