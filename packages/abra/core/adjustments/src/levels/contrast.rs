use abra_core::{
  Image,
  image::gpu::{GpuPass, GpuProcessor},
};

use options::{Effect, Options};

use rayon::prelude::*;

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
  /// The factor applied around mid gray, the same formula the CPU uses.
  fn factor(&self) -> f32 {
    let amount = self.amount.clamp(-100.0, 100.0) as f32;
    (259.0 * (amount + 255.0)) / (255.0 * (259.0 - amount))
  }
}

impl GpuProcessor for Contrast {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![GpuPass::new(include_str!("./contrast.wgsl"), self.factor().to_le_bytes().to_vec())]
  }
}

impl Effect for Contrast {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    1
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_contrast(p_image, self.amount);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
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

  #[test]
  fn gpu_matches_cpu_within_one_byte_for_contrast() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let pixels: Vec<u8> =
      (0..65536u32).flat_map(|i| [i as u8, (i >> 8) as u8, (i * 37 % 256) as u8, (i * 13 % 256) as u8]).collect();
    for amount in [0.0, 50.0, -50.0, 100.0, -100.0] {
      let effect = contrast(amount);
      let actual = renderer.process(&[effect.passes(256, 256)], 256, 256, &pixels)?;
      let mut expected = Image::new_from_pixels(256, 256, pixels.clone(), abra_core::Channels::RGBA);
      effect.apply_on_cpu(&mut expected);
      for (index, (&actual, &expected)) in actual.iter().zip(expected.rgba().iter()).enumerate() {
        let tolerance = if index % 4 == 3 { 0 } else { 1 };
        assert!(
          (actual as i16 - expected as i16).abs() <= tolerance,
          "byte {index}, amount={amount}: {actual} != {expected}"
        );
      }
    }
    Ok(())
  }
}
