use abra_core::{Image, ImageRef};

/// Inverts the colors of an image
pub fn invert<'a>(image: impl Into<ImageRef<'a>>) {
  let mut image_ref: ImageRef = image.into();
  let image = &mut image_ref as &mut Image;
  image.mut_channels_rgb(|channel| 255 - channel);
}
