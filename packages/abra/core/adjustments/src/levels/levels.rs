use abra_core::{
  Image,
  image::gpu::{GpuPass, GpuProcessor},
};

use crate::lut::channel_lut_pass;
use options::{Effect, Options};

use rayon::prelude::*;

/// Maps one value, `p_value` from `0` to `255`, the way a levels adjustment does: the input range is stretched to fill `0..=1`, the
/// midtones are bent by the gamma, and the result is put into the output range.
fn level(p_value: f32, p_levels: &Levels) -> u8 {
  let width = p_levels.input_white - p_levels.input_black;
  // An input range with no width is a threshold at its point: everything below goes to the lowest output and the rest to the highest.
  let stretched = if width > 0.0 {
    ((p_value - p_levels.input_black) / width).clamp(0.0, 1.0)
  } else if p_value > p_levels.input_black {
    1.0
  } else {
    0.0
  };
  let bent = stretched.powf(1.0 / p_levels.gamma.max(f32::EPSILON));
  let result = p_levels.output_black + bent * (p_levels.output_white - p_levels.output_black);
  result.round().clamp(0.0, 255.0) as u8
}

/// A levels adjustment: the tones of an image are remapped by where its darkest and lightest tones are said to be, how bright its
/// middle is, and the range they are put in. Each of the red, green and blue channels is mapped on its own, and the alpha is left alone.
///
/// A value `v` becomes `output_black + ((v - input_black) / (input_white - input_black)) ^ (1 / gamma) * (output_white - output_black)`,
/// with the part in brackets held to `0..=1`. Pulling `input_black` up makes the shadows darker, pulling `input_white` down brightens
/// the highlights, a `gamma` above `1` brightens the midtones and below `1` darkens them, and the output range sets the darkest and
/// lightest the result can be. All the values are on the scale of a byte, `0` to `255`. Create one with [`levels`].
///
/// The GPU version is a lookup table built from this CPU code, so the two give the same pixels.
#[derive(Clone)]
pub struct Levels {
  input_black: f32,
  input_white: f32,
  gamma: f32,
  output_black: f32,
  output_white: f32,
  options: Options,
}

impl Levels {
  /// Sets the midtone gamma. `1` leaves the middle where it is; above `1` makes the middle brighter, below `1` darker. Defaults to `1`.
  pub fn with_gamma(mut self, p_gamma: f32) -> Self {
    self.gamma = p_gamma;
    self
  }

  /// Sets the range the result is put in. Defaults to `0` and `255`, which uses all of it.
  /// - `p_black`: The value that the darkest tone becomes.
  /// - `p_white`: The value that the lightest tone becomes.
  pub fn with_output(mut self, p_black: f32, p_white: f32) -> Self {
    self.output_black = p_black;
    self.output_white = p_white;
    self
  }
}

/// A levels adjustment of an input range.
/// - `p_input_black`: The value, `0` to `255`, that becomes the darkest tone: anything at or below it goes to the lowest output.
/// - `p_input_white`: The value, `0` to `255`, that becomes the lightest tone: anything at or above it goes to the highest output.
pub fn levels(p_input_black: f32, p_input_white: f32) -> Levels {
  Levels {
    input_black: p_input_black,
    input_white: p_input_white,
    gamma: 1.0,
    output_black: 0.0,
    output_white: 255.0,
    options: None,
  }
}

impl GpuProcessor for Levels {
  /// A lookup table built from the CPU code, so the GPU gives exactly the CPU's pixels.
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![channel_lut_pass(|image| self.cpu_processor(image))]
  }
}

impl Effect for Levels {
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
    // Each of the 256 values is mapped once, and the pixels are looked up.
    let table: Vec<u8> = (0..=255u32).map(|value| level(value as f32, self)).collect();
    let colors = p_image.colors();
    let slice = colors.as_slice_mut().expect("Image colors must be contiguous");
    slice.par_chunks_mut(4096).for_each(|chunk| {
      for pixel in chunk.chunks_exact_mut(4) {
        for channel in &mut pixel[..3] {
          *channel = table[*channel as usize];
        }
      }
    });
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;

  fn mapped(p_levels: Levels, p_values: &[u8]) -> Vec<u8> {
    let pixels: Vec<u8> = p_values.iter().flat_map(|value| [*value, *value, *value, 200]).collect();
    let mut image = Image::new_from_pixels(p_values.len() as u32, 1, pixels, Channels::RGBA);
    p_levels.apply_on_cpu(&mut image);
    image.rgba().chunks_exact(4).map(|pixel| pixel[0]).collect()
  }

  #[test]
  fn the_full_range_with_no_bend_changes_nothing() {
    let values: Vec<u8> = (0..=255).collect();
    assert_eq!(mapped(levels(0.0, 255.0), &values), values);
  }

  #[test]
  fn the_input_points_stretch_the_tones_between_them() {
    assert_eq!(mapped(levels(50.0, 150.0), &[0, 50, 100, 150, 255]), vec![0, 0, 128, 255, 255]);
  }

  #[test]
  fn a_gamma_above_one_brightens_the_middle_and_below_one_darkens_it() {
    let brighter = mapped(levels(0.0, 255.0).with_gamma(2.0), &[0, 128, 255]);
    let darker = mapped(levels(0.0, 255.0).with_gamma(0.5), &[0, 128, 255]);
    assert_eq!((brighter[0], brighter[2]), (0, 255), "the ends stay where they are");
    assert!(brighter[1] > 128 && darker[1] < 128, "{brighter:?} {darker:?}");
  }

  #[test]
  fn the_output_range_limits_how_dark_and_how_light_the_result_is() {
    assert_eq!(mapped(levels(0.0, 255.0).with_output(40.0, 200.0), &[0, 255]), vec![40, 200]);
  }

  #[test]
  fn the_alpha_is_left_alone_and_a_range_with_no_width_is_a_threshold() {
    let mut image = Image::new_from_pixels(2, 1, vec![10, 10, 10, 77, 200, 200, 200, 88], Channels::RGBA);
    levels(100.0, 100.0).apply_on_cpu(&mut image);
    assert_eq!(image.rgba(), &[0, 0, 0, 77, 255, 255, 255, 88]);
  }

  #[test]
  fn the_gpu_gives_the_same_pixels_as_the_cpu() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let pixels: Vec<u8> = (0..65536u32).flat_map(|i| [i as u8, (i >> 8) as u8, (i * 37 % 256) as u8, (i * 13 % 256) as u8]).collect();
    let effect = levels(20.0, 230.0).with_gamma(1.7).with_output(10.0, 245.0);
    let actual = renderer.process(&[effect.passes(256, 256)], 256, 256, &pixels)?;
    let mut expected = Image::new_from_pixels(256, 256, pixels, Channels::RGBA);
    effect.apply_on_cpu(&mut expected);
    assert_eq!(actual, expected.to_rgba_vec());
    Ok(())
  }
}
