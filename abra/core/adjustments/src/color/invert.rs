use abra_core::{Image, ImageRef};
use options::Options;

use crate::apply_adjustment;

fn apply_invert<'a>(image: &mut Image) {
  image.mut_channels_rgb(|channel| 255 - channel);
}

/// Inverts the colors of an image
pub fn invert<'a>(image: impl Into<ImageRef<'a>>, p_options: impl Into<Options>) {
  let mut image_ref: ImageRef = image.into();
  let image = &mut image_ref as &mut Image;
  apply_adjustment!(apply_invert, image, p_options, 1);
}
