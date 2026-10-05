//! The effects the mobile editor can apply, described as plain data.
//!
//! [`EffectSpec`] is the one place the editor's parameter lists are mapped onto library effects, for a one-shot edit
//! of an image and for a live preview alike. A new effect needs a variant here and nothing else on the Rust side.

use abra::abra_core::{Color, ColorStop, Gradient};
use abra::adjustments::prelude::{color, levels};
use abra::live::{EffectSink, GpuProbe};
use abra_body_segmentation::Mask;

use levels::FilterType;

use crate::AbraImage;

/// One color stop of a gradient, at `position` from 0 to 1.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct GradientStop {
  /// Where the stop sits along the gradient, from 0 to 1.
  pub position: f32,
  /// Red, 0-255.
  pub r: u8,
  /// Green, 0-255.
  pub g: u8,
  /// Blue, 0-255.
  pub b: u8,
  /// Alpha, 0-255.
  pub a: u8,
}

// `FilterType` lives in `adjustments`, which knows nothing about bindings. This describes it to uniffi, so the editor
// names the library's own presets and the colors exist only in `adjustments`. Every variant must be listed.
#[uniffi::remote(Enum)]
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

// `Color` lives in `abra_core`, which knows nothing about bindings. This describes it to uniffi so a custom tint can be
// passed as `{ r, g, b, a }`. The fields must match `abra_core::Color` exactly.
#[uniffi::remote(Record)]
pub struct Color {
  pub r: u8,
  pub g: u8,
  pub b: u8,
  pub a: u8,
}

/// An effect and its parameters.
#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum EffectSpec {
  /// Threshold effect, 0 to 255.
  Threshold {
    /// The threshold value.
    amount: u8,
  },
  /// Brightness, -100 to 100 (0 is unchanged).
  Brightness {
    /// The amount of change.
    amount: i32,
  },
  /// Contrast, -100 to 100 (0 is unchanged).
  Contrast {
    /// The amount of change.
    amount: f64,
  },
  /// Saturation, -100 to 100 (0 is unchanged, -100 is grayscale).
  Saturation {
    /// The amount of change.
    amount: i32,
  },
  /// Vibrance adjustment, -100 to 100 (0 is unchanged).
  Vibrance {
    /// The vibrance adjustment, -100 to 100 (0 is unchanged).
    vibrance: f64,
    /// The saturation adjustment, -100 to 100 (0 is unchanged).
    saturation: f64,
  },
  /// Gaussian blur.
  GaussianBlur {
    /// Radius in pixels.
    radius: f64,
  },
  /// Exposure adjustment.
  Exposure {
    /// The exposure in stops.
    exposure: f64,
    /// The offset added to each color channel.
    offset: f64,
    /// The gamma correction.
    gamma_correction: f64,
  },
  /// A linear gradient drawn over the image.
  LinearGradient {
    /// The colors, at least one stop.
    stops: Vec<GradientStop>,
    /// CSS-style angle in degrees: 0 points up, 90 runs left to right.
    angle: f32,
    /// Overall opacity, 0 to 1.
    opacity: f32,
  },
  /// Tints the image as if shot through a colored lens filter.
  PhotoFilter {
    /// The preset filter, which carries its own color.
    filter: Option<FilterType>,
    /// How strongly the tint is applied, 0 to 1.
    density: f64,
    /// Whether the photo's brightness is kept. Off tints like a colored lens filter, which also darkens and is what a
    /// preset normally does; only that mode has a shader.
    preserve_luminosity: bool,
    /// Color of the custom filter. Ignored if a preset is used.
    color: Option<Color>,
  },
  SkinSmooth {
    /// The amount of smoothing.
    amount: f64,
  },
  /// Tans skin to a place on the tan scale ([`skin_tan_gradient`]).
  SkinTan {
    /// Where on the scale: 0 is white, which changes nothing, and 1 is the darkest tan.
    offset: f64,
  },
  /// Grayscale effect.
  Grayscale,
  /// Invert effect.
  Invert,
}

/// The tan scale: the colors skin is tanned to, from none (white multiplies by 1) to the darkest. The tan color picker draws
/// this as its bar and [`EffectSpec::SkinTan`] reads its color from it, so what the user sees is what the photo gets.
pub fn skin_tan_gradient() -> Gradient {
  Gradient::evenly(vec![
    Color::white(),
    Color::tan(),
    Color::light_brown(),
    Color::brown(),
    Color::dark_brown(),
    Color::black(),
  ])
}

impl EffectSpec {
  /// Whether this effect has a shader. An effect without one still works in a live image, but it runs on the CPU, so a
  /// chain that contains it recomputes it on every change instead of only changing uniform values.
  pub fn has_gpu(&self) -> bool {
    let mut has_gpu = false;
    self.apply(GpuProbe(&mut has_gpu));
    has_gpu
  }

  /// Applies this effect to `p_target`, an image or a live image, exactly as the effect's own `apply` would.
  /// This is the one place the editor maps its parameter lists onto effects.
  pub fn apply(&self, p_target: impl EffectSink) {
    self.apply_with_skin_mask(p_target, None);
  }

  /// Applies this effect, using a precomputed image skin mask when the effect supports it.
  pub(crate) fn apply_with_skin_mask(&self, p_target: impl EffectSink, p_skin_mask: Option<&Mask>) {
    match self {
      // Effects without parameters.
      EffectSpec::Grayscale => p_target.accept(color::grayscale()),
      EffectSpec::Invert => p_target.accept(color::invert()),
      // Effects with parameters.
      EffectSpec::SkinSmooth { amount } => {
        let effect = abra::filters::prelude::skin::smooth_skin(*amount);
        if let Some(mask) = p_skin_mask {
          p_target.accept(effect.with_mask(mask.clone()));
        } else {
          p_target.accept(effect);
        }
      }
      EffectSpec::SkinTan { offset } => {
        let effect = abra::filters::prelude::skin::tan_skin(skin_tan_gradient().color_at(*offset as f32));
        if let Some(mask) = p_skin_mask {
          p_target.accept(effect.with_mask(mask.clone()));
        } else {
          p_target.accept(effect);
        }
      }
      EffectSpec::Threshold { amount } => p_target.accept(color::threshold(*amount)),
      EffectSpec::Brightness { amount } => p_target.accept(levels::brightness(*amount)),
      EffectSpec::Contrast { amount } => p_target.accept(levels::contrast(*amount)),
      EffectSpec::Saturation { amount } => p_target.accept(levels::saturation(*amount)),
      EffectSpec::GaussianBlur { radius } => p_target.accept(abra::filters::prelude::blur::gaussian_blur(*radius)),
      EffectSpec::PhotoFilter {
        filter,
        density,
        preserve_luminosity,
        color,
      } => {
        let filter_to_use = if let Some(f) = filter {
          levels::PhotoFilter::Preset(*f)
        } else if let Some(f) = color {
          levels::PhotoFilter::Color(*f)
        } else {
          levels::PhotoFilter::Color(Color::transparent())
        };

        p_target.accept(
          levels::photo_filter(filter_to_use).with_density(*density).with_preserve_luminosity(*preserve_luminosity),
        )
      }
      EffectSpec::Vibrance { vibrance, saturation } => {
        p_target.accept(levels::vibrance(*vibrance).with_saturation(*saturation))
      }
      EffectSpec::Exposure {
        exposure,
        offset,
        gamma_correction,
      } => p_target.accept(levels::exposure(*exposure).with_offset(*offset).with_gamma(*gamma_correction)),
      EffectSpec::LinearGradient { stops, angle, opacity } => {
        let stops = stops
          .iter()
          .map(|stop| ColorStop::new(Color::from_rgba(stop.r, stop.g, stop.b, stop.a), stop.position))
          .collect::<Vec<_>>();
        let gradient = if stops.is_empty() {
          Gradient::from_to(Color::transparent(), Color::transparent())
        } else {
          Gradient::new(stops)
        };
        p_target.accept(color::LinearGradientEffect::angle(&gradient, *angle).with_opacity(*opacity))
      }
    }
  }
}

/// Whether the effect has a shader. One without runs on the CPU, so a live chain recomputes it on every change.
#[uniffi::export]
pub fn effect_has_gpu(effect: EffectSpec) -> bool {
  effect.has_gpu()
}

#[uniffi::export]
impl AbraImage {
  /// Applies one effect to the image. Uses the same definition as live previews.
  pub fn apply_effect(&self, effect: EffectSpec) {
    let skin_mask = self.skin_mask();
    self.with_image_mut(|img| effect.apply_with_skin_mask(img, skin_mask.as_ref()));
  }

  /// Applies an effect with an explicit person mask without changing the live selection.
  pub fn apply_effect_to_person(&self, effect: EffectSpec, person_id: Option<u32>) -> Result<(), crate::AbraError> {
    let skin_mask = self.skin_mask_for_person(person_id)?;
    self.with_image_mut(|img| effect.apply_with_skin_mask(img, skin_mask.as_ref()));
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::live_image::{LiveFrame, LiveImage};
  use abra::abra_core::{Channels, Image};

  fn original_pixels() -> Vec<u8> {
    (0..16 * 16).flat_map(|i| [(i % 251) as u8, 90, 40, 255]).collect()
  }

  fn preview() -> LiveImage {
    LiveImage::new(16, 16, original_pixels()).unwrap()
  }

  /// The original test pixels with `p_apply` run on them as a one-shot edit.
  fn one_shot(p_apply: impl Fn(&mut Image)) -> Vec<u8> {
    let mut image = Image::new_from_pixels(16, 16, original_pixels(), Channels::RGBA);
    p_apply(&mut image);
    image.into_rgba_vec()
  }

  /// The frame of the chain as it is now, which must satisfy `p_accept`.
  fn wait_for_frame_where(p_preview: &mut LiveImage, p_accept: impl Fn(&LiveFrame) -> bool) -> LiveFrame {
    let frame = p_preview.pixels().unwrap();
    assert!(p_accept(&frame), "the frame is not the one expected");
    frame
  }

  fn wait_for_frame(p_preview: &mut LiveImage) -> LiveFrame {
    wait_for_frame_where(p_preview, |_| true)
  }

  fn wait_for_pixels(p_preview: &mut LiveImage, p_expected: &[u8]) {
    wait_for_frame_where(p_preview, |frame| frame.pixels.iter().zip(p_expected).all(|(a, b)| a == b));
  }

  /// The Warming preset, tinting like a lens filter.
  fn warming_tint(p_density: f64) -> EffectSpec {
    EffectSpec::PhotoFilter {
      filter: Some(FilterType::WarmingLight),
      density: p_density,
      preserve_luminosity: false,
      color: None,
    }
  }

  fn gradient(p_angle: f32) -> EffectSpec {
    EffectSpec::LinearGradient {
      stops: vec![
        GradientStop {
          position: 0.0,
          r: 255,
          g: 0,
          b: 0,
          a: 255,
        },
        GradientStop {
          position: 1.0,
          r: 0,
          g: 0,
          b: 255,
          a: 255,
        },
      ],
      angle: p_angle,
      opacity: 1.0,
    }
  }

  #[test]
  fn a_parameter_change_produces_a_new_frame() {
    let mut preview = preview();
    EffectSpec::Brightness { amount: 20 }.apply(&mut preview);
    let id = preview.new_id();
    gradient(0.0).apply(preview.slot(id));
    let up = wait_for_frame(&mut preview);
    gradient(90.0).apply(preview.slot(id));
    let right = wait_for_frame(&mut preview);
    assert_eq!((right.width, right.height), (16, 16));
    assert_ne!(up.pixels, right.pixels);
  }

  #[test]
  fn grayscale_spec_turns_an_image_gray_and_matches_a_live_image() {
    let gray = one_shot(|image| EffectSpec::Grayscale.apply(&mut *image));
    assert!(gray.chunks_exact(4).all(|p| p[0] == p[1] && p[1] == p[2]));
    assert_ne!(gray, original_pixels());
    let mut preview = preview();
    EffectSpec::Grayscale.apply(&mut preview);
    wait_for_pixels(&mut preview, &gray);
  }

  #[test]
  fn has_gpu_tells_shader_effects_from_cpu_only_ones() {
    assert!(EffectSpec::Brightness { amount: 10 }.has_gpu());
    assert!(EffectSpec::Invert.has_gpu());
    // Skin smoothing is a chain of shader passes, so a live chain holding it stays on the GPU.
    assert!(EffectSpec::SkinSmooth { amount: 1.0 }.has_gpu());
    // Only the lens-filter mode of the photo filter has a shader.
    assert!(warming_tint(0.3).has_gpu());
    let preserving = EffectSpec::PhotoFilter {
      filter: Some(FilterType::WarmingLight),
      density: 0.3,
      preserve_luminosity: true,
      color: None,
    };
    assert!(!preserving.has_gpu());
  }

  #[test]
  fn photo_filter_spec_matches_the_preset_effect_and_a_live_image() {
    use abra::adjustments::prelude::Effect;
    use abra::adjustments::prelude::levels::{PhotoFilter, photo_filter};
    let spec = warming_tint(0.35);
    let expected = one_shot(|image| {
      photo_filter(PhotoFilter::Preset(FilterType::WarmingLight)).with_density(0.35).apply(&mut *image)
    });
    assert_ne!(expected, original_pixels(), "the tint must change something");
    assert_eq!(one_shot(|image| spec.apply(&mut *image)), expected);
    // A live image runs it on the GPU, within a level of the CPU.
    let mut preview = preview();
    spec.apply(&mut preview);
    wait_for_frame_where(&mut preview, |frame| frame.pixels.iter().zip(&expected).all(|(a, b)| a.abs_diff(*b) <= 1));
  }

  #[test]
  fn skin_tan_uses_the_cached_mask_for_images_and_live_previews() {
    let source = vec![60u8, 100, 200, 255].repeat(16 * 16);
    let mask = Mask::from_image(Image::new_from_color(16, 16, Color::white()));
    let effect = EffectSpec::SkinTan { offset: 1.0 };

    let mut unmasked = Image::new_from_pixels(16, 16, source.clone(), Channels::RGBA);
    effect.apply(&mut unmasked);
    assert_eq!(unmasked.rgba(), source, "blue should be excluded by color detection");

    let mut image = Image::new_from_pixels(16, 16, source.clone(), Channels::RGBA);
    effect.apply_with_skin_mask(&mut image, Some(&mask));
    assert_ne!(image.rgba(), source, "the supplied mask should allow the effect on blue pixels");

    let mut preview = LiveImage::new(16, 16, source).unwrap();
    effect.apply_with_skin_mask(&mut preview, Some(&mask));
    wait_for_pixels(&mut preview, image.rgba());
  }
}
