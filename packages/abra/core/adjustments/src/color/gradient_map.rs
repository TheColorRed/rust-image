use std::sync::Arc;

use abra_core::{
  Gradient, Image,
  image::gpu::{GpuAux, GpuPass, GpuProcessor},
};
use options::{Effect, Options};

/// How many brightness values there are: the gradient is sampled once for each, so the CPU and the shader look up the same
/// whole-number table.
const LUT_SIZE: u32 = 256;

/// The gradient's color for each brightness from 0 to 255, as RGBA texels.
fn build_lut(p_gradient: &Gradient) -> Vec<u8> {
  let mut lut = Vec::with_capacity(LUT_SIZE as usize * 4);
  for index in 0..LUT_SIZE {
    let (r, g, b, a) = p_gradient.color_at(index as f32 / (LUT_SIZE - 1) as f32).rgba();
    lut.extend_from_slice(&[r, g, b, a]);
  }
  lut
}

/// The brightness of a pixel, from 0 to 255, in the same whole-number steps as `grayscale` and the shader.
fn brightness(p_r: u8, p_g: u8, p_b: u8) -> usize {
  ((77 * p_r as u32 + 150 * p_g as u32 + 29 * p_b as u32 + 128) >> 8) as usize
}

fn apply_gradient_map(p_image: &mut Image, p_lut: &[u8]) {
  p_image.mut_pixels(|mut pixel| {
    let entry = brightness(pixel[0], pixel[1], pixel[2]) * 4;
    pixel[0] = p_lut[entry];
    pixel[1] = p_lut[entry + 1];
    pixel[2] = p_lut[entry + 2];
  });
}

#[derive(Clone)]
pub struct GradientMap {
  lut: Arc<[u8]>,
  options: Options,
}

impl Effect for GradientMap {
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
    apply_gradient_map(p_image, &self.lut);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for GradientMap {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![
      GpuPass::new(include_str!("./gradient_map.wgsl"), Vec::new()).with_aux(GpuAux {
        width: LUT_SIZE,
        height: 1,
        rgba: self.lut.clone(),
      }),
    ]
  }
}

/// Maps image brightness to the supplied gradient: black takes the gradient's start and white its end.
pub fn gradient_map(p_gradient: impl Into<Gradient>) -> GradientMap {
  GradientMap {
    lut: build_lut(&p_gradient.into()).into(),
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Color;
  use gpu::{GpuContext, LiveRenderer};

  fn image() -> (u32, u32, Vec<u8>) {
    let (width, height) = (37u32, 21u32);
    let mut pixels = Vec::new();
    for y in 0..height {
      for x in 0..width {
        pixels.extend_from_slice(&[(x * 6) as u8, (y * 12) as u8, ((x * y) % 256) as u8, (200 + x) as u8]);
      }
    }
    (width, height, pixels)
  }

  fn gradient() -> Gradient {
    Gradient::from_to(Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255))
  }

  #[test]
  fn black_takes_the_start_and_white_the_end_and_alpha_is_kept() {
    let mut image = Image::new_from_pixels(2, 1, vec![0, 0, 0, 40, 255, 255, 255, 90], abra_core::Channels::RGBA);
    gradient_map(gradient()).apply_on_cpu(&mut image);
    assert_eq!(image.to_rgba_vec(), vec![255, 0, 0, 40, 0, 0, 255, 90]);
  }

  #[test]
  fn the_table_has_one_color_for_each_brightness() {
    let lut = build_lut(&gradient());
    assert_eq!(lut.len(), 256 * 4);
    assert_eq!(&lut[..3], &[255, 0, 0]);
    assert_eq!(&lut[255 * 4..255 * 4 + 3], &[0, 0, 255]);
  }

  #[test]
  fn the_gpu_gives_exactly_the_cpu_pixels() -> anyhow::Result<()> {
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let (width, height, pixels) = image();
    let effect = gradient_map(gradient());
    let gpu = renderer.process(&[effect.passes(width, height)], width, height, &pixels)?;
    let mut cpu = Image::new_from_pixels(width, height, pixels, abra_core::Channels::RGBA);
    effect.apply_on_cpu(&mut cpu);
    assert_eq!(gpu, cpu.to_rgba_vec());
    Ok(())
  }
}
