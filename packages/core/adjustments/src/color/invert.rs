use core::Image;

/// Inverts the colors of an image
pub fn invert(image: &mut Image) {
  image.mut_channels_rgb(|channel| 255 - channel);
}
