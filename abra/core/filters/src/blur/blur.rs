use crate::common::*;

use crate::kernel::apply_kernel;

/// Blurs an image using the blur algorithm.
fn apply_blur(p_image: &mut Image) {
  #[rustfmt::skip]
  let kernel = vec![
    0.0625, 0.125, 0.0625,
    0.125, 0.25, 0.125,
    0.0625, 0.125, 0.0625
  ];
  apply_kernel(p_image, &kernel);
}

/// Applies a blur to to an image.
/// - `p_image`: The image to be blurred.
/// - `p_options`: Additional options for applying the blur.
#[derive(Default, Clone)]
pub struct Blur {
  options: Options,
}

options::cpu_processor!(Blur);

impl Apply for Blur {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    let options = self.options.clone();
    apply_filter!(apply_blur, image, options, 1);
  }
}

pub fn blur() -> Blur {
  Blur::default()
}
