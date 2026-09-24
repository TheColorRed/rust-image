use abra_core::{Image, ImageRef};

use options::{Apply, Options};

use crate::apply_adjustment;
use abra_core::image::gpu_op::{GpuOp, clear_gpu_op, set_gpu_op};

/// Adjust the brightness of an image.
/// * `p_image` - The image.
/// * `p_amount` - The amount in which to increase or decrease the brightness.
fn apply_brightness(p_image: &mut Image, p_amount: f32) {
  // amount = amount.clamp(-150f32, 150f32);
  let _ = p_image * p_amount;
}

pub struct Brightness {
  amount: i32,
  options: Options,
}

impl Apply for Brightness {
  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    // Convert integer amount to a multiplicative brightness factor for the shader.
    // CPU path uses additive amount; here we convert to a scale factor where
    // 0 means black and 1.0 means no change. A positive amount increases brightness.
    let amount = ((self.amount as f32) / 100.0) + 1.0;
    set_gpu_op(include_str!("./brightness.wgsl"), GpuOp::Brightness(amount));
    apply_adjustment!(apply_brightness, image, self.options.as_ref(), 0, amount);
    clear_gpu_op();
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
  use std::sync::Arc;

  #[test]
  fn brightness_gpu_provider_applies_brightness() {
    // Register a real GPU provider using a blocking context
    let ctx = Arc::new(gpu::context::GpuContext::new_default_blocking().expect("GpuContext init failed"));
    gpu::register_gpu_context(ctx);

    let mut img = Image::new_from_color(8, 8, Color::from_rgba(100, 0, 0, 255));
    // Apply brightness +50
    brightness(50).apply(&mut img);
    // Expect red component increased (since factor = 1.5)
    let r = img.rgba()[0];
    assert!(r > 100);

    // Clear provider so other tests are unaffected
    abra_core::image::gpu_registry::clear_gpu_provider();
  }
}
