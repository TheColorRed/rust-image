use abra_core::{Image, ImageRef};
use options::{Apply, Options};

use crate::apply_adjustment;

fn apply_invert<'a>(p_image: &mut Image) {
  p_image.mut_channels_rgb(|channel| 255 - channel);
}

/// Inverts the colors of an image
#[derive(Default)]
pub struct Invert {
  options: Options,
}

impl Apply for Invert {
  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    apply_adjustment!(apply_invert, image, self.options.as_ref(), 1);
  }
}

pub fn invert() -> Invert {
  Invert::default()
}
