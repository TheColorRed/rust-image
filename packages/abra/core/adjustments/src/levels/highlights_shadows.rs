use super::saturation::{gray, saturate_channel};
use abra_core::{
  Image,
  image::gpu::{GpuPass, GpuProcessor},
};
use options::{Effect, Options};
use primitives::color::{linear_f32_to_srgb_u8, linear_to_srgb, srgb_to_linear, srgb_u8_to_linear_f32};
use rayon::prelude::*;
#[path = "highlights_shadows_context.rs"]
mod context;

fn weight(p_value: f32, p_tone: f32) -> f32 {
  if p_tone == 0.0 {
    return 0.0;
  }
  let across = (p_value / p_tone).clamp(0.0, 1.0);
  1.0 - across * across * (3.0 - 2.0 * across)
}

/// The two end-weighted terms vanish at black and white. Their combined absolute
/// derivative is at most one, so a strength of 0.9 keeps the slope at least 0.1
/// for amount values within the basic slider range (-100..=100).
fn tone(p_value: f32, p_highlights: f32, p_shadows: f32) -> f32 {
  let inverse = 1.0 - p_value;
  p_value + 0.9 * p_value * inverse * (p_shadows * inverse * inverse - p_highlights * p_value * p_value)
}

/// A smooth luminance adjustment that preserves linear RGB proportions and alpha.
/// Create one with [`highlights_shadows`].
#[derive(Clone)]
pub struct HighlightsShadows {
  highlights: f32,
  shadows: f32,
  highlight_tone: f32,
  shadow_tone: f32,
  highlight_radius: u32,
  shadow_radius: u32,
  color: i32,
  midtone: f32,
  black_clip: f32,
  white_clip: f32,
  options: Options,
}

impl HighlightsShadows {
  /// Shadow tonal width as a fraction: 0.5 means 50%. Defaults to 0.5. The input is not clamped.
  pub fn with_shadow_tone(mut self, p_tone: impl Into<f64>) -> Self {
    self.shadow_tone = p_tone.into() as f32;
    self
  }
  /// Highlight tonal width as a fraction: 0.5 means 50%. Defaults to 0.5. The input is not clamped.
  pub fn with_highlight_tone(mut self, p_tone: impl Into<f64>) -> Self {
    self.highlight_tone = p_tone.into() as f32;
    self
  }
  /// Sets both tonal widths as fractions, without clamping.
  pub fn with_tone(self, p_tone: impl Into<f64>) -> Self {
    let tone = p_tone.into();
    self.with_shadow_tone(tone).with_highlight_tone(tone)
  }
  /// Shadow neighborhood radius in pixels, rounded to an unsigned integer. Defaults to 30; no radius cap.
  pub fn with_shadow_radius(mut self, p_radius: impl Into<f64>) -> Self {
    self.shadow_radius = p_radius.into().round() as u32;
    self
  }
  /// Highlight neighborhood radius in pixels, rounded to an unsigned integer. Defaults to 30; no radius cap.
  pub fn with_highlight_radius(mut self, p_radius: impl Into<f64>) -> Self {
    self.highlight_radius = p_radius.into().round() as u32;
    self
  }
  /// Sets both neighborhood radii in pixels.
  pub fn with_radius(self, p_radius: impl Into<f64>) -> Self {
    let radius = p_radius.into();
    self.with_shadow_radius(radius).with_highlight_radius(radius)
  }
  /// Color correction using the saturation math, rounded to an integer without clamping. Defaults to 0.
  pub fn with_color(mut self, p_color: impl Into<f64>) -> Self {
    self.color = p_color.into().round() as i32;
    self
  }
  /// Midtone contrast, with 100 representing full basic-slider strength. Defaults to 0; input is not clamped.
  pub fn with_midtone(mut self, p_midtone: impl Into<f64>) -> Self {
    self.midtone = p_midtone.into() as f32 / 100.0;
    self
  }
  /// Fraction of visible pixels clipped at the dark end, without clamping. Defaults to 0.
  /// For example, 0.0001 means 0.01%.
  /// Histogram clipping uses the CPU; the remaining settings have a GPU implementation.
  pub fn with_black_clip(mut self, p_percent: impl Into<f64>) -> Self {
    self.black_clip = p_percent.into() as f32;
    self
  }
  /// Fraction of visible pixels clipped at the bright end, without clamping. Defaults to 0.
  /// For example, 0.0001 means 0.01%.
  /// Brightening remains limited by the color gamut. Histogram clipping uses the CPU.
  pub fn with_white_clip(mut self, p_percent: impl Into<f64>) -> Self {
    self.white_clip = p_percent.into() as f32;
    self
  }
  fn unchanged(&self) -> bool {
    self.highlights == 0.0
      && self.shadows == 0.0
      && self.color == 0
      && self.midtone == 0.0
      && self.black_clip == 0.0
      && self.white_clip == 0.0
  }
  fn radii(&self) -> [u32; 2] {
    [
      if self.shadows == 0.0 { 0 } else { self.shadow_radius },
      if self.highlights == 0.0 { 0 } else { self.highlight_radius },
    ]
  }
  fn color_factor(&self) -> i32 {
    (((self.color as f64 + 100.0) * 256.0 / 100.0).round()) as i32
  }
  fn clip_bounds(&self, p_pixels: &[u8], p_decode: &[f32; 256]) -> [f32; 2] {
    let mut bounds = [0.0, 1.0];
    if self.black_clip == 0.0 && self.white_clip == 0.0 {
      return bounds;
    }
    let mut histogram = [0usize; 256];
    for pixel in p_pixels.chunks_exact(4).filter(|pixel| pixel[3] != 0) {
      let value = linear_to_srgb(context::luminance([
        p_decode[pixel[0] as usize],
        p_decode[pixel[1] as usize],
        p_decode[pixel[2] as usize],
      ]));
      histogram[(value * 255.0).round().clamp(0.0, 255.0) as usize] += 1;
    }
    let total = histogram.iter().sum::<usize>();
    if total == 0 {
      return bounds;
    }
    for (side, percent) in [self.black_clip, self.white_clip].into_iter().enumerate() {
      if percent == 0.0 {
        continue;
      }
      let count = (total as f64 * percent as f64).ceil() as usize;
      let mut accumulated = 0;
      for offset in 0..256 {
        let index = if side == 0 { offset } else { 255 - offset };
        accumulated += histogram[index];
        if accumulated >= count {
          bounds[side] = index as f32 / 255.0;
          break;
        }
      }
    }
    if bounds[1] <= bounds[0] { [0.0, 1.0] } else { bounds }
  }
}

impl Effect for HighlightsShadows {
  fn options(&self) -> &Options {
    &self.options
  }
  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }
  fn padding(&self) -> i32 {
    self.radii().into_iter().max().unwrap_or(0) as i32
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    if self.unchanged() || p_image.rgba().is_empty() {
      return;
    }
    let decode: [f32; 256] = std::array::from_fn(|index| srgb_u8_to_linear_f32(index as u8));
    let original = p_image.rgba();
    let contexts = context::contexts(p_image, self.radii());
    let bounds = self.clip_bounds(original, &decode);
    let mut out = original.to_vec();
    out.par_chunks_mut(4).zip(contexts.par_chunks(4)).for_each(|(pixel, neighborhood)| {
      let rgb = [
        decode[pixel[0] as usize],
        decode[pixel[1] as usize],
        decode[pixel[2] as usize],
      ];
      let luminance = context::luminance(rgb);
      if luminance == 0.0 {
        return;
      }
      let value = linear_to_srgb(luminance);
      let shadow_weight = weight(value, self.shadow_tone).min(weight(neighborhood[0] as f32 / 255.0, self.shadow_tone));
      let highlight_weight =
        weight(1.0 - value, self.highlight_tone).min(weight(1.0 - neighborhood[1] as f32 / 255.0, self.highlight_tone));
      let adjusted = tone(value, self.highlights * highlight_weight, self.shadows * shadow_weight);
      let adjusted = (adjusted - bounds[0]) / (bounds[1] - bounds[0]);
      let adjusted = adjusted.clamp(0.0, 1.0);
      let target = srgb_to_linear(adjusted + 0.9 * self.midtone * adjusted * (1.0 - adjusted) * (2.0 * adjusted - 1.0));
      // Limit the common gain before encoding. Clipping channels separately would
      // change the color; already saturated colors have less room to be lifted.
      let gain = (target / luminance).min(1.0 / rgb[0].max(rgb[1]).max(rgb[2]));
      for channel in 0..3 {
        pixel[channel] = linear_f32_to_srgb_u8(rgb[channel] * gain);
      }
    });
    p_image.set_rgba(out);
    if self.color != 0 {
      let factor = self.color_factor();
      p_image.mut_pixels(|mut pixel| {
        let gray = gray(pixel[0], pixel[1], pixel[2]);
        for channel in 0..3 {
          pixel[channel] = saturate_channel(pixel[channel], gray, factor);
        }
      });
    }
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    if self.black_clip == 0.0 && self.white_clip == 0.0 { Some(self) } else { None }
  }
}

impl GpuProcessor for HighlightsShadows {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    if self.unchanged() {
      return Vec::new();
    }
    let parameters = [
      self.highlights,
      self.shadows,
      self.highlight_tone,
      self.shadow_tone,
      self.midtone,
      0.0,
      0.0,
      0.0,
    ]
    .iter()
    .flat_map(|v| v.to_le_bytes())
    .collect::<Vec<u8>>();
    let mut passes = context::passes(self.radii());
    passes.push(GpuPass::new(include_str!("./highlights_shadows.wgsl"), parameters).with_base());
    if self.color != 0 {
      passes.push(GpuPass::new(include_str!("./saturation.wgsl"), self.color_factor().to_le_bytes().to_vec()));
    }
    passes
  }
}

/// Adjusts highlights and shadows with a smooth, increasing luminance curve.
/// The basic slider range is `-100..=100`. Inputs are not clamped; zero leaves the image unchanged.
/// Positive highlights darken bright tones; positive shadows lift dark tones.
/// Black, white and alpha are preserved. A common linear RGB gain preserves color;
/// brightening is limited by the strongest channel to keep it within the gamut.
/// Tone widths use fractions from 0 to 1, defaulting to 0.5 (50%). Neighborhood radii default to 30 pixels.
/// Color, midtone contrast and histogram clipping default to zero.
pub fn highlights_shadows(p_highlights: impl Into<f64>, p_shadows: impl Into<f64>) -> HighlightsShadows {
  HighlightsShadows {
    highlights: p_highlights.into() as f32 / 100.0,
    shadows: p_shadows.into() as f32 / 100.0,
    highlight_tone: 0.5,
    shadow_tone: 0.5,
    highlight_radius: 30,
    shadow_radius: 30,
    color: 0,
    midtone: 0.0,
    black_clip: 0.0,
    white_clip: 0.0,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;
  use gpu::{GpuContext, LiveRenderer};

  #[test]
  fn every_slider_pair_keeps_the_gray_ramp_in_order_and_preserves_endpoints_and_alpha() {
    let ramp: Vec<u8> = (0..=255u8).flat_map(|gray| [gray, gray, gray, gray]).collect();
    for highlights in (-100..=100).step_by(10) {
      for shadows in (-100..=100).step_by(10) {
        let mut image = Image::new_from_pixels(256, 1, ramp.clone(), Channels::RGBA);
        highlights_shadows(highlights, shadows).apply_on_cpu(&mut image);
        let pixels = image.rgba();
        assert_eq!(&pixels[..4], &[0, 0, 0, 0]);
        assert_eq!(&pixels[1020..], &[255, 255, 255, 255]);
        for (index, pixel) in pixels.chunks_exact(4).enumerate() {
          assert_eq!(pixel[0], pixel[1]);
          assert_eq!(pixel[1], pixel[2]);
          assert_eq!(pixel[3], index as u8);
          if index > 0 {
            assert!(pixel[0] >= pixels[(index - 1) * 4], "reversed at {index}, H={highlights}, S={shadows}");
          }
        }
      }
    }
  }

  #[test]
  fn controls_move_tones_in_the_expected_direction_and_zero_is_exact() {
    let pixels = vec![30, 50, 70, 17, 230, 200, 160, 213, 255, 40, 10, 255];
    let apply = |highlights, shadows| {
      let mut image = Image::new_from_pixels(3, 1, pixels.clone(), Channels::RGBA);
      highlights_shadows(highlights, shadows).with_radius(0).apply_on_cpu(&mut image);
      image.to_rgba_vec()
    };
    assert_eq!(apply(0, 0), pixels);
    assert!(apply(0, 100)[0] > pixels[0]);
    assert!(apply(0, -100)[0] < pixels[0]);
    assert!(apply(100, 0)[4] < pixels[4]);
    assert!(apply(-100, 0)[4] > pixels[4]);
    // The saturated pixel cannot be lifted further without changing its color.
    assert_eq!(&apply(0, 100)[8..12], &pixels[8..12]);
  }

  #[test]
  fn colors_keep_their_linear_channel_proportions() {
    for color in [[30, 50, 70], [230, 200, 160], [255, 40, 10], [8, 90, 160]] {
      for (highlights, shadows) in [(100, 0), (0, 100), (100, 100), (-100, -100)] {
        let mut image = Image::new_from_pixels(1, 1, vec![color[0], color[1], color[2], 91], Channels::RGBA);
        highlights_shadows(highlights, shadows).apply_on_cpu(&mut image);
        let result = image.rgba();
        assert_eq!(result[3], 91);
        // Each rounded output byte gives an interval of possible linear gains.
        // All three must share a gain, allowing half a byte of rounding each.
        let mut lower = 0.0f32;
        let mut upper = f32::INFINITY;
        for i in 0..3 {
          let original = srgb_u8_to_linear_f32(color[i]);
          lower = lower.max(srgb_to_linear((result[i] as f32 - 0.501).max(0.0) / 255.0) / original);
          upper = upper.min(srgb_to_linear((result[i] as f32 + 0.501).min(255.0) / 255.0) / original);
        }
        assert!(lower <= upper, "color changed: {color:?} -> {result:?}, H={highlights}, S={shadows}");
      }
    }
  }

  #[test]
  fn fractional_tone_settings_match_defaults_and_affect_the_requested_range() {
    let ramp: Vec<u8> = (0..=255u8).flat_map(|gray| [gray, gray, gray, 255]).collect();
    let apply = |effect: HighlightsShadows| {
      let mut image = Image::new_from_pixels(256, 1, ramp.clone(), Channels::RGBA);
      effect.apply_on_cpu(&mut image);
      image.to_rgba_vec()
    };
    let default = highlights_shadows(100, 100).with_radius(0);
    assert_eq!(apply(default.clone().with_tone(0.5)), apply(default.clone()));
    assert_eq!(apply(default.clone().with_shadow_tone(0.5).with_highlight_tone(0.5)), apply(default.clone()),);
    let result = apply(default.clone().with_tone(0.5));
    assert!(result[64 * 4] > 64, "50% tone must include a quarter-gray shadow");
    assert!(result[192 * 4] < 192, "50% tone must include a three-quarter-gray highlight");
  }

  #[test]
  fn builder_settings_are_not_clamped_to_the_basic_slider_ranges() {
    let effect = highlights_shadows(150, -175)
      .with_tone(1.5)
      .with_shadow_tone(-0.25)
      .with_radius(2048)
      .with_color(175)
      .with_midtone(-150)
      .with_black_clip(0.6)
      .with_white_clip(0.75);
    assert_eq!((effect.highlights, effect.shadows), (1.5, -1.75));
    assert_eq!((effect.shadow_tone, effect.highlight_tone), (-0.25, 1.5));
    assert_eq!((effect.shadow_radius, effect.highlight_radius), (2048, 2048));
    assert_eq!(effect.color, 175);
    assert_eq!(effect.color_factor(), 704);
    assert_eq!(effect.midtone, -1.5);
    assert_eq!((effect.black_clip, effect.white_clip), (0.6, 0.75));
  }

  #[test]
  fn tone_and_radius_change_the_targeted_regions() {
    let pixels: Vec<u8> = (0..9)
      .flat_map(|i| {
        let gray = if i == 4 { 30 } else { 200 };
        [gray, gray, gray, 255]
      })
      .collect();
    let apply = |effect: HighlightsShadows| {
      let mut image = Image::new_from_pixels(9, 1, pixels.clone(), Channels::RGBA);
      effect.apply_on_cpu(&mut image);
      image.to_rgba_vec()
    };
    assert_eq!(apply(highlights_shadows(100, 100).with_tone(0)), pixels);
    let small = apply(highlights_shadows(0, 100).with_shadow_radius(0));
    let large = apply(highlights_shadows(0, 100).with_shadow_radius(4));
    assert!(small[16] > large[16], "a small dark pixel in a bright neighborhood receives less shadow lift");
    let mut narrow = Image::new_from_pixels(1, 1, vec![180, 180, 180, 91], Channels::RGBA);
    let mut wide = narrow.clone();
    highlights_shadows(100, 0).with_highlight_tone(0.2).apply_on_cpu(&mut narrow);
    highlights_shadows(100, 0).with_highlight_tone(1.0).apply_on_cpu(&mut wide);
    assert_eq!(narrow.rgba()[0], 180);
    assert!(wide.rgba()[0] < narrow.rgba()[0]);
  }

  #[test]
  fn color_midtone_and_histogram_clipping_have_visible_effects_and_keep_alpha() {
    let mut color = Image::new_from_pixels(1, 1, vec![80, 120, 160, 91], Channels::RGBA);
    highlights_shadows(0, 0).with_color(-100).apply_on_cpu(&mut color);
    assert_eq!(color.rgba()[0], color.rgba()[1]);
    assert_eq!(color.rgba()[1], color.rgba()[2]);
    assert_eq!(color.rgba()[3], 91);
    let mut contrast = Image::new_from_pixels(2, 1, vec![64, 64, 64, 17, 192, 192, 192, 213], Channels::RGBA);
    highlights_shadows(0, 0).with_midtone(100).apply_on_cpu(&mut contrast);
    assert!(contrast.rgba()[0] < 64 && contrast.rgba()[4] > 192);
    assert_eq!((contrast.rgba()[3], contrast.rgba()[7]), (17, 213));
    let ramp: Vec<u8> = (50..=200u8).flat_map(|gray| [gray, gray, gray, 255]).collect();
    let mut image = Image::new_from_pixels(151, 1, ramp, Channels::RGBA);
    let clipped = highlights_shadows(0, 0).with_black_clip(0.01).with_white_clip(0.01);
    assert!(clipped.gpu_processor().is_none());
    clipped.apply_on_cpu(&mut image);
    assert_eq!(&image.rgba()[4..8], &[0, 0, 0, 255]);
    assert_eq!(&image.rgba()[149 * 4..150 * 4], &[255, 255, 255, 255]);
  }

  #[test]
  fn gpu_matches_cpu_with_independent_tones_radii_and_color_correction() -> anyhow::Result<()> {
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let pixels: Vec<u8> =
      (0..1024u32).flat_map(|i| [(i * 7) as u8, (i * 13) as u8, (i * 37) as u8, (i * 23) as u8]).collect();
    let effect = highlights_shadows(65, 45)
      .with_shadow_tone(0.7)
      .with_highlight_tone(0.6)
      .with_shadow_radius(2)
      .with_highlight_radius(5)
      .with_midtone(30)
      .with_color(20);
    let actual = renderer.process(&[effect.passes(32, 32)], 32, 32, &pixels)?;
    let mut expected = Image::new_from_pixels(32, 32, pixels, Channels::RGBA);
    effect.apply_on_cpu(&mut expected);
    for (index, (&actual, &expected)) in actual.iter().zip(expected.rgba().iter()).enumerate() {
      let tolerance = if index % 4 == 3 { 0 } else { 2 };
      assert!((actual as i16 - expected as i16).abs() <= tolerance, "byte {index}: {actual} != {expected}");
    }
    Ok(())
  }

  #[test]
  fn gpu_matches_cpu_within_one_byte_on_varied_colors_and_extreme_controls() -> anyhow::Result<()> {
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let pixels: Vec<u8> =
      (0..65536u32).flat_map(|i| [i as u8, (i >> 8) as u8, (i * 37 % 256) as u8, (i * 13 % 256) as u8]).collect();
    for (highlights, shadows) in [
      (0, 0),
      (40, 0),
      (0, 40),
      (100, 100),
      (-100, -100),
      (100, -100),
      (-100, 100),
    ] {
      let effect = highlights_shadows(highlights, shadows);
      let actual = renderer.process(&[effect.passes(256, 256)], 256, 256, &pixels)?;
      let mut expected = Image::new_from_pixels(256, 256, pixels.clone(), Channels::RGBA);
      effect.apply_on_cpu(&mut expected);
      for (index, (&actual, &expected)) in actual.iter().zip(expected.rgba().iter()).enumerate() {
        let tolerance = if index % 4 == 3 || (highlights == 0 && shadows == 0) { 0 } else { 1 };
        assert!(
          (actual as i16 - expected as i16).abs() <= tolerance,
          "byte {index}, H={highlights}, S={shadows}: {actual} != {expected}"
        );
      }
    }
    Ok(())
  }
}
