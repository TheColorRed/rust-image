use crate::image::Image;

/// Converts an image to grayscale
pub fn grayscale(image: &mut Image) {
  image.mut_pixels(|mut pixel| {
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
