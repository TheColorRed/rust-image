use crate::image::Image;

/// Adjust the brightness of an image.
/// * `image` - The image.
/// * `amount` - The amount in which to increase or decrease the brightness.
pub fn brightness(image: &mut Image, mut amount: i32) {
  amount = amount.clamp(-150, 150);
  image.mut_channels_rgb(|x| (x as i32 + amount).clamp(0, 255) as u8);
}
