use abra_core::{
  Image, hsl_to_rgb,
  image::gpu::{GpuPass, GpuProcessor},
};
use options::{Effect, Options};
use rayon::prelude::*;

/// Starting settings for a black and white conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlackWhitePreset {
  #[default]
  Default,
  BlueFilter,
  Darker,
  GreenFilter,
  HighContrastBlueFilter,
  HighContrastRedFilter,
  Infrared,
  Lighter,
  MaximumBlack,
  MaximumWhite,
  NeutralDensity,
  RedFilter,
  YellowFilter,
}

impl BlackWhitePreset {
  fn settings(self) -> [f32; 6] {
    match self {
      Self::Default => [0.0; 6],
      Self::BlueFilter => [-20.0, -10.0, 0.0, 0.0, 40.0, 20.0],
      Self::Darker => [-20.0, -20.0, -20.0, -20.0, -20.0, -20.0],
      Self::GreenFilter => [-10.0, 10.0, 40.0, 10.0, -10.0, -10.0],
      Self::HighContrastBlueFilter => [-50.0, -30.0, 0.0, 20.0, 80.0, 40.0],
      Self::HighContrastRedFilter => [80.0, 40.0, 0.0, -30.0, -50.0, 20.0],
      Self::Infrared => [100.0, 40.0, -40.0, -60.0, -80.0, 20.0],
      Self::Lighter => [20.0; 6],
      Self::MaximumBlack => [-100.0; 6],
      Self::MaximumWhite => [100.0; 6],
      Self::NeutralDensity => [-10.0; 6],
      Self::RedFilter => [50.0, 20.0, 0.0, -10.0, -20.0, 10.0],
      Self::YellowFilter => [20.0, 40.0, 10.0, -20.0, -30.0, 0.0],
    }
  }
}

/// Converts color to monochrome, with controls for six color ranges. The controls shift the brightness of pixels in each range;
/// their supplied values are kept as given. Alpha is preserved.
#[derive(Clone)]
pub struct BlackWhite {
  settings: [f32; 6],
  tint: Option<(f32, f32)>,
  options: Options,
}

impl BlackWhite {
  /// Sets the adjustment for reds, yellows, greens, cyans, blues and magentas. Values are percentages; zero leaves the basic
  /// luminance conversion unchanged.
  pub fn with_channels(
    mut self, p_reds: f32, p_yellows: f32, p_greens: f32, p_cyans: f32, p_blues: f32, p_magentas: f32,
  ) -> Self {
    self.settings = [p_reds, p_yellows, p_greens, p_cyans, p_blues, p_magentas];
    self
  }

  /// Sets the six channel controls to a named starting preset.
  pub fn with_preset(mut self, p_preset: BlackWhitePreset) -> Self {
    self.settings = p_preset.settings();
    self
  }

  /// Colors the monochrome result with the given HSL hue in degrees and saturation from 0 to 1.
  pub fn with_tint(mut self, p_hue: f32, p_saturation: f32) -> Self {
    self.tint = Some((p_hue, p_saturation));
    self
  }

  /// Removes tinting and returns to neutral monochrome.
  pub fn without_tint(mut self) -> Self {
    self.tint = None;
    self
  }
}

fn ranges(p_rgb: [f32; 3]) -> [f32; 6] {
  let [r, g, b] = p_rgb;
  let maximum = r.max(g).max(b);
  let minimum = r.min(g).min(b);
  let middle = r + g + b - maximum - minimum;
  let mut weights = [0.0; 6];
  if r >= g && r >= b {
    weights[0] = maximum - middle;
  }
  if g >= r && g >= b {
    weights[2] = maximum - middle;
  }
  if b >= r && b >= g {
    weights[4] = maximum - middle;
  }
  if b <= r && b <= g {
    weights[1] = middle - minimum;
  }
  if r <= g && r <= b {
    weights[3] = middle - minimum;
  }
  if g <= r && g <= b {
    weights[5] = middle - minimum;
  }
  weights
}

fn gray(p_rgb: [f32; 3], p_settings: [f32; 6]) -> f32 {
  let [r, g, b] = p_rgb;
  let luma = 0.299 * r + 0.587 * g + 0.114 * b;
  let range_weight = ranges(p_rgb);
  let shift: f32 = range_weight.iter().zip(p_settings).map(|(weight, setting)| weight * setting).sum();
  (luma + shift / 100.0).clamp(0.0, 1.0)
}

impl Effect for BlackWhite {
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
    p_image.colors().as_slice_mut().expect("Image colors must be contiguous").par_chunks_exact_mut(4).for_each(
      |pixel| {
        let value = gray(
          [
            pixel[0] as f32 / 255.0,
            pixel[1] as f32 / 255.0,
            pixel[2] as f32 / 255.0,
          ],
          self.settings,
        );
        if let Some((hue, saturation)) = self.tint {
          let rgb = hsl_to_rgb(hue.rem_euclid(360.0), saturation.clamp(0.0, 1.0), value);
          pixel[..3].copy_from_slice(&[rgb.0, rgb.1, rgb.2]);
        } else {
          let value = (value * 255.0 + 0.5).floor() as u8;
          pixel[..3].fill(value);
        }
      },
    );
  }
  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for BlackWhite {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    let tint = self.tint.unwrap_or((0.0, 0.0));
    let uniforms: Vec<u8> = self
      .settings
      .iter()
      .copied()
      .chain([
        tint.0,
        tint.1,
        if self.tint.is_some() { 1.0 } else { 0.0 },
        0.0,
        0.0,
        0.0,
      ])
      .flat_map(|value| value.to_le_bytes())
      .collect();
    vec![GpuPass::new(include_str!("./black_white.wgsl"), uniforms)]
  }
}

/// Creates a black and white conversion with default luminance weights.
pub fn black_white() -> BlackWhite {
  BlackWhite {
    settings: [0.0; 6],
    tint: None,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;

  #[test]
  fn default_is_grayscale_and_preserves_alpha() {
    let mut image = Image::new_from_pixels(2, 1, vec![255, 0, 0, 19, 0, 255, 0, 37], Channels::RGBA);
    black_white().apply_on_cpu(&mut image);
    assert_eq!(image.rgba(), &[76, 76, 76, 19, 150, 150, 150, 37]);
  }

  #[test]
  fn channel_controls_and_presets_change_the_result() {
    let pixels = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];
    let mut adjusted = Image::new_from_pixels(3, 1, pixels.clone(), Channels::RGBA);
    black_white().with_channels(100.0, 0.0, 0.0, 0.0, 0.0, 0.0).apply_on_cpu(&mut adjusted);
    assert!(adjusted.rgba()[0] > 76);
    let mut preset = Image::new_from_pixels(3, 1, pixels, Channels::RGBA);
    black_white().with_preset(BlackWhitePreset::Infrared).apply_on_cpu(&mut preset);
    assert_ne!(&preset.rgba()[..3], &[76, 76, 76]);
  }

  #[test]
  fn gpu_matches_cpu() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};
    let width = 31;
    let height = 17;
    let pixels: Vec<u8> =
      (0..width * height).flat_map(|i| [(i * 31) as u8, (i * 17) as u8, (i * 7) as u8, (i * 13) as u8]).collect();
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    for effect in [
      black_white(),
      black_white().with_preset(BlackWhitePreset::BlueFilter),
      black_white().with_channels(-50.0, 20.0, 40.0, 60.0, 20.0, 80.0),
      black_white().with_preset(BlackWhitePreset::Infrared).with_tint(42.0, 0.2),
    ] {
      let actual = renderer.process(&[effect.passes(width, height)], width, height, &pixels)?;
      let mut expected = Image::new_from_pixels(width, height, pixels.clone(), Channels::RGBA);
      effect.apply_on_cpu(&mut expected);
      assert_eq!(actual, expected.rgba());
    }
    Ok(())
  }
}
