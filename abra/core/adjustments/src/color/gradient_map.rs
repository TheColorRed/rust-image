use abra_core::{Gradient, Image, ImageRef};
use options::{Apply, Options};

use crate::apply_adjustment;

fn apply_gradient_map(p_image: &mut Image, p_gradient: &Gradient) {
  p_image.mut_pixels(|mut pixel| {
    let r = pixel[0] as f32;
    let g = pixel[1] as f32;
    let b = pixel[2] as f32;

    // Map the pixel to a grayscale value.
    let gray = r * 0.299 + g * 0.587 + b * 0.114;
    // Normalize the grayscale value to a value between 0 and 1.
    let time = gray / 255.0;
    // Get the color from the gradient at the normalized time.
    let (r, g, b, _) = p_gradient.color_at(time).rgba();

    pixel[0] = r;
    pixel[1] = g;
    pixel[2] = b;
  });
}

#[derive(Clone)]
pub struct GradientMap {
  gradient: Gradient,
  options: Options,
}

options::cpu_processor!(GradientMap);

impl Apply for GradientMap {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    apply_adjustment!(apply_gradient_map, image, self.options.as_ref(), 1, &self.gradient);
  }
}

/// Maps image luminance to the supplied gradient.
pub fn gradient_map(p_gradient: impl Into<Gradient>) -> GradientMap {
  GradientMap {
    gradient: p_gradient.into(),
    options: None,
  }
}
