use abra_core::{
  Image,
  image::gpu_op::{GpuOp, clear_gpu_op, set_gpu_op},
};

use options::Options;

use crate::apply_adjustment;

/// Adjusts the contrast of an image.
fn apply_contrast(image: &mut Image, amount: impl Into<f64>) {
  let amount = amount.into().clamp(-100.0, 100.0) as f32;
  // Use floating point math for the contrast factor to avoid integer truncation.
  let factor = (259.0 * (amount + 255.0)) / (255.0 * (259.0 - amount));
  image.mut_channels_rgb(|x| {
    let v = factor * (x as f32 - 128.0) + 128.0;
    v.clamp(0.0, 255.0) as u8
  });
}

pub fn contrast(image: &mut Image, amount: impl Into<f64>, p_apply_options: impl Into<Options>) {
  let amount = amount.into();
  set_gpu_op(include_str!("./contrast.wgsl"), GpuOp::Contrast(amount as f32));
  apply_adjustment!(apply_contrast, image, p_apply_options, 1, amount);
  clear_gpu_op();
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn contrast_changes_pixels_with_50() {
    let mut img = Image::new(2u32, 2u32);
    // set a pixel to a value different from the pivot (128) so contrast changes it
    img.set_pixel(0, 0, (100u8, 100u8, 100u8, 255u8));
    // Call the private implementation directly; it uses the same logic as the public
    // wrapper and lets us avoid ApplyOptions complexity in the test.
    apply_contrast(&mut img, 50);
    let p = img.get_pixel(0, 0).unwrap();
    // With contrast 50, the value should not remain unchanged (and should be darker than 100).</n+    assert!(p.0 != 100 || p.1 != 100 || p.2 != 100, "Pixel didn't change with contrast 50");
    assert!(p.0 < 100 && p.1 < 100 && p.2 < 100, "Pixel didn't become darker as expected");
  }
}
