use abra_core::{Image, ImageRef};

use options::{Apply, Options};

use crate::apply_adjustment;
use abra_core::image::gpu::{CpuProcessor, GpuEffect, GpuOp, GpuPass, GpuProcessor};

/// Adjust the brightness of an image.
/// * `p_image` - The image.
/// * `p_amount` - The amount in which to increase or decrease the brightness.
fn apply_brightness(p_image: &mut Image, p_amount: f32) {
  // amount = amount.clamp(-150f32, 150f32);
  let _ = p_image * p_amount;
}

#[derive(Clone)]
pub struct Brightness {
  amount: i32,
  options: Options,
}

impl Brightness {
  /// The multiplicative factor the shader uses: 0 is black and 1.0 is no change. A positive amount increases
  /// brightness.
  fn factor(&self) -> f32 {
    ((self.amount as f32) / 100.0) + 1.0
  }

  fn gpu_op(&self) -> GpuOp {
    GpuOp::new(include_str!("./brightness.wgsl"), self.factor().to_le_bytes())
  }
}

impl CpuProcessor for Brightness {
  fn process(&self, p_image: &mut Image) {
    self.apply_to_image(p_image);
  }

  fn gpu(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for Brightness {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    self.gpu_op().passes(p_width, p_height)
  }
}

impl Apply for Brightness {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    apply_adjustment!(gpu = self; apply_brightness, image, self.options.as_ref(), 0, self.factor());
  }
}

pub fn brightness(p_amount: i32) -> Brightness {
  Brightness {
    amount: p_amount,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Image;
  use primitives::Color;

  #[test]
  fn brightness_gpu_provider_applies_brightness() {
    // Register a real GPU provider using a blocking context
    gpu::register();

    let mut img = Image::new_from_color(8, 8, Color::from_rgba(100, 0, 0, 255));
    // Apply brightness +50
    brightness(50).apply(&mut img);
    // Expect red component increased (since factor = 1.5)
    let r = img.rgba()[0];
    assert!(r > 100);

    // Clear provider so other tests are unaffected
    abra_core::image::gpu::clear_gpu_provider();
  }
}
