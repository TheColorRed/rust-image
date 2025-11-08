use crate::image::Image;

/// Reduces the opacity of an image by a factor of `opacity`.
/// The opacity is a value between 0.0 and 1.0.
pub fn reduce_opacity(image: &mut Image, opacity: f32) {
  let opacity = opacity.clamp(0.0, 1.0);
  image.mut_channel("a", |channel| (channel as f32 * opacity) as u8);
}
