use abra_core::{Channel, Image, ImageRef};
use options::{Apply, Options};

use crate::apply_adjustment;

/// Reduces the opacity of an image by a factor of `p_opacity`.
/// The opacity is a value between 0.0 and 1.0.
fn apply_opacity(p_image: &mut Image, p_opacity: f32) {
  let p_opacity = p_opacity.clamp(0.0, 1.0);
  p_image.mut_channels([Channel::A], |channel| (channel as f32 * p_opacity) as u8);
}

#[derive(Clone)]
pub struct Opacity {
  opacity: f32,
  options: Options,
}

options::cpu_processor!(Opacity);

impl Apply for Opacity {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    apply_adjustment!(apply_opacity, image, self.options.as_ref(), 0, self.opacity);
  }
}

pub fn reduce_opacity(p_opacity: f32) -> Opacity {
  Opacity {
    opacity: p_opacity,
    options: None,
  }
}
