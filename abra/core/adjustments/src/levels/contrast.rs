use abra_core::{
  Image, ImageRef,
  image::gpu::{CpuProcessor, GpuEffect, GpuOp, GpuPass, GpuProcessor},
};

use options::{Apply, Options};

use rayon::prelude::*;

use crate::apply_adjustment;

/// Adjusts the contrast of an image.
fn apply_contrast(p_image: &mut Image, p_amount: impl Into<f64>) {
  let p_amount = p_amount.into().clamp(-100.0, 100.0) as f32;
  // Use floating point math for the contrast factor to avoid integer truncation.
  let factor = (259.0 * (p_amount + 255.0)) / (255.0 * (259.0 - p_amount));
  let colors = p_image.colors();
  let slice = colors.as_slice_mut().expect("Image colors must be contiguous");
  slice.par_chunks_mut(4096).for_each(|chunk| {
    for i in (0..chunk.len()).step_by(4) {
      let pixel = &mut chunk[i..i + 4];
      pixel[0] = (factor * (pixel[0] as f32 - 128.0) + 128.0).clamp(0.0, 255.0) as u8;
      pixel[1] = (factor * (pixel[1] as f32 - 128.0) + 128.0).clamp(0.0, 255.0) as u8;
      pixel[2] = (factor * (pixel[2] as f32 - 128.0) + 128.0).clamp(0.0, 255.0) as u8;
    }
  });
}

#[derive(Clone)]
pub struct Contrast {
  amount: f64,
  options: Options,
}

impl Contrast {
  fn gpu_op(&self) -> GpuOp {
    GpuOp::new(include_str!("./contrast.wgsl"), (self.amount as f32).to_le_bytes())
  }
}

impl CpuProcessor for Contrast {
  fn process(&self, p_image: &mut Image) {
    self.apply_to_image(p_image);
  }

  fn gpu(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for Contrast {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    self.gpu_op().passes(p_width, p_height)
  }
}

impl Apply for Contrast {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    apply_adjustment!(gpu = self; apply_contrast, image, self.options.as_ref(), 1, self.amount);
  }
}

pub fn contrast(p_amount: impl Into<f64>) -> Contrast {
  Contrast {
    amount: p_amount.into(),
    options: None,
  }
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
