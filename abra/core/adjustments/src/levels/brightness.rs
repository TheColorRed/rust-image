use abra_core::Image;

use options::{Effect, Options};

use crate::lut::channel_lut_pass;
use abra_core::image::gpu::{GpuPass, GpuProcessor};

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
  /// The multiplicative factor: 0 is black and 1.0 is no change. A positive amount increases
  /// brightness.
  fn factor(&self) -> f32 {
    ((self.amount as f32) / 100.0) + 1.0
  }
}

impl GpuProcessor for Brightness {
  /// A lookup table built from the CPU code, so the GPU gives exactly the CPU's pixels.
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![channel_lut_pass(|image| self.cpu_processor(image))]
  }
}

impl Effect for Brightness {
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
    apply_brightness(p_image, self.factor());
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
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
