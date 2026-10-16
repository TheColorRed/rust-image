use abra_core::{
  Image,
  image::gpu::{GpuPass, GpuProcessor},
};
use options::{Effect, Options};
use rayon::prelude::*;

/// The parts of a picture's colors that [`SelectiveColor`] can change on their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorRange {
  /// Pixels that are mostly red.
  Reds,
  /// Pixels that are red and green together, between red and green.
  Yellows,
  /// Pixels that are mostly green.
  Greens,
  /// Pixels that are green and blue together.
  Cyans,
  /// Pixels that are mostly blue.
  Blues,
  /// Pixels that are red and blue together.
  Magentas,
  /// The light parts, whatever their color.
  Whites,
  /// The middle tones, whatever their color.
  Neutrals,
  /// The dark parts, whatever their color.
  Blacks,
}

impl ColorRange {
  /// Every range, in the order of the adjustments (and the shader's table).
  pub const ALL: [ColorRange; 9] = [
    ColorRange::Reds,
    ColorRange::Yellows,
    ColorRange::Greens,
    ColorRange::Cyans,
    ColorRange::Blues,
    ColorRange::Magentas,
    ColorRange::Whites,
    ColorRange::Neutrals,
    ColorRange::Blacks,
  ];

  fn index(self) -> usize {
    self as usize
  }
}

/// How much a [`SelectiveColor`] adjustment changes the ink it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SelectiveMethod {
  /// By a share of the ink the pixel already has (`+0.5` makes it half again as much), so a pixel with none of an ink is not changed
  /// by it.
  #[default]
  Relative,
  /// By a fixed amount of ink, whatever the pixel has (`+0.5` adds half the ink there could be).
  Absolute,
}

/// Changes the colors in one range of a picture without touching the others, as the CMYK inks of a print: how much cyan, magenta, yellow
/// and black the pixels of each [`ColorRange`] have. Take away yellow from the reds and only the reds turn redder, less orange; take
/// away black from the blacks and only the dark parts lighten. Create one with [`selective_color`].
///
/// A pixel belongs to ranges by its red, green and blue values, `max`, `mid` and `min`: the primary it is most of (red, green or blue) by
/// `max - mid`, the secondary it is least of (cyan, magenta or yellow, named for the channel that is smallest) by `mid - min`, the whites
/// by `min`, the blacks by `1 - max`, and the neutrals by how near both `max` and `min` are to half. Ranges overlap, and the change of
/// each is its adjustment times how much the pixel belongs to it.
///
/// Inks are worked out from the pixel as in a print: the black ink is the least of `1 - red`, `1 - green` and `1 - blue` (that is,
/// `1 - max`), and the cyan, magenta and yellow are what is left of each once the black is taken out. So a dark pixel has a lot of
/// black and little cyan, magenta and yellow, and changing its cyan changes only the little it has. A black adjustment moves the black
/// ink, which lightens or darkens all three channels together. Results are clamped to what a byte holds. The alpha is left alone.
///
/// The GPU version (`selective_color.wgsl`) does the same sums in 32-bit floats, so it can differ from the CPU by one level.
#[derive(Clone)]
pub struct SelectiveColor {
  /// For each range, in [`ColorRange::ALL`] order, the change of `[cyan, magenta, yellow, black]`.
  adjustments: [[f32; 4]; 9],
  method: SelectiveMethod,
  options: Options,
}

impl SelectiveColor {
  /// Sets how much cyan, magenta, yellow and black one range gains or loses. Positive adds ink (darker, more of that color) and
  /// negative takes it away. A range is changed again when it is set again.
  /// - `p_range`: The range to change.
  /// - `p_cyan`, `p_magenta`, `p_yellow`, `p_black`: The change of each ink. `1` and `-1` are the most a slider would give; the
  ///   values are used as they are.
  pub fn with_range(mut self, p_range: ColorRange, p_cyan: f32, p_magenta: f32, p_yellow: f32, p_black: f32) -> Self {
    self.adjustments[p_range.index()] = [p_cyan, p_magenta, p_yellow, p_black];
    self
  }

  /// Sets whether the adjustments are shares of the ink a pixel has or fixed amounts. Defaults to [`SelectiveMethod::Relative`].
  pub fn with_method(mut self, p_method: SelectiveMethod) -> Self {
    self.method = p_method;
    self
  }
}

/// A selective color adjustment that changes nothing until ranges are set with [`SelectiveColor::with_range`].
pub fn selective_color() -> SelectiveColor {
  SelectiveColor {
    adjustments: [[0.0; 4]; 9],
    method: SelectiveMethod::Relative,
    options: None,
  }
}

/// How much a pixel belongs to each range, in [`ColorRange::ALL`] order. `p_rgb` is from `0` to `1`.
fn weights(p_rgb: [f32; 3]) -> [f32; 9] {
  let [r, g, b] = p_rgb;
  let max = r.max(g).max(b);
  let min = r.min(g).min(b);
  let mid = r + g + b - max - min;
  let mut weight = [0.0f32; 9];
  // The primary the pixel is most of, and the secondary it is least of (named for the smallest channel's opposite).
  let most = max - mid;
  if r >= g && r >= b {
    weight[ColorRange::Reds.index()] = most;
  } else if g >= b {
    weight[ColorRange::Greens.index()] = most;
  } else {
    weight[ColorRange::Blues.index()] = most;
  }
  let least = mid - min;
  if b <= r && b <= g {
    weight[ColorRange::Yellows.index()] = least;
  } else if r <= g {
    weight[ColorRange::Cyans.index()] = least;
  } else {
    weight[ColorRange::Magentas.index()] = least;
  }
  weight[ColorRange::Whites.index()] = min;
  weight[ColorRange::Blacks.index()] = 1.0 - max;
  weight[ColorRange::Neutrals.index()] = (1.0 - ((max - 0.5).abs() + (min - 0.5).abs())).clamp(0.0, 1.0);
  weight
}

/// The pixel with the adjustments made. `p_rgb` and the result are from `0` to `1`.
fn adjusted(p_rgb: [f32; 3], p_adjustments: &[[f32; 4]; 9], p_method: SelectiveMethod) -> [f32; 3] {
  let weight = weights(p_rgb);
  let ink = [1.0 - p_rgb[0], 1.0 - p_rgb[1], 1.0 - p_rgb[2]];
  let black = ink[0].min(ink[1]).min(ink[2]);
  // The cyan, magenta and yellow that are left once the black is taken out.
  let colors = ink.map(|ink| ink - black);
  let mut result = colors;
  let mut result_black = black;
  for (adjustment, weight) in p_adjustments.iter().zip(weight) {
    if weight == 0.0 {
      continue;
    }
    for channel in 0..3 {
      result[channel] += match p_method {
        SelectiveMethod::Relative => colors[channel] * adjustment[channel],
        SelectiveMethod::Absolute => adjustment[channel],
      } * weight;
    }
    result_black += match p_method {
      SelectiveMethod::Relative => black * adjustment[3],
      SelectiveMethod::Absolute => adjustment[3],
    } * weight;
  }
  let result_black = result_black.clamp(0.0, 1.0);
  result.map(|color| 1.0 - (color.max(0.0) + result_black).clamp(0.0, 1.0))
}

impl Effect for SelectiveColor {
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
      let rgb = adjusted([pixel[0] as f32 / 255.0, pixel[1] as f32 / 255.0, pixel[2] as f32 / 255.0], &self.adjustments, self.method);
      for channel in 0..3 {
        // Rounded half up, as the shader does.
        pixel[channel] = (rgb[channel] * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8;
      }
    });
  }
}

impl GpuProcessor for SelectiveColor {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    // Nine rows of four floats, then one row whose first float is 1 for the relative method, as the shader's struct lays them out.
    let method = [if self.method == SelectiveMethod::Relative { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0];
    let uniforms: Vec<u8> = self.adjustments.iter().chain(std::iter::once(&method)).flatten().flat_map(|value| value.to_le_bytes()).collect();
    vec![GpuPass::new(include_str!("./selective_color.wgsl"), uniforms)]
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;
  use gpu::{GpuContext, LiveRenderer};

  fn changed(p_effect: SelectiveColor, p_pixels: Vec<u8>) -> Vec<u8> {
    let mut image = Image::new_from_pixels(p_pixels.len() as u32 / 4, 1, p_pixels, Channels::RGBA);
    p_effect.apply_on_cpu(&mut image);
    image.rgba().to_vec()
  }

  #[test]
  fn with_no_range_set_nothing_changes() {
    let pixels = vec![10, 200, 30, 255, 0, 0, 0, 7, 255, 255, 255, 1, 128, 100, 90, 255];
    assert_eq!(changed(selective_color(), pixels.clone()), pixels);
  }

  #[test]
  fn adding_ink_to_the_reds_darkens_a_red_and_leaves_a_green_alone() {
    let effect = selective_color().with_method(SelectiveMethod::Absolute).with_range(ColorRange::Reds, 0.5, 0.0, 0.0, 0.0);
    // Pure red loses half its red (cyan ink absorbs red); pure green is not in the reds at all.
    assert_eq!(changed(effect, vec![255, 0, 0, 255, 0, 255, 0, 255]), vec![128, 0, 0, 255, 0, 255, 0, 255]);
  }

  #[test]
  fn taking_black_from_the_blacks_lightens_black_to_gray_and_leaves_white_alone() {
    let effect = selective_color().with_method(SelectiveMethod::Absolute).with_range(ColorRange::Blacks, 0.0, 0.0, 0.0, -0.5);
    assert_eq!(changed(effect, vec![0, 0, 0, 9, 255, 255, 255, 9]), vec![128, 128, 128, 9, 255, 255, 255, 9]);
  }

  #[test]
  fn a_relative_change_does_nothing_to_a_pixel_with_none_of_that_ink() {
    // Pure red has no cyan ink, so a relative change of the cyan has nothing to work on; an absolute one does.
    let relative = selective_color().with_range(ColorRange::Reds, 1.0, 0.0, 0.0, 0.0);
    assert_eq!(changed(relative, vec![255, 0, 0, 255]), vec![255, 0, 0, 255]);
  }

  #[test]
  fn taking_yellow_from_the_yellows_makes_a_yellow_less_yellow() {
    let effect = selective_color().with_range(ColorRange::Yellows, 0.0, 0.0, -1.0, 0.0);
    let pixel = changed(effect, vec![220, 200, 60, 255]);
    // Yellow ink is absorbed blue, so less of it leaves more blue.
    assert!(pixel[2] > 60 && pixel[0] == 220 && pixel[1] == 200, "{pixel:?}");
  }

  #[test]
  fn the_gpu_gives_the_same_pixels_as_the_cpu_to_a_level() -> anyhow::Result<()> {
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let pixels: Vec<u8> =
      (0..65536u32).flat_map(|i| [i as u8, (i >> 8) as u8, (i * 37 % 256) as u8, (i * 13 % 256) as u8]).collect();
    for method in [SelectiveMethod::Relative, SelectiveMethod::Absolute] {
      let effect = selective_color()
        .with_method(method)
        .with_range(ColorRange::Reds, -0.3, 0.2, 0.4, 0.1)
        .with_range(ColorRange::Yellows, 0.0, -0.5, 0.6, -0.2)
        .with_range(ColorRange::Neutrals, 0.1, 0.0, 0.3, -0.4)
        .with_range(ColorRange::Blacks, 0.0, 0.0, 0.0, -0.8)
        .with_range(ColorRange::Whites, 0.2, 0.2, 0.0, 0.5);
      let actual = renderer.process(&[effect.passes(256, 256)], 256, 256, &pixels)?;
      let mut expected = Image::new_from_pixels(256, 256, pixels.clone(), Channels::RGBA);
      effect.apply_on_cpu(&mut expected);
      let worst = actual.iter().zip(expected.rgba()).map(|(a, e)| (*a as i16 - *e as i16).abs()).max().unwrap_or(0);
      assert!(worst <= 1, "{method:?}: the GPU is {worst} levels from the CPU");
    }
    Ok(())
  }
}
