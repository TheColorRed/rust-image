use abra_core::image::gpu::{GpuPass, GpuProcessor};
use abra_core::{Image, hsl_to_rgb, rgb_to_hsl};
use options::{Effect, Options};
use rayon::prelude::*;

/// Adjust the hue of an image, in degrees. A zero amount makes no change; positive and negative amounts rotate the hue
/// around the color wheel. The alpha channel is left alone.
fn apply_hue(p_image: &mut Image, p_amount: i32) {
  let amount = p_amount.rem_euclid(360) as f32;
  let (width, height) = p_image.dimensions::<i32>();
  let src = p_image.rgba();
  let mut out = vec![0u8; (width * height * 4) as usize];

  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let i = idx * 4;
    let (h, s, l) = rgb_to_hsl(src[i], src[i + 1], src[i + 2]);
    let (r, g, b) = hsl_to_rgb((h + amount).rem_euclid(360.0), s, l);
    dst_px.copy_from_slice(&[r, g, b, src[i + 3]]);
  });

  p_image.set_rgba(out);
}

/// A hue adjustment. Create one with [`hue`].
#[derive(Clone)]
pub struct Hue {
  amount: i32,
  options: Options,
}

impl Effect for Hue {
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
    apply_hue(p_image, self.amount);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for Hue {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    let amount = self.amount.rem_euclid(360);
    vec![GpuPass::new(include_str!("./hue.wgsl"), amount.to_le_bytes().to_vec())]
  }
}

/// Adjust the hue of an image by `p_amount` degrees. The supplied amount is preserved and wraps around the color wheel.
pub fn hue(p_amount: i32) -> Hue {
  Hue {
    amount: p_amount,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;

  #[test]
  fn rotates_hues_and_preserves_alpha() {
    let mut image = Image::new_from_pixels(3, 1, vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255], Channels::RGBA);

    hue(120).apply_on_cpu(&mut image);

    assert_eq!(image.rgba(), &[0, 255, 0, 255, 0, 0, 255, 255, 255, 0, 0, 255]);
  }

  #[test]
  fn negative_and_full_turn_amounts_wrap() {
    let pixels = [255, 0, 0, 73];
    let mut negative = Image::new_from_pixels(1, 1, pixels.to_vec(), Channels::RGBA);
    let mut equivalent = Image::new_from_pixels(1, 1, pixels.to_vec(), Channels::RGBA);
    hue(-60).apply_on_cpu(&mut negative);
    hue(300).apply_on_cpu(&mut equivalent);
    assert_eq!(negative.rgba(), equivalent.rgba());

    let mut full_turn = Image::new_from_pixels(1, 1, pixels.to_vec(), Channels::RGBA);
    hue(360).apply_on_cpu(&mut full_turn);
    assert_eq!(full_turn.rgba(), &pixels);
  }

  #[test]
  fn gpu_hue_matches_cpu() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};

    let width = 32;
    let height = 16;
    let pixels: Vec<u8> = (0..width * height)
      .flat_map(|pixel| {
        [
          (pixel * 31) as u8,
          (pixel * 17) as u8,
          (pixel * 7) as u8,
          (pixel * 13) as u8,
        ]
      })
      .collect();
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);

    for amount in [-360, -120, -1, 0, 1, 45, 120, 359, 360, i32::MAX, i32::MIN] {
      let effect = hue(amount);
      let actual = renderer.process(&[effect.passes(width, height)], width, height, &pixels)?;
      let mut expected = Image::new_from_pixels(width, height, pixels.clone(), abra_core::Channels::RGBA);
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
