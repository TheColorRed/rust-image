use crate::image::Image;

/// Adjust the saturation of an image where 0.0 is grayscale and 100.0 is maximum saturation.
/// - `image` - The image to adjust.
/// - `value` - The value to adjust the saturation by. The range is [-100, 100] where 0 means no change.
pub fn saturation(image: &mut Image, value: i32) {
  let value = (value as f32).clamp(-100.0, 100.0);
  let value = (value / 100.0) + 1.0; // Scale value to range [0, 2] where 1.0 means no change

  image.mut_pixels_simd(|mut pixel| {
    let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);
    let gray = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
    pixel[0] = (gray + (r as f32 - gray) * value).round() as u8;
    pixel[1] = (gray + (g as f32 - gray) * value).round() as u8;
    pixel[2] = (gray + (b as f32 - gray) * value).round() as u8;
    pixel[3] = a;
  });
}
