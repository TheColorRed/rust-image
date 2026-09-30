use crate::common::*;
use abra_core::Channels;
use abra_core::transform::*;
use rayon::prelude::*;

fn apply_pinch(p_image: &mut Image, p_amount: f32) {
  let (width, height) = p_image.dimensions();
  let center_x = width as f32 / 2.0;
  let center_y = height as f32 / 2.0;
  let max_radius = center_x.min(center_y);

  // Scale down the amount to make the effect less intense
  let amount = p_amount * 0.5;

  let original_image = p_image.clone();
  let original_pixels = original_image.rgba();
  let mut new_pixels = vec![0u8; (width * height * 4) as usize];

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = (i as u32) % width;
    let y = (i as u32) / width;

    let dx = x as f32 - center_x;
    let dy = y as f32 - center_y;
    let distance = (dx * dx + dy * dy).sqrt();

    if distance < max_radius {
      let r = distance / max_radius;
      let theta =
        if amount > 0.0 { r.powf(1.0 - amount) * max_radius } else { r.powf(1.0 + amount.abs()) * max_radius };

      let scale = if distance == 0.0 { 1.0 } else { theta / distance };
      let src_x = (center_x + dx * scale).clamp(0.0, (width - 1) as f32);
      let src_y = (center_y + dy * scale).clamp(0.0, (height - 1) as f32);

      // Use bilinear interpolation for smoother results
      let pixel = sample(&original_image, src_x, src_y, Interpolation::Bilinear, EdgeMode::Transparent);
      chunk.copy_from_slice(&pixel);
    } else {
      // Copy original pixel for areas outside the effect radius
      let idx = (y * width + x) as usize * 4;
      chunk.copy_from_slice(&original_pixels[idx..idx + 4]);
    }
  });

  p_image.set_pixels(width, height, new_pixels, Channels::RGBA);
}

/// Applies a pinch distortion effect to the image.
/// - `p_image`: The image to apply the effect to.
/// - `p_amount`: The amount of pinch effect to apply. Positive values pinch inward, negative values bulge outward.
/// - `p_apply_options`: Options to specify for the filter.
#[derive(Clone)]
pub struct Pinch {
  amount: f32,
  options: Options,
}
options::cpu_processor!(Pinch);

impl Apply for Pinch {
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
    apply_filter!(apply_pinch, image, options, 1, self.amount);
  }
}
pub fn pinch(p_amount: f32) -> Pinch {
  Pinch {
    amount: p_amount.clamp(-1.0, 1.0),
    options: None,
  }
}
