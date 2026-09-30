use abra_core::{
  Image, ImageRef,
  image::gpu::{CpuProcessor, GpuEffect, GpuOp, GpuPass, GpuProcessor},
};

use options::{Apply, Options};

use crate::apply_adjustment;

/// Adjust the saturation of an image where 0.0 is grayscale and 100.0 is maximum saturation.
/// - `p_image` - The image to adjust.
/// - `p_value` - The value to adjust the saturation by. The range is [-100, 100] where 0 means no change.
fn apply_saturation(p_image: &mut Image, p_value: i32) {
  let p_value = (p_value as f32).clamp(-100.0, 100.0);
  let p_value = (p_value / 100.0) + 1.0; // Scale value to range [0, 2] where 1.0 means no change

  p_image.mut_pixels(|mut pixel| {
    let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);
    let gray = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
    pixel[0] = (gray + (r as f32 - gray) * p_value).round() as u8;
    pixel[1] = (gray + (g as f32 - gray) * p_value).round() as u8;
    pixel[2] = (gray + (b as f32 - gray) * p_value).round() as u8;
    pixel[3] = a;
  });
}

#[derive(Clone)]
pub struct Saturation {
  amount: i32,
  options: Options,
}

impl Apply for Saturation {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    apply_adjustment!(apply_saturation, image, self.options.as_ref(), 0, self.amount);
  }
}

impl CpuProcessor for Saturation {
  fn process(&self, p_image: &mut Image) {
    self.apply_to_image(p_image);
  }

  fn gpu(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for Saturation {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let factor = (self.amount as f32).clamp(-100.0, 100.0) / 100.0 + 1.0;
    let op = GpuOp::new(include_str!("./saturation.wgsl"), factor.to_le_bytes());
    op.passes(p_width, p_height)
  }
}

pub fn saturation(p_amount: i32) -> Saturation {
  Saturation {
    amount: p_amount,
    options: None,
  }
}
