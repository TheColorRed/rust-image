use abra_core::{
  Image,
  image::gpu::{GpuPass, GpuProcessor},
};
use options::{Effect, Options};
use rayon::prelude::*;

/// A channel mixer: each color channel of the result is a weighted sum of the red, green and blue channels of the pixel.
///
/// It can move color between channels, which no per-channel adjustment can: make red from green and blue, swap channels, or
/// build a gray from chosen weights. The alpha is left alone. Create one with [`channel_mix`].
///
/// The GPU version (`channel_mix.wgsl`) does the same sum in the same order. It is not a table for each channel, as other
/// color adjustments use, because each result channel depends on all three source channels.
#[derive(Clone)]
pub struct ChannelMix {
  /// The weights of the red, green and blue channels of the result, each as `[red, green, blue]` of the source.
  weights: [[f32; 3]; 3],
  options: Options,
}

impl Effect for ChannelMix {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    0
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let pixels = p_image.colors().as_slice_mut().expect("Image colors must be contiguous");
    pixels.par_chunks_exact_mut(4).for_each(|pixel| {
      let source = [pixel[0] as f32, pixel[1] as f32, pixel[2] as f32];
      for (channel, weights) in self.weights.iter().enumerate() {
        let mixed = weights[0] * source[0] + weights[1] * source[1] + weights[2] * source[2];
        pixel[channel] = mixed.round().clamp(0.0, 255.0) as u8;
      }
    });
  }
}

impl GpuProcessor for ChannelMix {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    // Three rows of four floats, as the shader's `Weights` struct lays them out; the fourth of each row is unused.
    let uniforms: Vec<u8> =
      self.weights.iter().flat_map(|row| [row[0], row[1], row[2], 0.0]).flat_map(f32::to_le_bytes).collect();
    vec![GpuPass::new(include_str!("./channel_mix.wgsl"), uniforms)]
  }
}

/// A channel mixer. Each argument gives the weights of the source's `[red, green, blue]` for one channel of the result, so
/// `[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]` changes nothing. Results are clamped to `0..=255`.
/// - `p_red`: The weights that make the red channel.
/// - `p_green`: The weights that make the green channel.
/// - `p_blue`: The weights that make the blue channel.
pub fn channel_mix(p_red: [f32; 3], p_green: [f32; 3], p_blue: [f32; 3]) -> ChannelMix {
  ChannelMix {
    weights: [p_red, p_green, p_blue],
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;
  use gpu::{GpuContext, LiveRenderer};

  fn mixed(p_mix: ChannelMix, p_pixels: Vec<u8>) -> Vec<u8> {
    let mut image = Image::new_from_pixels(p_pixels.len() as u32 / 4, 1, p_pixels, Channels::RGBA);
    p_mix.apply_on_cpu(&mut image);
    image.rgba().to_vec()
  }

  #[test]
  fn the_identity_changes_nothing() {
    let pixels = vec![10, 200, 30, 255, 0, 0, 0, 7];
    assert_eq!(mixed(channel_mix([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), pixels.clone()), pixels);
  }

  #[test]
  fn a_channel_can_be_made_from_the_others() {
    // Red from half of green and half of blue, as a flash-red pupil would be fixed: (220, 40, 30) -> (35, 40, 30).
    let result = mixed(channel_mix([0.0, 0.5, 0.5], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), vec![220, 40, 30, 255]);
    assert_eq!(result, vec![35, 40, 30, 255]);
  }

  #[test]
  fn channels_can_be_swapped_and_the_alpha_is_kept() {
    let result = mixed(channel_mix([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]), vec![1, 2, 3, 99]);
    assert_eq!(result, vec![3, 2, 1, 99]);
  }

  #[test]
  fn results_are_clamped_to_the_range_of_a_byte() {
    let result = mixed(channel_mix([2.0, 2.0, 2.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 0.0]), vec![200, 100, 50, 255]);
    assert_eq!(result, vec![255, 0, 0, 255]);
  }

  #[test]
  fn the_gpu_gives_the_same_pixels_as_the_cpu() -> anyhow::Result<()> {
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let pixels: Vec<u8> =
      (0..65536u32).flat_map(|i| [i as u8, (i >> 8) as u8, (i * 37 % 256) as u8, (i * 13 % 256) as u8]).collect();
    // The weights are exact in a float, so the two must agree to the byte; the other set is not, so it may differ by one.
    for (weights, tolerance) in [
      (([0.0, 0.5, 0.5], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), 0),
      (([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]), 0),
      (([0.33, 0.59, 0.11], [-0.2, 1.4, 0.3], [0.6, 0.1, 2.0]), 1),
    ] {
      let effect = channel_mix(weights.0, weights.1, weights.2);
      let actual = renderer.process(&[effect.passes(256, 256)], 256, 256, &pixels)?;
      let mut expected = Image::new_from_pixels(256, 256, pixels.clone(), Channels::RGBA);
      effect.apply_on_cpu(&mut expected);
      for (index, (&actual, &expected)) in actual.iter().zip(expected.rgba().iter()).enumerate() {
        let tolerance = if index % 4 == 3 { 0 } else { tolerance };
        assert!((actual as i16 - expected as i16).abs() <= tolerance, "byte {index}, {weights:?}: {actual} != {expected}");
      }
    }
    Ok(())
  }
}
