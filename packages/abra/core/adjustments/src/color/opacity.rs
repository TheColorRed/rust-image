use abra_core::{Channel, Image};
use options::{Effect, Options};

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

impl Effect for Opacity {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    0
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_opacity(p_image, self.opacity);
  }
}

pub fn reduce_opacity(p_opacity: f32) -> Opacity {
  Opacity {
    opacity: p_opacity,
    options: None,
  }
}
