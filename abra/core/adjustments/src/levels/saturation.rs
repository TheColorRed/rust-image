use abra_core::{Image, ImageRef};

use options::{Apply, Options};

use crate::apply_adjustment;

/// Adjust the saturation of an image where 0.0 is grayscale and 100.0 is maximum saturation.
/// - `p_image` - The image to adjust.
/// - `p_value` - The value to adjust the saturation by. The range is [-100, 100] where 0 means no change.
fn apply_saturation(p_image: &mut Image, p_value: i32) {
  let p_value = (p_value as f32).clamp(-100.0, 100.0);
  let p_value = (p_value / 100.0) + 1.0; // Scale value to range [0, 2] where 1.0 means no change

  p_image.mut_pixels_simd(|mut pixel| {
    let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);
    let gray = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
    pixel[0] = (gray + (r as f32 - gray) * p_value).round() as u8;
    pixel[1] = (gray + (g as f32 - gray) * p_value).round() as u8;
    pixel[2] = (gray + (b as f32 - gray) * p_value).round() as u8;
    pixel[3] = a;
  });
}

pub struct Saturation {
  amount: i32,
  options: Options,
}

impl Apply for Saturation {
  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    apply_adjustment!(apply_saturation, image, self.options.as_ref(), 0, self.amount);
  }
}

pub fn saturation(p_amount: i32) -> Saturation {
  Saturation {
    amount: p_amount,
    options: None,
  }
}
