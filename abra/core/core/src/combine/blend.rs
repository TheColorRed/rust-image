//! Compositing one image onto another with a blend mode.

use crate::{Image, IntoNumber, PointF, Rect, hsl_to_rgb, rgb_to_hsl};
use rayon::prelude::*;

/// A color with red, green, blue, and alpha channels.
pub type RGBA = (u8, u8, u8, u8);

/// How a source color combines with the destination color beneath it, before the source's alpha composites the
/// result over the destination. The names and behavior follow the common image-editor blend modes.
#[derive(Clone, Copy, Debug, Default)]
pub enum BlendMode {
  /// The source color replaces the destination. Default.
  #[default]
  Normal,
  /// The darker of each channel.
  Darken,
  /// The darker of the two colors, by lightness.
  DarkerColor,
  /// The mean of each channel.
  Average,
  /// Multiplies the channels. Always darker; white leaves the color unchanged.
  Multiply,
  /// Darkens the destination by increasing contrast toward the source. White leaves the color unchanged.
  ColorBurn,
  /// Darkens the destination by decreasing brightness. White leaves the color unchanged.
  LinearBurn,
  /// The lighter of each channel.
  Lighten,
  /// The lighter of the two colors, by lightness.
  LighterColor,
  /// Multiplies the inverses. Always lighter; black leaves the color unchanged.
  Screen,
  /// Brightens the destination by decreasing contrast toward the source. Black leaves the color unchanged.
  ColorDodge,
  /// Adds the channels. Black leaves the color unchanged.
  LinearDodge,
  /// Multiplies dark destination values and screens light ones, keeping the destination's highlights and shadows.
  Overlay,
  /// A gentle, diffused-light version of hard light.
  SoftLight,
  /// Multiplies or screens depending on the source, like a harsh spotlight.
  HardLight,
  /// Color burn for dark source values and color dodge for light ones.
  VividLight,
  /// Linear burn for dark source values and linear dodge for light ones.
  LinearLight,
  /// Darken for dark source values and lighten for light ones.
  PinLight,
  /// Vivid light snapped to 0 or 255 per channel.
  HardMix,
  /// The absolute difference of each channel.
  Difference,
  /// A lower-contrast difference.
  Exclusion,
  /// The source subtracted from the destination.
  Subtract,
  /// The destination divided by the source.
  Divide,
  /// The destination's saturation and lightness with the source's hue.
  Hue,
  /// The destination's hue and lightness with the source's saturation.
  Saturation,
  /// The destination's lightness with the source's hue and saturation.
  Color,
  /// The destination's hue and saturation with the source's lightness.
  Luminosity,
  /// Squares the destination over the inverted source, for shiny highlights.
  Reflect,
  /// Reflect with the layers swapped.
  Glow,
  /// Intense highlights and deep shadows.
  Phoenix,
  /// An inverted difference, like a film negative.
  Negation,
  /// Extracts film grain from the destination.
  GrainExtract,
  /// Merges grain from both layers.
  GrainMerge,
  /// A custom function taking `(destination, source)` and returning the blended color.
  Custom(fn(RGBA, RGBA) -> RGBA),
}

impl BlendMode {
  /// Every built-in mode, in menu order.
  pub const ALL: [BlendMode; 33] = [
    BlendMode::Normal,
    BlendMode::Darken,
    BlendMode::DarkerColor,
    BlendMode::Average,
    BlendMode::Multiply,
    BlendMode::ColorBurn,
    BlendMode::LinearBurn,
    BlendMode::Lighten,
    BlendMode::LighterColor,
    BlendMode::Screen,
    BlendMode::ColorDodge,
    BlendMode::LinearDodge,
    BlendMode::Overlay,
    BlendMode::SoftLight,
    BlendMode::HardLight,
    BlendMode::VividLight,
    BlendMode::LinearLight,
    BlendMode::PinLight,
    BlendMode::HardMix,
    BlendMode::Difference,
    BlendMode::Exclusion,
    BlendMode::Subtract,
    BlendMode::Divide,
    BlendMode::Hue,
    BlendMode::Saturation,
    BlendMode::Color,
    BlendMode::Luminosity,
    BlendMode::Reflect,
    BlendMode::Glow,
    BlendMode::Phoenix,
    BlendMode::Negation,
    BlendMode::GrainExtract,
    BlendMode::GrainMerge,
  ];

  /// The mode's identifier, such as `"color-burn"`. Custom modes are `"custom"`.
  pub fn name(&self) -> &'static str {
    self.names().0
  }

  /// The mode's display label, such as `"Color Burn"`. Custom modes are `"Custom"`.
  pub fn label(&self) -> &'static str {
    self.names().1
  }

  /// The built-in mode with the given [`BlendMode::name`], or `None` when there is none.
  pub fn from_name(p_name: &str) -> Option<BlendMode> {
    BlendMode::ALL.into_iter().find(|mode| mode.name() == p_name)
  }

  fn names(&self) -> (&'static str, &'static str) {
    match self {
      BlendMode::Normal => ("normal", "Normal"),
      BlendMode::Darken => ("darken", "Darken"),
      BlendMode::DarkerColor => ("darker-color", "Darker Color"),
      BlendMode::Average => ("average", "Average"),
      BlendMode::Multiply => ("multiply", "Multiply"),
      BlendMode::ColorBurn => ("color-burn", "Color Burn"),
      BlendMode::LinearBurn => ("linear-burn", "Linear Burn"),
      BlendMode::Lighten => ("lighten", "Lighten"),
      BlendMode::LighterColor => ("lighter-color", "Lighter Color"),
      BlendMode::Screen => ("screen", "Screen"),
      BlendMode::ColorDodge => ("color-dodge", "Color Dodge"),
      BlendMode::LinearDodge => ("linear-dodge", "Linear Dodge"),
      BlendMode::Overlay => ("overlay", "Overlay"),
      BlendMode::SoftLight => ("soft-light", "Soft Light"),
      BlendMode::HardLight => ("hard-light", "Hard Light"),
      BlendMode::VividLight => ("vivid-light", "Vivid Light"),
      BlendMode::LinearLight => ("linear-light", "Linear Light"),
      BlendMode::PinLight => ("pin-light", "Pin Light"),
      BlendMode::HardMix => ("hard-mix", "Hard Mix"),
      BlendMode::Difference => ("difference", "Difference"),
      BlendMode::Exclusion => ("exclusion", "Exclusion"),
      BlendMode::Subtract => ("subtract", "Subtract"),
      BlendMode::Divide => ("divide", "Divide"),
      BlendMode::Hue => ("hue", "Hue"),
      BlendMode::Saturation => ("saturation", "Saturation"),
      BlendMode::Color => ("color", "Color"),
      BlendMode::Luminosity => ("luminosity", "Luminosity"),
      BlendMode::Reflect => ("reflect", "Reflect"),
      BlendMode::Glow => ("glow", "Glow"),
      BlendMode::Phoenix => ("phoenix", "Phoenix"),
      BlendMode::Negation => ("negation", "Negation"),
      BlendMode::GrainExtract => ("grain-extract", "Grain Extract"),
      BlendMode::GrainMerge => ("grain-merge", "Grain Merge"),
      BlendMode::Custom(_) => ("custom", "Custom"),
    }
  }

  /// Blends a source color over a destination color, returning the blend color. The source's alpha is applied
  /// afterwards by [`BlendImage::apply`].
  /// - `p_destination`: The color underneath (the base).
  /// - `p_source`: The color being blended in.
  pub fn apply(&self, p_destination: RGBA, p_source: RGBA) -> RGBA {
    let (a, b) = (p_destination, p_source);
    match self {
      BlendMode::Normal => b,
      BlendMode::Custom(mode) => mode(a, b),
      BlendMode::DarkerColor => {
        if lightness(a) < lightness(b) {
          a
        } else {
          b
        }
      }
      BlendMode::LighterColor => {
        if lightness(a) > lightness(b) {
          a
        } else {
          b
        }
      }
      BlendMode::Hue => hsl_mix(a, b, [true, false, false]),
      BlendMode::Saturation => hsl_mix(a, b, [false, true, false]),
      BlendMode::Color => hsl_mix(a, b, [true, true, false]),
      BlendMode::Luminosity => hsl_mix(a, b, [false, false, true]),

      // Channel modes that blend alpha the same way as color.
      BlendMode::Darken => per_channel(a, b, true, |a, b| a.min(b)),
      BlendMode::Lighten => per_channel(a, b, true, |a, b| a.max(b)),
      BlendMode::Average => per_channel(a, b, true, |a, b| ((a as u16 + b as u16) / 2) as u8),
      BlendMode::Multiply => per_channel(a, b, true, |a, b| (a as u16 * b as u16 / 255) as u8),
      BlendMode::ColorBurn => per_channel(a, b, true, color_burn),
      BlendMode::LinearBurn => per_channel(a, b, true, linear_burn),
      BlendMode::Screen => {
        per_channel(a, b, true, |a, b| (255.0 - (255.0 - a as f32) * (255.0 - b as f32) / 255.0) as u8)
      }
      BlendMode::ColorDodge => per_channel(a, b, true, color_dodge),
      BlendMode::LinearDodge => per_channel(a, b, true, linear_dodge),
      BlendMode::Overlay => per_channel(a, b, true, |a, b| {
        if a < 128 { multiply_or_screen(a, b).round() as u8 } else { screen2(a, b).round() as u8 }
      }),
      BlendMode::SoftLight => per_channel(a, b, true, |a, b| {
        let hard = if b < 128 { multiply_or_screen(a, b) } else { screen2(a, b) };
        (hard * 0.5 + a as f32 * 0.5) as u8
      }),
      BlendMode::HardLight => {
        per_channel(a, b, true, |a, b| if b < 128 { multiply_or_screen(a, b) as u8 } else { screen2(a, b) as u8 })
      }
      BlendMode::VividLight => per_channel(a, b, true, vivid_light),
      BlendMode::LinearLight => per_channel(a, b, true, |a, b| {
        if b < 128 { linear_burn(a, (2.0 * b as f32) as u8) } else { linear_dodge(a, (2.0 * (b as f32 - 128.0)) as u8) }
      }),
      BlendMode::PinLight => per_channel(a, b, true, |a, b| if b < 128 { a.min(b) } else { a.max(b) }),
      BlendMode::HardMix => per_channel(a, b, true, |a, b| if vivid_light(a, b) < 128 { 0 } else { 255 }),
      BlendMode::Reflect => per_channel(a, b, true, reflect),
      BlendMode::Glow => per_channel(a, b, true, |a, b| reflect(b, a)),

      // Channel modes that keep the destination's alpha.
      BlendMode::Difference => per_channel(a, b, false, |a, b| a.abs_diff(b)),
      BlendMode::Exclusion => {
        per_channel(a, b, false, |a, b| (a as i32 + b as i32 - 2 * a as i32 * b as i32 / 255) as u8)
      }
      BlendMode::Subtract => per_channel(a, b, false, |a, b| a.saturating_sub(b)),
      BlendMode::Divide => {
        per_channel(a, b, false, |a, b| if b == 0 { 0 } else { (a as f32 / b as f32 * 255.0).round() as u8 })
      }
      BlendMode::Phoenix => per_channel(a, b, false, |a, b| {
        let (a, b) = (a as f32, b as f32);
        (a + b - 2.0 * a * b / 255.0).clamp(0.0, 255.0) as u8
      }),
      BlendMode::Negation => {
        per_channel(a, b, false, |a, b| (255.0 - (a as f32 - b as f32).abs()).clamp(0.0, 255.0) as u8)
      }
      BlendMode::GrainExtract => {
        per_channel(a, b, false, |a, b| ((a as i32 + b as i32 - 255) / 2 + 128).clamp(0, 255) as u8)
      }
      BlendMode::GrainMerge => per_channel(a, b, false, |a, b| ((a as i32 + b as i32 - 128) / 2).clamp(0, 255) as u8),
    }
  }
}

impl PartialEq for BlendMode {
  /// Built-in modes are equal when they are the same mode; custom modes when they are the same function.
  fn eq(&self, p_other: &Self) -> bool {
    match (self, p_other) {
      (BlendMode::Custom(a), BlendMode::Custom(b)) => std::ptr::fn_addr_eq(*a, *b),
      _ => std::mem::discriminant(self) == std::mem::discriminant(p_other),
    }
  }
}

impl Eq for BlendMode {}

impl From<fn(RGBA, RGBA) -> RGBA> for BlendMode {
  fn from(p_mode: fn(RGBA, RGBA) -> RGBA) -> Self {
    BlendMode::Custom(p_mode)
  }
}

/// Applies `p_blend` to the red, green, and blue channels, and to alpha when `p_blend_alpha` is set; otherwise the
/// destination's alpha is kept.
#[inline]
fn per_channel(p_a: RGBA, p_b: RGBA, p_blend_alpha: bool, p_blend: impl Fn(u8, u8) -> u8) -> RGBA {
  let alpha = if p_blend_alpha { p_blend(p_a.3, p_b.3) } else { p_a.3 };
  (p_blend(p_a.0, p_b.0), p_blend(p_a.1, p_b.1), p_blend(p_a.2, p_b.2), alpha)
}

/// Rebuilds the destination color in HSL, taking hue, saturation, and lightness from the source where
/// `p_from_source` is set (in that order). The destination's alpha is kept.
fn hsl_mix(p_a: RGBA, p_b: RGBA, p_from_source: [bool; 3]) -> RGBA {
  let base = rgb_to_hsl(p_a.0, p_a.1, p_a.2);
  let blend = rgb_to_hsl(p_b.0, p_b.1, p_b.2);
  let pick = |i: usize, base: f32, blend: f32| if p_from_source[i] { blend } else { base };
  let (r, g, b) = hsl_to_rgb(pick(0, base.0, blend.0), pick(1, base.1, blend.1), pick(2, base.2, blend.2));
  (r, g, b, p_a.3)
}

/// HSL lightness of a color.
fn lightness(p_color: RGBA) -> f32 {
  rgb_to_hsl(p_color.0, p_color.1, p_color.2).2
}

/// `2ab/255`: the multiply half of overlay and the light modes.
#[inline]
fn multiply_or_screen(p_a: u8, p_b: u8) -> f32 {
  2.0 * p_a as f32 * p_b as f32 / 255.0
}

/// `255 - 2(255-a)(255-b)/255`: the screen half of overlay and the light modes.
#[inline]
fn screen2(p_a: u8, p_b: u8) -> f32 {
  255.0 - 2.0 * (255.0 - p_a as f32) * (255.0 - p_b as f32) / 255.0
}

fn color_burn(p_a: u8, p_b: u8) -> u8 {
  if p_b == 0 { 0 } else { (255.0 - (255.0 - p_a as f32) * 255.0 / p_b as f32) as u8 }
}

fn color_dodge(p_a: u8, p_b: u8) -> u8 {
  if p_b == 255 { 255 } else { (p_a as f32 * 255.0 / (255.0 - p_b as f32)).min(255.0) as u8 }
}

fn linear_burn(p_a: u8, p_b: u8) -> u8 {
  (p_a as i32 + p_b as i32 - 255).max(0) as u8
}

fn linear_dodge(p_a: u8, p_b: u8) -> u8 {
  p_a.saturating_add(p_b)
}

fn vivid_light(p_a: u8, p_b: u8) -> u8 {
  if p_b < 128 {
    color_burn(p_a, (2.0 * p_b as f32) as u8)
  } else {
    color_dodge(p_a, (2.0 * (p_b as f32 - 128.0)) as u8)
  }
}

fn reflect(p_a: u8, p_b: u8) -> u8 {
  if p_b == 255 { 255 } else { ((p_a as f32 * p_a as f32) / (255.0 - p_b as f32)).min(255.0) as u8 }
}

/// A blend that has been described but not yet run. Create one with [`blend`], optionally set the offset, opacity,
/// and mode, then composite it onto a destination with [`BlendImage::apply`].
#[derive(Clone, Copy)]
pub struct BlendImage<'a> {
  /// The image composited onto the destination.
  pub source: &'a Image,
  /// Position of the source image in destination pixels. Rounded to whole pixels.
  pub offset: PointF,
  /// Opacity applied to the source alpha before alpha-over compositing.
  pub opacity: f32,
  /// How the source color combines with the destination.
  pub mode: BlendMode,
}

impl<'a> BlendImage<'a> {
  /// Sets the position of the source in destination pixels. Defaults to `(0, 0)`.
  pub fn with_offset(mut self, p_offset: impl Into<PointF>) -> Self {
    self.offset = p_offset.into();
    self
  }

  /// Sets the opacity applied to the source, clamped to `0.0..=1.0`. Defaults to `1.0`.
  pub fn with_opacity(mut self, p_opacity: impl IntoNumber) -> Self {
    self.opacity = p_opacity.into::<f32>();
    self
  }

  /// Sets the blend mode, such as [`BlendMode::Multiply`]. Defaults to [`BlendMode::Normal`].
  pub fn with_mode(mut self, p_mode: impl Into<BlendMode>) -> Self {
    self.mode = p_mode.into();
    self
  }

  /// Composites the source onto the destination. The source's alpha, multiplied by the opacity, is composited over
  /// the destination exactly once. Parts of the source outside the destination are ignored, and only the
  /// destination pixels under the source are touched.
  pub fn apply(&self, p_destination: &mut Image) {
    let opacity = self.opacity.clamp(0.0, 1.0);
    let (offset_x, offset_y) = (self.offset.x.round() as i64, self.offset.y.round() as i64);
    let (destination_width, destination_height) = p_destination.dimensions::<u32>();
    let (source_width, source_height) = self.source.dimensions::<u32>();
    let overlap = Rect::new((offset_x as f32, offset_y as f32), (source_width, source_height))
      .intersect(Rect::new((0, 0), (destination_width, destination_height)));
    if overlap.is_empty() || opacity == 0.0 {
      return;
    }
    let (left, top, right, bottom) = overlap.edges::<i64>();

    let source = self.source.rgba();
    let source_stride = source_width as usize * 4;
    let destination_stride = destination_width as usize * 4;
    let mode = self.mode;
    p_destination
      .colors()
      .as_slice_mut()
      .expect("Image colors must be contiguous")
      .par_chunks_exact_mut(destination_stride)
      .skip(top as usize)
      .take((bottom - top) as usize)
      .enumerate()
      .for_each(|(row_index, row)| {
        let source_y = (top + row_index as i64 - offset_y) as usize;
        for x in left..right {
          let d = x as usize * 4;
          let s = source_y * source_stride + (x - offset_x) as usize * 4;
          let destination = (row[d], row[d + 1], row[d + 2], row[d + 3]);
          let source = (source[s], source[s + 1], source[s + 2], source[s + 3]);
          let color = alpha_over(destination, mode.apply(destination, source), source.3 as f32 / 255.0 * opacity);
          row[d..d + 4].copy_from_slice(&[color.0, color.1, color.2, color.3]);
        }
      });
  }
}

/// Composite a source image onto a destination image.
/// # Arguments
/// - `p_source`: The image to composite.
///
/// By default the source is placed at `(0, 0)` at full opacity with [`BlendMode::Normal`]. Change these with
/// [`BlendImage::with_offset`], [`BlendImage::with_opacity`], and [`BlendImage::with_mode`].
pub fn blend(p_source: &Image) -> BlendImage<'_> {
  BlendImage {
    source: p_source,
    offset: PointF::zero(),
    opacity: 1.0,
    mode: BlendMode::Normal,
  }
}

/// Composites a blend-color source over a destination pixel using source alpha.
fn alpha_over(p_destination: RGBA, p_source_color: RGBA, p_source_alpha: f32) -> RGBA {
  let destination_alpha = p_destination.3 as f32 / 255.0;
  let output_alpha = p_source_alpha + destination_alpha * (1.0 - p_source_alpha);
  if output_alpha == 0.0 {
    return (0, 0, 0, 0);
  }

  let blend_channel = |destination: u8, source: u8| {
    ((source as f32 * p_source_alpha + destination as f32 * destination_alpha * (1.0 - p_source_alpha)) / output_alpha)
      .round() as u8
  };
  (
    blend_channel(p_destination.0, p_source_color.0),
    blend_channel(p_destination.1, p_source_color.1),
    blend_channel(p_destination.2, p_source_color.2),
    (output_alpha * 255.0).round() as u8,
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Channels;

  fn image(p_width: u32, p_height: u32, p_pixels: &[u8]) -> Image {
    Image::new_from_pixels(p_width, p_height, p_pixels, Channels::RGBA)
  }

  #[test]
  fn blends_source_at_its_offset() {
    let mut destination = image(3, 1, &[0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]);
    let source = image(1, 1, &[255, 255, 0, 255]);

    blend(&source).with_offset((1, 0)).apply(&mut destination);

    assert_eq!(destination.rgba(), &[0, 0, 0, 255, 255, 255, 0, 255, 0, 0, 0, 255]);
  }

  #[test]
  fn offsets_past_the_edges_clip_the_source() {
    let mut destination = image(2, 2, &[0; 16]);
    let source = image(2, 2, &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 9, 9, 9, 255]);
    blend(&source).with_offset((-1, 1)).apply(&mut destination);
    assert_eq!(destination.get_pixel(0, 1), Some((0, 255, 0, 255)));
    assert_eq!(destination.get_pixel(1, 1), Some((0, 0, 0, 0)));
    assert_eq!(destination.get_pixel(0, 0), Some((0, 0, 0, 0)));
  }

  #[test]
  fn blends_source_alpha_and_opacity_once() {
    let mut destination = image(1, 1, &[255, 0, 0, 255]);
    let source = image(1, 1, &[0, 0, 255, 128]);

    blend(&source).with_opacity(0.5).apply(&mut destination);

    assert_eq!(destination.rgba(), &[191, 0, 64, 255]);
  }

  #[test]
  fn transparent_source_leaves_destination_unchanged() {
    let mut destination = image(1, 1, &[12, 34, 56, 200]);
    let source = image(1, 1, &[255, 0, 0, 0]);

    blend(&source).apply(&mut destination);

    assert_eq!(destination.rgba(), &[12, 34, 56, 200]);
  }

  #[test]
  fn transparent_destination_receives_source_opacity() {
    let mut destination = image(1, 1, &[0, 0, 0, 0]);
    let source = image(1, 1, &[0, 255, 0, 128]);

    blend(&source).with_opacity(0.5).apply(&mut destination);

    assert_eq!(destination.rgba(), &[0, 255, 0, 64]);
  }

  #[test]
  fn modes_round_trip_through_their_names() {
    for mode in BlendMode::ALL {
      assert_eq!(BlendMode::from_name(mode.name()), Some(mode));
    }
    assert_eq!(BlendMode::from_name("nope"), None);
  }

  #[test]
  fn channel_modes_match_their_formulas() {
    let (a, b) = ((200, 100, 50, 255), (100, 200, 25, 128));
    assert_eq!(BlendMode::Multiply.apply(a, b), (78, 78, 4, 128));
    assert_eq!(BlendMode::Darken.apply(a, b), (100, 100, 25, 128));
    assert_eq!(BlendMode::Difference.apply(a, b), (100, 100, 25, 255));
    assert_eq!(BlendMode::Subtract.apply(a, b), (100, 0, 25, 255));
    assert_eq!(BlendMode::LinearDodge.apply(a, b), (255, 255, 75, 255));
    assert_eq!(BlendMode::HardMix.apply((255, 0, 0, 255), (255, 0, 0, 255)).0, 255);
    // Hue takes the source hue onto the destination's saturation and lightness.
    assert_eq!(BlendMode::Hue.apply((255, 0, 0, 255), (0, 0, 255, 9)), (0, 0, 255, 255));
    assert_eq!(BlendMode::Luminosity.apply((255, 0, 0, 255), (0, 0, 0, 255)), (0, 0, 0, 255));
  }
}
