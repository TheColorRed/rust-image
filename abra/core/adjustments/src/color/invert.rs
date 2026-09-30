use abra_core::{Channel, Image, ImageRef};
use options::{Apply, Options};

use crate::apply_adjustment;

fn apply_invert<'a>(p_image: &mut Image) {
  p_image.mut_channels(Channel::RGB, |channel| 255 - channel);
}

/// Inverts the colors of an image
#[derive(Default, Clone)]
pub struct Invert {
  options: Options,
}

options::cpu_processor!(Invert);

impl Apply for Invert {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    apply_adjustment!(apply_invert, image, self.options.as_ref(), 1);
  }
}

pub fn invert() -> Invert {
  Invert::default()
}
