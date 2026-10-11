use crate::lut::channel_lut_pass;
use abra_core::{
  Color, Image,
  image::gpu::{GpuPass, GpuProcessor},
};
use options::{Effect, Options};
use rayon::prelude::*;

/// Multiplies each color channel by a color, preserving the image alpha.
fn apply_color_multiply(p_image: &mut Image, p_color: Color) {
  let (red, green, blue, alpha) = p_color.rgba();
  let tint = [red as f32, green as f32, blue as f32];
  let strength = alpha as f32 / 255.0;
  let source = p_image.rgba();
  let mut output = source.to_vec();

  output.par_chunks_exact_mut(4).zip(source.par_chunks_exact(4)).for_each(|(out, original)| {
    for channel in 0..3 {
      let before = original[channel] as f32;
      let multiplied = (before * tint[channel] / 255.0).round();
      out[channel] = (before + (multiplied - before) * strength).round().clamp(0.0, 255.0) as u8;
    }
  });
  p_image.set_rgba(output);
}

/// A color multiplication adjustment. Create one with [`color_multiply`].
#[derive(Clone)]
pub struct ColorMultiply {
  color: Color,
  options: Options,
}

impl Effect for ColorMultiply {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_color_multiply(p_image, self.color);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for ColorMultiply {
  /// A channel lookup table made from the CPU operation, so CPU and GPU results match exactly.
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![channel_lut_pass(|image| self.cpu_processor(image))]
  }
}

/// Multiplies the image by `p_color`. White leaves it unchanged; the color alpha controls the adjustment strength.
pub fn color_multiply(p_color: Color) -> ColorMultiply {
  ColorMultiply {
    color: p_color,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn multiplies_channels_and_preserves_alpha() {
    let mut image = Image::new_from_color(1, 1, Color::from_rgba(200, 100, 50, 77));
    color_multiply(Color::from_rgb(128, 255, 64)).apply_on_cpu(&mut image);
    assert_eq!(image.get_pixel(0, 0), Some((100, 100, 13, 77)));
  }

  #[test]
  fn color_alpha_controls_strength() {
    let mut image = Image::new_from_color(1, 1, Color::from_rgb(200, 100, 50));
    color_multiply(Color::from_rgba(128, 255, 64, 128)).apply_on_cpu(&mut image);
    assert_eq!(image.get_pixel(0, 0), Some((150, 100, 31, 255)));
  }
}
