use abra_core::{Image, ImageRef};
use options::{Apply, Options};

use crate::apply_adjustment;

/// Converts an image to grayscale
fn apply_grayscale(p_image_ref: &mut Image) {
  p_image_ref.mut_pixels(|mut pixel| {
    let r = pixel[0] as f32;
    let g = pixel[1] as f32;
    let b = pixel[2] as f32;

    // Map the pixel to a grayscale value.
    let gray = (r * 0.299 + g * 0.587 + b * 0.114) as u8;

    // Set the pixel to the grayscale value.
    pixel[0] = gray;
    pixel[1] = gray;
    pixel[2] = gray;
  });
}

#[derive(Default, Clone)]
pub struct Grayscale {
  options: Options,
}

options::cpu_processor!(Grayscale);

impl Apply for Grayscale {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    apply_adjustment!(apply_grayscale, image, self.options.as_ref(), 1);
  }
}

pub fn grayscale() -> Grayscale {
  Grayscale::default()
}
