//! Effects from `adjustments`: the `color` and `levels` modules.
//!
//! An effect is made by its function and then applied to an image, as in the library: `posterize(6).apply(image)`. The
//! functions every effect has from the library's `Effect` trait are written for each handle by [`effect_methods!`].

use std::sync::{Arc, RwLock};

use abra::adjustments::prelude::{color, levels};

use crate::effect::effect_methods;
use crate::{Color, Curve, Gradient, Image};
use abra::abra_core::{Image as CoreImage, PointF};

/// Reduces an image to this many levels of each color channel. Made by [`posterize`].
#[derive(uniffi::Object)]
pub struct Posterize(color::Posterize);

/// A posterize effect with this many levels per color channel, 2 to 255.
#[uniffi::export]
pub fn posterize(levels: u8) -> Arc<Posterize> {
  Arc::new(Posterize(color::posterize(levels)))
}

/// Merges the colors of an image into a few: similar colors are grouped, and every pixel is painted with the average color of its
/// group. Made by [`quantize`].
#[derive(uniffi::Object)]
pub struct Quantize(color::Quantize);

/// A merge of the image's colors into this many average colors. 0 is taken as 1.
#[uniffi::export]
pub fn quantize(colors: u32) -> Arc<Quantize> {
  Arc::new(Quantize(color::quantize(colors)))
}

#[uniffi::export]
impl Quantize {
  /// The most times the groups' centers are moved to the average of their colors. Defaults to 8.
  pub fn with_iterations(&self, iterations: u32) -> Arc<Quantize> {
    Arc::new(Quantize(self.0.clone().with_iterations(iterations)))
  }

  /// About how many pixels are looked at to find the groups. Every pixel is still painted. Defaults to 20000.
  pub fn with_sample(&self, pixels: u32) -> Arc<Quantize> {
    Arc::new(Quantize(self.0.clone().with_sample(pixels)))
  }

  /// How different two of the found colors may be, as the distance between them in the Lab color space, and still be merged into
  /// one color. Defaults to 0, which merges nothing.
  pub fn with_tolerance(&self, distance: f32) -> Arc<Quantize> {
    Arc::new(Quantize(self.0.clone().with_tolerance(distance)))
  }

  /// How much a difference in brightness counts, against a difference in color, when deciding how alike two colors are, in merging
  /// found colors and in finding the nearest color of each pixel. Defaults to 1.
  pub fn with_lightness(&self, weight: f32) -> Arc<Quantize> {
    Arc::new(Quantize(self.0.clone().with_lightness(weight)))
  }

  /// The colors that decide which palette color a pixel takes, one for each color of the palette: a pixel takes the palette color
  /// that goes with the source color nearest to it.
  pub fn with_sources(&self, sources: Vec<Arc<Color>>) -> Arc<Quantize> {
    Arc::new(Quantize(self.0.clone().with_sources(sources.iter().map(|color| color.0.clone()))))
  }

  /// Paints with these colors instead of finding them from the image.
  pub fn with_palette(&self, palette: Vec<Arc<Color>>) -> Arc<Quantize> {
    Arc::new(Quantize(self.0.clone().with_palette(palette.iter().map(|color| color.0.clone()))))
  }

  /// The colors the pixels of `image` are painted with: the palette given, or else the average colors found from the image.
  pub fn palette(&self, image: Arc<Image>) -> Vec<Arc<Color>> {
    let image: CoreImage = image.core();
    self.0.palette(&image).into_iter().map(|color| Arc::new(Color(color))).collect()
  }
}

/// Turns every pixel black or white: white where the mean of its red, green and blue is above the threshold. Made by
/// [`threshold`].
#[derive(uniffi::Object)]
pub struct Threshold(color::Threshold);

/// A threshold effect, a value from 0 to 255.
#[uniffi::export]
pub fn threshold(value: u8) -> Arc<Threshold> {
  Arc::new(Threshold(color::threshold(value)))
}

/// Makes the colors of the image sensible on their own. Made by [`auto_color`].
#[derive(uniffi::Object)]
pub struct AutoColor(color::AutoColor);

/// An auto color effect.
#[uniffi::export]
pub fn auto_color() -> Arc<AutoColor> {
  Arc::new(AutoColor(color::auto_color()))
}

/// Stretches the tones of the image. Made by [`auto_tone`].
#[derive(uniffi::Object)]
pub struct AutoTone(color::AutoTone);

/// An auto tone effect.
#[uniffi::export]
pub fn auto_tone() -> Arc<AutoTone> {
  Arc::new(AutoTone(color::auto_tone()))
}

/// Maps the luminance of the image to a gradient. Made by [`gradient_map`].
#[derive(uniffi::Object)]
pub struct GradientMap(color::GradientMap);

/// Maps image luminance to the gradient.
#[uniffi::export]
pub fn gradient_map(gradient: Arc<Gradient>) -> Arc<GradientMap> {
  Arc::new(GradientMap(color::gradient_map(gradient.0.clone())))
}

/// Turns the image gray. Made by [`grayscale`].
#[derive(uniffi::Object)]
pub struct Grayscale(color::Grayscale);

/// A grayscale effect.
#[uniffi::export]
pub fn grayscale() -> Arc<Grayscale> {
  Arc::new(Grayscale(color::grayscale()))
}

/// Inverts the colors of the image. Made by [`invert`].
#[derive(uniffi::Object)]
pub struct Invert(color::Invert);

/// An invert effect.
#[uniffi::export]
pub fn invert() -> Arc<Invert> {
  Arc::new(Invert(color::invert()))
}

/// Multiplies the image by a color. Made by [`color_multiply`].
#[derive(uniffi::Object)]
pub struct ColorMultiply(color::ColorMultiply);

/// A multiply of the image by this color.
#[uniffi::export]
pub fn color_multiply(color: Arc<Color>) -> Arc<ColorMultiply> {
  Arc::new(ColorMultiply(color::color_multiply(color.0)))
}

/// Mixes the color channels: each channel of the result is a weighted sum of the red, green and blue channels. Made by
/// [`channel_mix`].
#[derive(uniffi::Object)]
pub struct ChannelMix(color::ChannelMix);

/// A channel mixer. Each argument is the weights, `[red, green, blue]`, of the source's channels that make one channel of the
/// result, so `[1, 0, 0]`, `[0, 1, 0]`, `[0, 0, 1]` changes nothing. An array that is not three numbers panics.
#[uniffi::export]
pub fn channel_mix(red: Vec<f32>, green: Vec<f32>, blue: Vec<f32>) -> Arc<ChannelMix> {
  let weights = |name: &str, values: Vec<f32>| -> [f32; 3] {
    values
      .try_into()
      .unwrap_or_else(|values: Vec<f32>| panic!("the {name} weights need three numbers, got {}", values.len()))
  };
  Arc::new(ChannelMix(color::channel_mix(weights("red", red), weights("green", green), weights("blue", blue))))
}

/// Makes the image more transparent. Made by [`reduce_opacity`].
#[derive(uniffi::Object)]
pub struct Opacity(color::Opacity);

/// An effect that reduces the opacity of the image to `opacity`, from 0 to 1.
#[uniffi::export]
pub fn reduce_opacity(opacity: f32) -> Arc<Opacity> {
  Arc::new(Opacity(color::reduce_opacity(opacity)))
}

/// Makes the image lighter or darker. Made by [`brightness`].
#[derive(uniffi::Object)]
pub struct Brightness(levels::Brightness);

/// A brightness adjustment, from -100 to 100.
#[uniffi::export]
pub fn brightness(amount: i32) -> Arc<Brightness> {
  Arc::new(Brightness(levels::brightness(amount)))
}

/// Adds or removes contrast. Made by [`contrast`].
#[derive(uniffi::Object)]
pub struct Contrast(levels::Contrast);

/// A contrast adjustment, from -100 to 100.
#[uniffi::export]
pub fn contrast(amount: f64) -> Arc<Contrast> {
  Arc::new(Contrast(levels::contrast(amount)))
}

/// Adjusts highlight and shadow luminance while preserving color. Made by [`highlights_shadows`].
#[derive(uniffi::Object)]
pub struct HighlightsShadows(levels::HighlightsShadows);

/// Basic slider amounts are -100 to 100; inputs are not clamped. Positive highlights darken bright tones; positive shadows lift dark tones.
#[uniffi::export]
pub fn highlights_shadows(highlights: f64, shadows: f64) -> Arc<HighlightsShadows> {
  Arc::new(HighlightsShadows(levels::highlights_shadows(highlights, shadows)))
}

#[uniffi::export]
impl HighlightsShadows {
  /// Shadow tonal width as a fraction, default 0.5 (50%). Input is not clamped.
  pub fn with_shadow_tone(&self, tone: f64) -> Arc<HighlightsShadows> {
    Arc::new(HighlightsShadows(self.0.clone().with_shadow_tone(tone)))
  }
  /// Highlight tonal width as a fraction, default 0.5 (50%). Input is not clamped.
  pub fn with_highlight_tone(&self, tone: f64) -> Arc<HighlightsShadows> {
    Arc::new(HighlightsShadows(self.0.clone().with_highlight_tone(tone)))
  }
  /// Sets both tonal widths as fractions without clamping.
  pub fn with_tone(&self, tone: f64) -> Arc<HighlightsShadows> {
    Arc::new(HighlightsShadows(self.0.clone().with_tone(tone)))
  }
  /// Shadow neighborhood radius in pixels, default 30.
  pub fn with_shadow_radius(&self, radius: f64) -> Arc<HighlightsShadows> {
    Arc::new(HighlightsShadows(self.0.clone().with_shadow_radius(radius)))
  }
  /// Highlight neighborhood radius in pixels, default 30.
  pub fn with_highlight_radius(&self, radius: f64) -> Arc<HighlightsShadows> {
    Arc::new(HighlightsShadows(self.0.clone().with_highlight_radius(radius)))
  }
  /// Sets both neighborhood radii in pixels.
  pub fn with_radius(&self, radius: f64) -> Arc<HighlightsShadows> {
    Arc::new(HighlightsShadows(self.0.clone().with_radius(radius)))
  }
  /// Color correction, rounded to an integer without clamping; default 0.
  pub fn with_color(&self, amount: f64) -> Arc<HighlightsShadows> {
    Arc::new(HighlightsShadows(self.0.clone().with_color(amount)))
  }
  /// Midtone contrast without input clamping; default 0.
  pub fn with_midtone(&self, amount: f64) -> Arc<HighlightsShadows> {
    Arc::new(HighlightsShadows(self.0.clone().with_midtone(amount)))
  }
  /// Dark-end histogram clipping as a fraction, default 0. 0.0001 means 0.01%. Uses the CPU.
  pub fn with_black_clip(&self, percent: f64) -> Arc<HighlightsShadows> {
    Arc::new(HighlightsShadows(self.0.clone().with_black_clip(percent)))
  }
  /// Bright-end histogram clipping as a fraction, default 0. 0.0001 means 0.01%. Uses the CPU.
  pub fn with_white_clip(&self, percent: f64) -> Arc<HighlightsShadows> {
    Arc::new(HighlightsShadows(self.0.clone().with_white_clip(percent)))
  }
}

/// Brightens or darkens the image in stops. Made by [`exposure`].
#[derive(uniffi::Object)]
pub struct Exposure(levels::Exposure);

/// An exposure adjustment in stops, from -20 to 20.
#[uniffi::export]
pub fn exposure(stops: f64) -> Arc<Exposure> {
  Arc::new(Exposure(levels::exposure(stops)))
}

#[uniffi::export]
impl Exposure {
  /// The offset added to each color channel, from -0.5 to 0.5. Defaults to 0.
  pub fn with_offset(&self, offset: f64) -> Arc<Exposure> {
    Arc::new(Exposure(self.0.clone().with_offset(offset)))
  }

  /// The gamma correction, from 0.01 to 9.99. 1 means no correction. Defaults to 1.
  pub fn with_gamma(&self, gamma_correction: f64) -> Arc<Exposure> {
    Arc::new(Exposure(self.0.clone().with_gamma(gamma_correction)))
  }
}

/// A levels adjustment: remaps the tones of the image by where its darkest and lightest tones are, how bright its middle is and the range
/// they are put in. Made by [`levels`].
#[derive(uniffi::Object)]
pub struct Levels(levels::Levels);

/// A levels adjustment of an input range, on the scale of a byte (0 to 255): anything at or below `input_black` becomes the darkest tone
/// and anything at or above `input_white` the lightest. The midtone gamma is 1 and the output range 0 to 255 until they are set.
#[uniffi::export]
pub fn levels(input_black: f32, input_white: f32) -> Arc<Levels> {
  Arc::new(Levels(levels::levels(input_black, input_white)))
}

#[uniffi::export]
impl Levels {
  /// The midtone gamma. 1 leaves the middle where it is; above 1 makes it brighter and below 1 darker. Defaults to 1.
  pub fn with_gamma(&self, gamma: f32) -> Arc<Levels> {
    Arc::new(Levels(self.0.clone().with_gamma(gamma)))
  }

  /// The range the result is put in, 0 to 255: the value the darkest tone becomes and the value the lightest becomes.
  pub fn with_output(&self, black: f32, white: f32) -> Arc<Levels> {
    Arc::new(Levels(self.0.clone().with_output(black, white)))
  }
}

/// A channel of a [`Curves`] adjustment that a curve can be set for.
#[derive(Clone, Copy, Debug, PartialEq, uniffi::Enum)]
pub enum CurveChannel {
  /// All of the red, green and blue channels together, applied after the curve of each channel on its own.
  Rgb,
  Red,
  Green,
  Blue,
}

impl From<CurveChannel> for levels::CurveChannel {
  fn from(channel: CurveChannel) -> Self {
    match channel {
      CurveChannel::Rgb => levels::CurveChannel::Rgb,
      CurveChannel::Red => levels::CurveChannel::Red,
      CurveChannel::Green => levels::CurveChannel::Green,
      CurveChannel::Blue => levels::CurveChannel::Blue,
    }
  }
}

/// A curves adjustment: remaps the tones of the image by a curve for the red, green and blue channels together and for each of them
/// on its own. Made by [`curves`].
#[derive(uniffi::Object)]
pub struct Curves(levels::Curves);

/// A curves adjustment that changes nothing until a curve is set for a channel.
#[uniffi::export]
pub fn curves() -> Arc<Curves> {
  Arc::new(Curves(levels::curves()))
}

#[uniffi::export]
impl Curves {
  /// Sets the curve of a channel, in place of the one it had. The value of a channel goes through its own curve first and then the `Rgb` one.
  pub fn with_curve(&self, channel: CurveChannel, curve: Arc<Curve>) -> Arc<Curves> {
    Arc::new(Curves(self.0.clone().with_curve(channel.into(), curve.core())))
  }

  /// Sets the curve of a channel to a smooth line through points, each `(input, output)` on a scale of 0 to 255.
  pub fn with_points(&self, channel: CurveChannel, points: Vec<PointF>) -> Arc<Curves> {
    Arc::new(Curves(self.0.clone().with_points(channel.into(), points)))
  }
}

/// The parts of the colors of a picture that [`SelectiveColor`] can change on their own.
#[derive(Clone, Copy, Debug, PartialEq, uniffi::Enum)]
pub enum ColorRange {
  Reds,
  Yellows,
  Greens,
  Cyans,
  Blues,
  Magentas,
  Whites,
  Neutrals,
  Blacks,
}

impl From<ColorRange> for color::ColorRange {
  fn from(range: ColorRange) -> Self {
    match range {
      ColorRange::Reds => color::ColorRange::Reds,
      ColorRange::Yellows => color::ColorRange::Yellows,
      ColorRange::Greens => color::ColorRange::Greens,
      ColorRange::Cyans => color::ColorRange::Cyans,
      ColorRange::Blues => color::ColorRange::Blues,
      ColorRange::Magentas => color::ColorRange::Magentas,
      ColorRange::Whites => color::ColorRange::Whites,
      ColorRange::Neutrals => color::ColorRange::Neutrals,
      ColorRange::Blacks => color::ColorRange::Blacks,
    }
  }
}

/// Whether the changes of a [`SelectiveColor`] are shares of the ink a pixel has or fixed amounts.
#[derive(Clone, Copy, Debug, PartialEq, uniffi::Enum)]
pub enum SelectiveMethod {
  Relative,
  Absolute,
}

impl From<SelectiveMethod> for color::SelectiveMethod {
  fn from(method: SelectiveMethod) -> Self {
    match method {
      SelectiveMethod::Relative => color::SelectiveMethod::Relative,
      SelectiveMethod::Absolute => color::SelectiveMethod::Absolute,
    }
  }
}

/// Changes the colors in one range of a picture without touching the others, as the cyan, magenta, yellow and black inks of a print.
/// Made by [`selective_color`].
#[derive(uniffi::Object)]
pub struct SelectiveColor(color::SelectiveColor);

/// A selective color adjustment that changes nothing until ranges are set with `with_range`.
#[uniffi::export]
pub fn selective_color() -> Arc<SelectiveColor> {
  Arc::new(SelectiveColor(color::selective_color()))
}

#[uniffi::export]
impl SelectiveColor {
  /// How much cyan, magenta, yellow and black one range gains (positive) or loses (negative). `1` and `-1` are the most a slider gives.
  pub fn with_range(&self, range: ColorRange, cyan: f32, magenta: f32, yellow: f32, black: f32) -> Arc<SelectiveColor> {
    Arc::new(SelectiveColor(self.0.clone().with_range(range.into(), cyan, magenta, yellow, black)))
  }

  /// Whether the changes are shares of the ink a pixel has (relative, the default) or fixed amounts (absolute).
  pub fn with_method(&self, method: SelectiveMethod) -> Arc<SelectiveColor> {
    Arc::new(SelectiveColor(self.0.clone().with_method(method.into())))
  }
}

/// Rotates the hues of the image. Made by [`hue`].
#[derive(uniffi::Object)]
pub struct Hue(levels::Hue);

/// A hue adjustment.
#[uniffi::export]
pub fn hue(amount: i32) -> Arc<Hue> {
  Arc::new(Hue(levels::hue(amount)))
}

/// Makes the colors more or less intense. Made by [`saturation`].
#[derive(uniffi::Object)]
pub struct Saturation(levels::Saturation);

/// A saturation adjustment, from -100 to 100.
#[uniffi::export]
pub fn saturation(amount: i32) -> Arc<Saturation> {
  Arc::new(Saturation(levels::saturation(amount)))
}

/// Boosts muted colors more than saturated ones. Made by [`vibrance`].
#[derive(uniffi::Object)]
pub struct Vibrance(levels::Vibrance);

/// A vibrance adjustment, from -100 to 100.
#[uniffi::export]
pub fn vibrance(amount: f64) -> Arc<Vibrance> {
  Arc::new(Vibrance(levels::vibrance(amount)))
}

#[uniffi::export]
impl Vibrance {
  /// Also adjusts saturation evenly across all colors, from -100 to 100. Defaults to 0.
  pub fn with_saturation(&self, saturation: f64) -> Arc<Vibrance> {
    Arc::new(Vibrance(self.0.clone().with_saturation(saturation)))
  }
}

/// Types of preset photo filters.
#[derive(Clone, Copy, Debug, PartialEq, uniffi::Enum)]
pub enum FilterType {
  WarmingDark,
  WarmingLight,
  CoolingDark,
  CoolingLight,
  Red,
  Orange,
  Yellow,
  Green,
  Cyan,
  Blue,
  Violet,
  Magenta,
  Sepia,
  DeepRed,
  DeepBlue,
  DeepEmerald,
  DeepYellow,
  Underwater,
}

impl From<FilterType> for levels::FilterType {
  fn from(filter: FilterType) -> Self {
    match filter {
      FilterType::WarmingDark => levels::FilterType::WarmingDark,
      FilterType::WarmingLight => levels::FilterType::WarmingLight,
      FilterType::CoolingDark => levels::FilterType::CoolingDark,
      FilterType::CoolingLight => levels::FilterType::CoolingLight,
      FilterType::Red => levels::FilterType::Red,
      FilterType::Orange => levels::FilterType::Orange,
      FilterType::Yellow => levels::FilterType::Yellow,
      FilterType::Green => levels::FilterType::Green,
      FilterType::Cyan => levels::FilterType::Cyan,
      FilterType::Blue => levels::FilterType::Blue,
      FilterType::Violet => levels::FilterType::Violet,
      FilterType::Magenta => levels::FilterType::Magenta,
      FilterType::Sepia => levels::FilterType::Sepia,
      FilterType::DeepRed => levels::FilterType::DeepRed,
      FilterType::DeepBlue => levels::FilterType::DeepBlue,
      FilterType::DeepEmerald => levels::FilterType::DeepEmerald,
      FilterType::DeepYellow => levels::FilterType::DeepYellow,
      FilterType::Underwater => levels::FilterType::Underwater,
    }
  }
}

/// Selects either a custom filter color or a named preset.
#[derive(Clone, Debug, uniffi::Enum)]
pub enum PhotoFilter {
  Color(Arc<Color>),
  Preset(FilterType),
}

impl From<PhotoFilter> for levels::PhotoFilter {
  fn from(filter: PhotoFilter) -> Self {
    match filter {
      PhotoFilter::Color(color) => levels::PhotoFilter::Color(color.0),
      PhotoFilter::Preset(preset) => levels::PhotoFilter::Preset(preset.into()),
    }
  }
}

/// Tints the image as if shot through a colored lens filter. Made by [`photo_filter`].
#[derive(uniffi::Object)]
pub struct PhotoFilterAdjustment(levels::PhotoFilterAdjustment);

/// Tints the image as if shot through a colored lens filter: a custom color or a preset.
#[uniffi::export]
pub fn photo_filter(filter: PhotoFilter) -> Arc<PhotoFilterAdjustment> {
  Arc::new(PhotoFilterAdjustment(levels::photo_filter(levels::PhotoFilter::from(filter))))
}

#[uniffi::export]
impl PhotoFilterAdjustment {
  /// How strongly the filter is applied, from 0 to 1. Defaults to 0.25.
  pub fn with_density(&self, density: f64) -> Arc<PhotoFilterAdjustment> {
    Arc::new(PhotoFilterAdjustment(self.0.clone().with_density(density)))
  }

  /// Whether the photo's brightness is kept while its color is tinted.
  pub fn with_preserve_luminosity(&self, preserve: bool) -> Arc<PhotoFilterAdjustment> {
    Arc::new(PhotoFilterAdjustment(self.0.clone().with_preserve_luminosity(preserve)))
  }
}

/// A linear gradient drawn over the image. Made by [`LinearGradientEffect::angle`] or [`LinearGradientEffect::between`].
///
/// The library's `set_*` functions change the effect in place, so it is kept behind a lock.
#[derive(uniffi::Object)]
pub struct LinearGradientEffect(RwLock<color::LinearGradientEffect>);

#[uniffi::export]
impl LinearGradientEffect {
  /// A gradient whose line crosses the image center at `degrees`, CSS style: 0 points up and 90 runs left to right.
  #[uniffi::constructor]
  pub fn angle(gradient: Arc<Gradient>, degrees: f32) -> Self {
    Self(RwLock::new(color::LinearGradientEffect::angle(&gradient.0, degrees)))
  }

  /// A gradient from `start` to `end`, each `[x, y]` in image pixels.
  #[uniffi::constructor]
  pub fn between(gradient: Arc<Gradient>, start: Vec<f32>, end: Vec<f32>) -> Self {
    let [start_x, start_y]: [f32; 2] = start.try_into().expect("a point takes 2 values");
    let [end_x, end_y]: [f32; 2] = end.try_into().expect("a point takes 2 values");
    Self(RwLock::new(color::LinearGradientEffect::between(&gradient.0, (start_x, start_y), (end_x, end_y))))
  }

  /// The overall opacity, from 0 to 1, as a new effect.
  pub fn with_opacity(&self, opacity: f32) -> Arc<LinearGradientEffect> {
    Arc::new(Self(RwLock::new(self.0.read().unwrap().clone().with_opacity(opacity))))
  }

  /// Turns the gradient to `degrees`.
  pub fn set_angle(&self, degrees: f32) {
    self.0.write().unwrap().set_angle(degrees);
  }

  /// Moves the gradient line to run from `start` to `end`, each `[x, y]` in image pixels.
  pub fn set_between(&self, start: Vec<f32>, end: Vec<f32>) {
    let [start_x, start_y]: [f32; 2] = start.try_into().expect("a point takes 2 values");
    let [end_x, end_y]: [f32; 2] = end.try_into().expect("a point takes 2 values");
    self.0.write().unwrap().set_between((start_x, start_y), (end_x, end_y));
  }

  /// Sets the overall opacity, from 0 to 1.
  pub fn set_opacity(&self, opacity: f32) {
    self.0.write().unwrap().set_opacity(opacity);
  }

  /// Replaces the colors.
  pub fn set_gradient(&self, gradient: Arc<Gradient>) {
    self.0.write().unwrap().set_gradient(&gradient.0);
  }
}

effect_methods!(
  Posterize,
  Quantize,
  Threshold,
  AutoColor,
  AutoTone,
  GradientMap,
  Grayscale,
  Invert,
  ColorMultiply,
  ChannelMix,
  Opacity,
  Brightness,
  Contrast,
  Exposure,
  Levels,
  Curves,
  SelectiveColor,
  HighlightsShadows,
  Hue,
  Saturation,
  Vibrance,
  PhotoFilterAdjustment,
);
effect_methods!(locked LinearGradientEffect);

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Image;
  use abra::abra_core::Channels;
  use abra::adjustments::prelude::Effect;

  fn pixel(r: u8, g: u8, b: u8, a: u8) -> Arc<Image> {
    Arc::new(Image::new_from_pixels(1, 1, vec![r, g, b, a], Channels::RGBA))
  }

  #[test]
  fn applied_gives_a_changed_copy_and_leaves_the_image_alone() {
    let image = pixel(10, 20, 30, 255);
    let result = invert().applied(image.clone());
    assert_eq!(result.get_pixel(0, 0), Some(vec![245, 235, 225, 255]));
    assert_eq!(image.get_pixel(0, 0), Some(vec![10, 20, 30, 255]));
    // A handle whose effect is behind a lock does the same.
    let gradient = Arc::new(Gradient::from_to(Arc::new(Color::black()), Arc::new(Color::white())));
    let effect = LinearGradientEffect::angle(gradient, 0.0);
    let flat = Arc::new(Image::new_from_pixels(6, 4, picture(), Channels::RGBA));
    let result = effect.applied(flat.clone());
    assert_eq!(flat.rgba(), picture(), "the image is as it was");
    assert_ne!(result.rgba(), picture(), "the copy has the gradient");
  }

  #[test]
  fn posterizing_to_two_levels_makes_each_channel_dark_or_light_and_leaves_alpha() {
    let image = pixel(100, 200, 255, 77);
    posterize(2).apply(image.clone());
    assert_eq!(image.get_pixel(0, 0), Some(vec![0, 255, 255, 77]));
  }

  #[test]
  fn quantizing_merges_close_colors_and_a_given_palette_is_used_as_it_is() {
    let row = || {
      let pixels = vec![200, 30, 30, 255, 205, 34, 28, 255, 30, 30, 210, 255, 36, 28, 205, 255];
      Arc::new(Image::new_from_pixels(4, 1, pixels, Channels::RGBA))
    };
    let image = row();
    quantize(2).apply_on_cpu(image.clone());
    let out = image.rgba();
    assert_eq!(out[0..4], out[4..8], "the two reds became one color");
    assert_eq!(out[8..12], out[12..16], "the two blues became one color");
    assert_ne!(out[0..3], out[8..11]);

    let palette = vec![Arc::new(Color::from_rgb(255, 0, 0)), Arc::new(Color::from_rgb(0, 0, 255))];
    let image = row();
    let effect = quantize(9).with_palette(palette);
    assert_eq!(effect.palette(image.clone()).len(), 2);
    effect.apply_on_cpu(image.clone());
    assert_eq!(image.rgba(), vec![255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255]);
  }

  #[test]
  fn applying_an_effect_later_gives_the_same_pixels_as_applying_it_now() {
    // The GPU provider is global, so a test that registers one makes this wait.
    let _guard = abra::abra_core::image::gpu::GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    abra::abra_core::image::gpu::clear_gpu_provider();
    let now = pixel(100, 200, 50, 77);
    brightness(30).apply_on_cpu(now.clone());
    let later = pixel(100, 200, 50, 77);
    brightness(30).apply_deferred(later.clone());
    assert_eq!(later.rgba(), now.rgba());
  }

  #[test]
  fn an_effect_limited_to_a_mask_or_an_area_changes_only_that_part() {
    let row = || Arc::new(Image::new_from_pixels(2, 1, vec![10, 20, 30, 255, 10, 20, 30, 255], Channels::RGBA));
    let image = row();
    let masked = invert().with_mask(Some(Arc::new(crate::Mask::from_values(2, 1, vec![0, 255]))));
    masked.apply_on_cpu(image.clone());
    assert_eq!(image.rgba(), vec![10, 20, 30, 255, 245, 235, 225, 255]);

    let image = row();
    let area = Arc::new(crate::Area::rect(crate::point_f_new(0.0, 0.0), crate::size_new(1.0, 1.0)));
    invert().with_area(Some(area)).apply_on_cpu(image.clone());
    assert_eq!(image.rgba(), vec![245, 235, 225, 255, 10, 20, 30, 255]);

    let image = row();
    let removed = masked.with_mask(None);
    removed.apply_on_cpu(image.clone());
    assert_eq!(image.rgba(), vec![245, 235, 225, 255, 245, 235, 225, 255], "nothing removes the mask");

    let image = row();
    invert().apply_on_cpu(image.clone());
    assert_eq!(image.rgba()[..4], [245, 235, 225, 255], "the effect it was made from is unchanged");
  }

  #[test]
  fn a_gradient_map_makes_a_tone_mask_that_is_full_at_the_dark_end_and_gone_by_the_tone() {
    use crate::{Color, ColorStop, Gradient, Mask};
    let stop = |value: u8, time: f32| Arc::new(ColorStop::new(Arc::new(Color::from_rgb(value, value, value)), time));
    // Shadows at a tone of 0.5: white at black, black from half way up.
    let gradient = Arc::new(Gradient::new(vec![stop(255, 0.0), stop(0, 0.5), stop(0, 1.0)]));
    let ramp: Vec<u8> = [0u8, 64, 128, 192, 255].iter().flat_map(|&v| [v, v, v, 255]).collect();
    let image = Arc::new(Image::new_from_pixels(5, 1, ramp, Channels::RGBA));
    gradient_map(gradient).apply_on_cpu(image.clone());
    let mask = Mask::from_image(image);
    let values = mask.values();
    assert_eq!(values[0], 255, "black is fully in the mask");
    assert!(values[1] > 100 && values[1] < 160, "a quarter of the way is about half: {}", values[1]);
    assert_eq!(values[2], 0, "half way is out of it");
    assert_eq!(values[4], 0);
  }

  #[test]
  fn highlights_shadows_settings_build_a_new_effect_and_leave_the_original_unchanged() {
    let base = highlights_shadows(100.0, 0.0);
    let disabled = base.with_highlight_tone(0.0);
    let original_result = pixel(220, 220, 220, 91);
    let disabled_result = pixel(220, 220, 220, 91);
    base.apply_on_cpu(original_result.clone());
    disabled.apply_on_cpu(disabled_result.clone());
    assert!(original_result.rgba()[0] < 220);
    assert_eq!(disabled_result.rgba(), vec![220, 220, 220, 91]);
  }

  #[test]
  fn local_tone_masks_and_curves_do_not_reverse_a_gray_ramp() {
    use crate::{ColorStop, Mask};
    let ramp: Vec<u8> = (0..=255u8).flat_map(|gray| [gray, gray, gray, 255]).collect();
    let mask = |shadows: bool| {
      let mut stops: Vec<Arc<ColorStop>> = [0.0f32, 0.25, 0.5, 0.75, 1.0]
        .into_iter()
        .map(|across| {
          let gray = (255.0 * (1.0 - across * across * (3.0 - 2.0 * across))).round() as u8;
          let time = if shadows { across * 0.4 } else { 1.0 - across * 0.4 };
          Arc::new(ColorStop::new(Arc::new(Color::from_rgb(gray, gray, gray)), time))
        })
        .collect();
      stops.push(Arc::new(ColorStop::new(Arc::new(Color::black()), if shadows { 1.0 } else { 0.0 })));
      let image = Arc::new(Image::new_from_pixels(256, 1, ramp.clone(), Channels::RGBA));
      gradient_map(Arc::new(Gradient::new(stops))).apply_on_cpu(image.clone());
      Arc::new(Mask::from_image(image))
    };
    let highlight_mask = mask(false);
    let shadow_mask = mask(true);
    for highlights in (-100..=100).step_by(10) {
      for shadows in (-100..=100).step_by(10) {
        let image = Arc::new(Image::new_from_pixels(256, 1, ramp.clone(), Channels::RGBA));
        highlights_shadows(highlights as f64, 0.0).with_mask(Some(highlight_mask.clone())).apply_on_cpu(image.clone());
        highlights_shadows(0.0, shadows as f64).with_mask(Some(shadow_mask.clone())).apply_on_cpu(image.clone());
        let pixels = image.rgba();
        for index in 1..256 {
          assert!(pixels[index * 4] >= pixels[(index - 1) * 4], "reversed at {index}, H={highlights}, S={shadows}");
          assert_eq!(pixels[index * 4 + 3], 255);
        }
        assert_eq!(&pixels[128 * 4..129 * 4], &ramp[128 * 4..129 * 4], "midgray is outside both masks");
      }
    }
  }

  #[test]
  fn the_result_is_the_librarys() {
    let pixels: Vec<u8> = (0..16).flat_map(|i| [i * 16, 255 - i * 16, i * 7, 255]).collect();
    let image = Arc::new(Image::new_from_pixels(4, 4, pixels.clone(), Channels::RGBA));
    posterize(5).apply_on_cpu(image.clone());
    let mut expected = abra::abra_core::Image::new_from_pixels(4, 4, pixels, Channels::RGBA);
    color::posterize(5).apply(&mut expected);
    assert_eq!(image.rgba(), expected.rgba());
  }

  #[test]
  fn an_image_that_is_still_a_recipe_is_made_first_and_then_posterized() {
    let base = pixel(100, 100, 100, 255);
    // `blended` hands back an image that may not be made yet; applying an effect changes the pixels, so they are made first.
    let top = pixel(200, 200, 200, 255);
    let blended = crate::blended(base, top, crate::BlendMode::Normal, 1.0);
    posterize(2).apply(blended.clone());
    assert_eq!(blended.get_pixel(0, 0), Some(vec![255, 255, 255, 255]));
  }

  #[test]
  #[should_panic(expected = "this operation has no GPU version")]
  fn an_effect_without_a_shader_cannot_be_applied_on_the_gpu() {
    posterize(4).apply_on_gpu(pixel(1, 2, 3, 4));
  }

  #[test]
  fn a_pixel_goes_white_above_the_threshold_and_black_at_or_below_it_and_alpha_is_left() {
    // The mean of 100, 200 and 255 is 185.
    let above = pixel(100, 200, 255, 77);
    threshold(128).apply(above.clone());
    assert_eq!(above.get_pixel(0, 0), Some(vec![255, 255, 255, 77]));
    let below = pixel(10, 20, 30, 9);
    threshold(128).apply_on_cpu(below.clone());
    assert_eq!(below.get_pixel(0, 0), Some(vec![0, 0, 0, 9]));
    let level = pixel(128, 128, 128, 255);
    threshold(128).apply_on_cpu(level.clone());
    assert_eq!(level.get_pixel(0, 0), Some(vec![0, 0, 0, 255]), "a mean equal to the threshold is not above it");
  }

  #[test]
  fn a_channel_mix_makes_one_channel_from_the_others_through_a_mask() {
    let image = Arc::new(Image::new_from_pixels(2, 1, vec![220, 40, 30, 255, 220, 40, 30, 255], Channels::RGBA));
    let mask = Arc::new(crate::Mask::from_values(2, 1, vec![255, 0]));
    channel_mix(vec![0.0, 0.5, 0.5], vec![0.0, 1.0, 0.0], vec![0.0, 0.0, 1.0])
      .with_mask(Some(mask))
      .apply_on_cpu(image.clone());
    assert_eq!(image.rgba(), vec![35, 40, 30, 255, 220, 40, 30, 255], "only the pixel the mask reaches changes");
  }

  #[test]
  #[should_panic(expected = "need three numbers")]
  fn a_channel_mix_needs_three_weights_for_each_channel() {
    channel_mix(vec![1.0, 0.0], vec![0.0, 1.0, 0.0], vec![0.0, 0.0, 1.0]);
  }

  #[test]
  fn the_threshold_result_is_the_librarys() {
    let pixels: Vec<u8> = (0..16).flat_map(|i| [i * 16, 255 - i * 16, i * 7, 255]).collect();
    let image = Arc::new(Image::new_from_pixels(4, 4, pixels.clone(), Channels::RGBA));
    threshold(100).apply_on_cpu(image.clone());
    let mut expected = abra::abra_core::Image::new_from_pixels(4, 4, pixels, Channels::RGBA);
    color::threshold(100).apply_on_cpu(&mut expected);
    assert_eq!(image.rgba(), expected.rgba());
  }

  /// A varied 6 x 4 picture, as pixels.
  fn picture() -> Vec<u8> {
    (0..24u32)
      .flat_map(|i| {
        [
          (i * 10) as u8,
          (255 - i * 9) as u8,
          (i * 37 % 256) as u8,
          if i % 5 == 0 { 120 } else { 255 },
        ]
      })
      .collect()
  }

  /// What the handle's `apply_on_cpu` makes of the picture, and what the library's own effect makes of it.
  fn handle_and_library(
    handle: impl FnOnce(Arc<Image>), library: impl FnOnce(&mut abra::abra_core::Image),
  ) -> (Vec<u8>, Vec<u8>) {
    let image = Arc::new(Image::new_from_pixels(6, 4, picture(), Channels::RGBA));
    handle(image.clone());
    let mut expected = abra::abra_core::Image::new_from_pixels(6, 4, picture(), Channels::RGBA);
    library(&mut expected);
    (image.rgba(), expected.rgba().to_vec())
  }

  #[test]
  fn every_effect_gives_the_pixels_the_library_gives() {
    let gradient = || Arc::new(Gradient::from_to(Arc::new(Color::black()), Arc::new(Color::red())));
    let cases: Vec<(&str, (Vec<u8>, Vec<u8>))> = vec![
      ("auto_color", handle_and_library(|i| auto_color().apply_on_cpu(i), |e| color::auto_color().apply_on_cpu(e))),
      ("auto_tone", handle_and_library(|i| auto_tone().apply_on_cpu(i), |e| color::auto_tone().apply_on_cpu(e))),
      (
        "gradient_map",
        handle_and_library(
          |i| gradient_map(gradient()).apply_on_cpu(i),
          |e| color::gradient_map(gradient().0.clone()).apply_on_cpu(e),
        ),
      ),
      ("grayscale", handle_and_library(|i| grayscale().apply_on_cpu(i), |e| color::grayscale().apply_on_cpu(e))),
      ("invert", handle_and_library(|i| invert().apply_on_cpu(i), |e| color::invert().apply_on_cpu(e))),
      (
        "color_multiply",
        handle_and_library(
          |i| color_multiply(Arc::new(Color::from_rgb(200, 150, 100))).apply_on_cpu(i),
          |e| color::color_multiply(abra::abra_core::Color::from_rgb(200, 150, 100)).apply_on_cpu(e),
        ),
      ),
      (
        "reduce_opacity",
        handle_and_library(|i| reduce_opacity(0.4).apply_on_cpu(i), |e| color::reduce_opacity(0.4).apply_on_cpu(e)),
      ),
      (
        "brightness",
        handle_and_library(|i| brightness(30).apply_on_cpu(i), |e| levels::brightness(30).apply_on_cpu(e)),
      ),
      ("contrast", handle_and_library(|i| contrast(25.0).apply_on_cpu(i), |e| levels::contrast(25.0).apply_on_cpu(e))),
      (
        "highlights_shadows",
        handle_and_library(
          |i| highlights_shadows(40.0, 25.0).apply_on_cpu(i),
          |e| levels::highlights_shadows(40.0, 25.0).apply_on_cpu(e),
        ),
      ),
      (
        "highlights_shadows settings",
        handle_and_library(
          |i| {
            highlights_shadows(40.0, 35.0)
              .with_tone(0.5)
              .with_radius(3.0)
              .with_shadow_tone(0.6)
              .with_highlight_tone(0.45)
              .with_shadow_radius(2.0)
              .with_highlight_radius(4.0)
              .with_color(20.0)
              .with_midtone(15.0)
              .with_black_clip(0.0001)
              .with_white_clip(0.0001)
              .apply_on_cpu(i)
          },
          |e| {
            levels::highlights_shadows(40.0, 35.0)
              .with_tone(0.5)
              .with_radius(3)
              .with_shadow_tone(0.6)
              .with_highlight_tone(0.45)
              .with_shadow_radius(2)
              .with_highlight_radius(4)
              .with_color(20)
              .with_midtone(15)
              .with_black_clip(0.0001)
              .with_white_clip(0.0001)
              .apply_on_cpu(e)
          },
        ),
      ),
      (
        "exposure",
        handle_and_library(
          |i| exposure(0.7).with_offset(0.1).with_gamma(1.2).apply_on_cpu(i),
          |e| levels::exposure(0.7).with_offset(0.1).with_gamma(1.2).apply_on_cpu(e),
        ),
      ),
      ("hue", handle_and_library(|i| hue(40).apply_on_cpu(i), |e| levels::hue(40).apply_on_cpu(e))),
      (
        "saturation",
        handle_and_library(|i| saturation(-35).apply_on_cpu(i), |e| levels::saturation(-35).apply_on_cpu(e)),
      ),
      (
        "vibrance",
        handle_and_library(
          |i| vibrance(40.0).with_saturation(10.0).apply_on_cpu(i),
          |e| levels::vibrance(40.0).with_saturation(10.0).apply_on_cpu(e),
        ),
      ),
      (
        "levels",
        handle_and_library(
          |i| levels(20.0, 230.0).with_gamma(1.6).with_output(10.0, 245.0).apply_on_cpu(i),
          |e| levels::levels(20.0, 230.0).with_gamma(1.6).with_output(10.0, 245.0).apply_on_cpu(e),
        ),
      ),
      (
        "curves",
        handle_and_library(
          |i| {
            curves()
              .with_curve(CurveChannel::Rgb, Arc::new(Curve::ease_in_out(2.0)))
              .with_points(CurveChannel::Blue, vec![PointF::new(0.0, 20.0), PointF::new(255.0, 235.0)])
              .apply_on_cpu(i)
          },
          |e| {
            levels::curves()
              .with_curve(levels::CurveChannel::Rgb, abra::abra_core::Curve::ease_in_out(2.0))
              .with_points(levels::CurveChannel::Blue, [(0.0, 20.0), (255.0, 235.0)])
              .apply_on_cpu(e)
          },
        ),
      ),
      (
        "selective_color",
        handle_and_library(
          |i| {
            selective_color()
              .with_method(SelectiveMethod::Absolute)
              .with_range(ColorRange::Neutrals, 0.1, 0.0, 0.3, -0.3)
              .with_range(ColorRange::Reds, 0.0, -0.2, 0.2, 0.0)
              .apply_on_cpu(i)
          },
          |e| {
            color::selective_color()
              .with_method(color::SelectiveMethod::Absolute)
              .with_range(color::ColorRange::Neutrals, 0.1, 0.0, 0.3, -0.3)
              .with_range(color::ColorRange::Reds, 0.0, -0.2, 0.2, 0.0)
              .apply_on_cpu(e)
          },
        ),
      ),
      (
        "photo_filter preset",
        handle_and_library(
          |i| photo_filter(PhotoFilter::Preset(FilterType::Sepia)).with_density(0.5).apply_on_cpu(i),
          |e| {
            levels::photo_filter(levels::PhotoFilter::Preset(levels::FilterType::Sepia))
              .with_density(0.5)
              .apply_on_cpu(e)
          },
        ),
      ),
      (
        "photo_filter color",
        handle_and_library(
          |i| {
            photo_filter(PhotoFilter::Color(Arc::new(Color::from_rgb(10, 120, 200))))
              .with_preserve_luminosity(true)
              .apply_on_cpu(i)
          },
          |e| {
            levels::photo_filter(levels::PhotoFilter::Color(abra::abra_core::Color::from_rgb(10, 120, 200)))
              .with_preserve_luminosity(true)
              .apply_on_cpu(e)
          },
        ),
      ),
      (
        "linear gradient angle",
        handle_and_library(
          |i| LinearGradientEffect::angle(gradient(), 35.0).with_opacity(0.6).apply_on_cpu(i),
          |e| color::LinearGradientEffect::angle(&gradient().0, 35.0).with_opacity(0.6).apply_on_cpu(e),
        ),
      ),
      (
        "linear gradient between",
        handle_and_library(
          |i| LinearGradientEffect::between(gradient(), vec![0.0, 0.0], vec![5.0, 3.0]).apply_on_cpu(i),
          |e| color::LinearGradientEffect::between(&gradient().0, (0.0, 0.0), (5.0, 3.0)).apply_on_cpu(e),
        ),
      ),
    ];
    for (name, (made, expected)) in cases {
      assert_eq!(made, expected, "{name} differs from the library");
    }
  }

  #[test]
  fn the_effects_change_the_picture() {
    // A guard for the test above: a handle that did nothing would also equal a library effect that did nothing.
    let image = Arc::new(Image::new_from_pixels(6, 4, picture(), Channels::RGBA));
    invert().apply_on_cpu(image.clone());
    assert_ne!(image.rgba(), picture(), "invert left the picture as it was");
    let image = Arc::new(Image::new_from_pixels(6, 4, picture(), Channels::RGBA));
    brightness(30).apply_on_cpu(image.clone());
    assert_ne!(image.rgba(), picture(), "brightness left the picture as it was");
  }

  #[test]
  fn a_linear_gradient_can_be_changed_after_it_is_made() {
    let gradient = Arc::new(Gradient::from_to(Arc::new(Color::black()), Arc::new(Color::white())));
    let effect = LinearGradientEffect::angle(gradient.clone(), 0.0);
    effect.set_angle(90.0);
    effect.set_opacity(0.5);
    effect.set_between(vec![0.0, 0.0], vec![5.0, 0.0]);
    effect.set_gradient(gradient.clone());
    let image = Arc::new(Image::new_from_pixels(6, 4, picture(), Channels::RGBA));
    effect.apply_on_cpu(image.clone());
    let mut expected = abra::abra_core::Image::new_from_pixels(6, 4, picture(), Channels::RGBA);
    let mut library = color::LinearGradientEffect::angle(&gradient.0, 0.0);
    library.set_angle(90.0);
    library.set_opacity(0.5);
    library.set_between((0.0, 0.0), (5.0, 0.0));
    library.set_gradient(&gradient.0);
    library.apply_on_cpu(&mut expected);
    assert_eq!(image.rgba(), expected.rgba());
  }

  #[test]
  #[should_panic(expected = "a point takes 2 values")]
  fn a_point_of_the_wrong_length_panics() {
    let gradient = Arc::new(Gradient::from_to(Arc::new(Color::black()), Arc::new(Color::white())));
    LinearGradientEffect::between(gradient, vec![1.0], vec![2.0, 3.0]);
  }
}
