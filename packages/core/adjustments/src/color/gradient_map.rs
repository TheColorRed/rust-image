use abra_core::{Gradient, Image};

/// Apply a gradient map to an image. This will map the colors of the image to the colors of the gradient.
/// Darker colors will be mapped to the first color in the gradient, and lighter colors will be mapped to the last color in the gradient.
pub fn gradient_map(image: &mut Image, gradient: Gradient) {
  image.mut_pixels(|mut pixel| {
    let r = pixel[0] as f32;
    let g = pixel[1] as f32;
    let b = pixel[2] as f32;

    // Map the pixel to a grayscale value.
    let gray = r * 0.299 + g * 0.587 + b * 0.114;
    // Normalize the grayscale value to a value between 0 and 1.
    let time = gray / 255.0;
    // Get the color from the gradient at the normalized time.
    let (r, g, b, _) = gradient.get_color(time);

    pixel[0] = r;
    pixel[1] = g;
    pixel[2] = b;
  });
}

/// Apply a gradient map to an image, but reverse the gradient.
pub fn gradient_map_reverse(image: &mut Image, gradient: Gradient) {
  gradient_map(image, gradient.clone().reverse());
}
