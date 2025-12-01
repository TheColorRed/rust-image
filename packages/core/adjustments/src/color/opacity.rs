use abra_core::{Image, ImageRef};

/// Reduces the opacity of an image by a factor of `opacity`.
/// The opacity is a value between 0.0 and 1.0.
pub fn reduce_opacity<'a>(image: impl Into<ImageRef<'a>>, opacity: f32) {
  let mut image_ref: ImageRef = image.into();
  let image = &mut image_ref as &mut Image;
  let opacity = opacity.clamp(0.0, 1.0);
  image.mut_channel("a", |channel| (channel as f32 * opacity) as u8);
}
