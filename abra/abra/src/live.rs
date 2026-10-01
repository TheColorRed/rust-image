//! Effect descriptions shared by one-shot edits and bindings, and (with the `live` feature) interactive rendering.
//!
//! [`EffectSpec`] maps a parameter list onto effects, for any image or [`LiveImage`]. With the `live` feature,
//! [`LiveImage`] holds an image and re-renders it as effects change, on the GPU when one is available.

use core::f32::consts::E;

use abra_core::{Color, ColorStop, Gradient, Image, ImageRef};
use adjustments::color::{LinearGradientEffect, grayscale, invert};
use options::Effect;

#[cfg(feature = "live")]
pub use abra_live::{EffectId, LiveFrame, LiveImage, LiveSlot};

/// One color stop of a gradient, at `position` from 0 to 1.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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

/// An effect and its parameters.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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
  /// Grayscale effect.
  Grayscale,
  /// Invert effect.
  Invert,
}

/// Any effect an [`EffectSpec`] can build. Every effect qualifies, so this only names the bounds once.
pub trait SpecEffect: Effect + Clone + 'static {}
impl<E: Effect + Clone + 'static> SpecEffect for E {}

/// Something an [`EffectSpec`] can be applied to: an image or a [`LiveImage`]. It takes whichever effect the spec
/// builds, so a new effect only needs an [`EffectSpec`] variant, not a bound here.
pub trait EffectSink {
  /// Applies `p_effect` to this target, as `p_effect.apply(target)` would.
  fn accept<E: SpecEffect>(self, p_effect: E);
}

impl EffectSink for &mut Image {
  fn accept<E: SpecEffect>(self, p_effect: E) {
    p_effect.apply(self);
  }
}

impl EffectSink for ImageRef<'_> {
  fn accept<E: SpecEffect>(self, p_effect: E) {
    p_effect.apply(self);
  }
}

#[cfg(feature = "live")]
impl EffectSink for &mut LiveImage {
  fn accept<E: SpecEffect>(self, p_effect: E) {
    p_effect.apply(self);
  }
}

#[cfg(feature = "live")]
impl EffectSink for LiveSlot<'_> {
  fn accept<E: SpecEffect>(self, p_effect: E) {
    p_effect.apply(self);
  }
}

impl EffectSpec {
  /// Applies this effect to `p_target`, an image or a [`LiveImage`], exactly as the effect's own `apply` would.
  /// This is the one place a binding maps its parameter list onto effects.
  pub fn apply(&self, p_target: impl EffectSink) {
    match self {
      // Effects without parameters.
      EffectSpec::Grayscale => p_target.accept(grayscale()),
      EffectSpec::Invert => p_target.accept(invert()),
      // Effects with parameters.
      EffectSpec::Threshold { amount } => p_target.accept(adjustments::color::threshold(*amount)),
      EffectSpec::Brightness { amount } => p_target.accept(adjustments::levels::brightness(*amount)),
      EffectSpec::Contrast { amount } => p_target.accept(adjustments::levels::contrast(*amount)),
      EffectSpec::Saturation { amount } => p_target.accept(adjustments::levels::saturation(*amount)),
      EffectSpec::GaussianBlur { radius } => p_target.accept(filters::blur::gaussian_blur(*radius)),
      EffectSpec::Vibrance { vibrance, saturation } => {
        p_target.accept(adjustments::levels::vibrance(*vibrance).with_saturation(*saturation))
      }
      EffectSpec::Exposure {
        exposure,
        offset,
        gamma_correction,
      } => p_target.accept(adjustments::levels::exposure(*exposure).with_offset(*offset).with_gamma(*gamma_correction)),
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
        p_target.accept(LinearGradientEffect::angle(&gradient, *angle).with_opacity(*opacity))
      }
    }
  }
}

#[cfg(all(test, feature = "live"))]
mod tests {
  use super::*;
  use abra_core::{Channels, Image};

  fn preview() -> LiveImage {
    let pixels: Vec<u8> = (0..16 * 16).flat_map(|i| [(i % 251) as u8, 90, 40, 255]).collect();
    LiveImage::new(16, 16, pixels).unwrap()
  }

  fn wait_for_frame(p_preview: &mut LiveImage) -> LiveFrame {
    wait_for_frame_where(p_preview, |_| true)
  }

  /// Waits for a frame that satisfies `p_accept`. An earlier chain's frame can still be in flight on the GPU and
  /// arrive first, so a test that expects a particular result has to skip it.
  fn wait_for_frame_where(p_preview: &mut LiveImage, p_accept: impl Fn(&LiveFrame) -> bool) -> LiveFrame {
    for _ in 0..400 {
      if let Some(frame) = p_preview.poll().unwrap()
        && p_accept(&frame)
      {
        return frame;
      }
      std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("no matching frame arrived");
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
  fn rejects_mismatched_pixels() {
    assert!(LiveImage::new(4, 4, vec![0; 10]).is_err());
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
  fn setting_an_effect_by_id_replaces_it_and_matches_a_one_shot() {
    let mut preview = preview();
    let id = preview.new_id();
    adjustments::levels::brightness(60).apply(preview.slot(id));
    adjustments::levels::brightness(20).apply(preview.slot(id));
    assert_eq!(preview.ids(), vec![id]);

    let pixels: Vec<u8> = (0..16 * 16).flat_map(|i| [(i % 251) as u8, 90, 40, 255]).collect();
    let mut image = Image::new_from_pixels(16, 16, pixels, Channels::RGBA);
    adjustments::levels::brightness(20).apply(&mut image);
    let expected = image.into_rgba_vec();
    let close = |frame: &LiveFrame| frame.pixels.iter().zip(&expected).all(|(a, b)| a.abs_diff(*b) <= 2);
    // The replaced brightness(60) may still deliver a frame first; the last state is what must arrive.
    wait_for_frame_where(&mut preview, close);
  }

  #[test]
  fn clear_returns_the_original() {
    let mut preview = preview();
    adjustments::levels::brightness(60).apply(&mut preview);
    preview.clear();
    let frame = wait_for_frame_where(&mut preview, |frame| frame.pixels[..4] == [0, 90, 40, 255]);
    assert_eq!(frame.pixels[..4], [0, 90, 40, 255]);
  }

  #[test]
  fn exposure_in_a_preview_matches_a_one_shot() {
    let mut preview = preview();
    let effect = || adjustments::levels::exposure(1.5).with_offset(0.05).with_gamma(1.3);
    effect().apply(&mut preview);
    let frame = wait_for_frame(&mut preview);

    let pixels: Vec<u8> = (0..16 * 16).flat_map(|i| [(i % 251) as u8, 90, 40, 255]).collect();
    let mut image = Image::new_from_pixels(16, 16, pixels, Channels::RGBA);
    effect().apply(&mut image);
    for (a, b) in frame.pixels.iter().zip(image.into_rgba_vec()) {
      assert!(a.abs_diff(b) <= 3, "preview {a} vs one-shot {b}");
    }
  }

  /// The original test pixels with `p_apply` run on them as a one-shot edit.
  fn one_shot(p_apply: impl Fn(&mut Image)) -> Vec<u8> {
    let pixels: Vec<u8> = (0..16 * 16).flat_map(|i| [(i % 251) as u8, 90, 40, 255]).collect();
    let mut image = Image::new_from_pixels(16, 16, pixels, Channels::RGBA);
    p_apply(&mut image);
    image.into_rgba_vec()
  }

  fn wait_for_pixels(p_preview: &mut LiveImage, p_expected: &[u8]) {
    wait_for_frame_where(p_preview, |frame| frame.pixels.iter().zip(p_expected).all(|(a, b)| a == b));
  }

  #[test]
  fn applying_the_same_kind_twice_adds_two_effects() {
    let mut preview = preview();
    adjustments::levels::brightness(30).apply(&mut preview);
    adjustments::levels::brightness(30).apply(&mut preview);
    assert_eq!(preview.ids().len(), 2);
    let expected = one_shot(|image| {
      adjustments::levels::brightness(30).apply(&mut *image);
      adjustments::levels::brightness(30).apply(&mut *image);
    });
    wait_for_pixels(&mut preview, &expected);
  }

  #[test]
  fn removing_an_effect_takes_it_out_of_the_chain() {
    let mut preview = preview();
    adjustments::levels::brightness(60).apply(&mut preview);
    let keep = preview.new_id();
    adjustments::levels::exposure(1.0).apply(preview.slot(keep));
    let first = preview.ids()[0];
    assert!(preview.remove(first));
    assert!(!preview.remove(first));
    assert_eq!(preview.ids(), vec![keep]);
    let expected = one_shot(|image| adjustments::levels::exposure(1.0).apply(&mut *image));
    wait_for_pixels(&mut preview, &expected);
  }

  #[test]
  fn moving_an_effect_changes_the_order_effects_run_in() {
    let mut preview = preview();
    let brightness = preview.new_id();
    adjustments::levels::brightness(40).apply(preview.slot(brightness));
    let exposure = preview.new_id();
    adjustments::levels::exposure(0.0).with_offset(0.1).apply(preview.slot(exposure));
    assert_eq!(preview.ids(), vec![brightness, exposure]);

    assert!(preview.move_to(exposure, 0));
    assert_eq!(preview.ids(), vec![exposure, brightness]);
    let expected = one_shot(|image| {
      adjustments::levels::exposure(0.0).with_offset(0.1).apply(&mut *image);
      adjustments::levels::brightness(40).apply(&mut *image);
    });
    let original_order = one_shot(|image| {
      adjustments::levels::brightness(40).apply(&mut *image);
      adjustments::levels::exposure(0.0).with_offset(0.1).apply(&mut *image);
    });
    assert!(
      expected.iter().zip(&original_order).any(|(a, b)| a.abs_diff(*b) > 6),
      "the two orders must give different pixels for this test to mean anything"
    );
    wait_for_pixels(&mut preview, &expected);
    assert!(!preview.move_to(EffectId(999), 0));
  }

  #[test]
  fn an_effect_keeps_the_area_and_mask_it_was_given() {
    use options::ApplyOptions;
    let mut preview = preview();
    adjustments::levels::brightness(20).apply(&mut preview);
    let id = preview.new_id();
    let options = ApplyOptions::new().with_area(abra_core::Area::rect((2.0, 2.0), (8.0, 8.0)));
    adjustments::levels::brightness(20).with_options(options).apply(preview.slot(id));

    let plain = preview.options(preview.ids()[0]).unwrap();
    assert!(plain.is_none());
    let kept = preview.options(id).unwrap().as_ref().expect("options are kept");
    assert_eq!(kept.area().map(|areas| areas.len()), Some(1));
    assert!(preview.options(EffectId(999)).is_none());

    // Replacing the effect with one that has no options clears them.
    adjustments::levels::brightness(20).apply(preview.slot(id));
    assert!(preview.options(id).unwrap().is_none());
  }

  fn original_pixels() -> Vec<u8> {
    (0..16 * 16).flat_map(|i| [(i % 251) as u8, 90, 40, 255]).collect()
  }

  #[test]
  fn an_area_limits_the_effect_like_it_does_on_an_image() {
    use options::ApplyOptions;
    let options = || ApplyOptions::new().with_area(abra_core::Area::rect((3.0, 3.0), (8.0, 8.0)).with_feather(3));
    let mut preview = preview();
    adjustments::levels::brightness(60).with_options(options()).apply(&mut preview);
    let expected = one_shot(|image| adjustments::levels::brightness(60).with_options(options()).apply(&mut *image));

    let original = original_pixels();
    assert_ne!(expected, original, "the effect must change something inside the area");
    assert_eq!(expected[..4], original[..4], "and leave the corner outside the area alone");
    let frame = wait_for_frame_where(&mut preview, |frame| frame.pixels.iter().zip(&expected).all(|(a, b)| a == b));
    assert_eq!(frame.pixels[..4], original[..4]);
  }

  #[test]
  fn a_mask_limits_the_effect_like_it_does_on_an_image() {
    use options::ApplyOptions;
    // Left half white (full effect), right half black (no effect).
    let mask_pixels: Vec<u8> =
      (0..16 * 16).flat_map(|i| if i % 16 < 8 { [255, 255, 255, 255] } else { [0, 0, 0, 255] }).collect();
    let mask = || mask::Mask::from_image(Image::new_from_pixels(16, 16, mask_pixels.clone(), Channels::RGBA));
    let mut preview = preview();
    adjustments::levels::brightness(60).with_options(ApplyOptions::new().with_mask(mask())).apply(&mut preview);
    let expected = one_shot(|image| {
      adjustments::levels::brightness(60).with_options(ApplyOptions::new().with_mask(mask())).apply(&mut *image)
    });

    let original = original_pixels();
    let pixel = |pixels: &[u8], x: usize, y: usize| pixels[(y * 16 + x) * 4..(y * 16 + x) * 4 + 3].to_vec();
    assert_ne!(pixel(&expected, 2, 5), pixel(&original, 2, 5));
    assert_eq!(pixel(&expected, 12, 5), pixel(&original, 12, 5));
    wait_for_pixels(&mut preview, &expected);
  }

  #[test]
  fn an_area_on_exposure_matches_a_one_shot() {
    use options::ApplyOptions;
    let options = || ApplyOptions::new().with_area(abra_core::Area::rect((0.0, 0.0), (8.0, 16.0)));
    let mut preview = preview();
    adjustments::levels::exposure(1.5).with_options(options()).apply(&mut preview);
    let expected = one_shot(|image| adjustments::levels::exposure(1.5).with_options(options()).apply(&mut *image));
    wait_for_pixels(&mut preview, &expected);
  }

  #[test]
  fn an_effect_that_only_runs_on_the_cpu_works_in_a_live_image() {
    let mut preview = preview();
    adjustments::levels::brightness(30).apply(&mut preview);
    adjustments::color::invert().apply(&mut preview);
    let expected = one_shot(|image| {
      adjustments::levels::brightness(30).apply(&mut *image);
      adjustments::color::invert().apply(&mut *image);
    });
    wait_for_pixels(&mut preview, &expected);
  }

  #[test]
  fn an_area_limits_an_effect_that_only_runs_on_the_cpu() {
    use options::ApplyOptions;
    let options = || ApplyOptions::new().with_area(abra_core::Area::rect((0.0, 0.0), (8.0, 16.0)));
    let mut preview = preview();
    adjustments::color::grayscale().with_options(options()).apply(&mut preview);
    let expected = one_shot(|image| adjustments::color::grayscale().with_options(options()).apply(&mut *image));
    let original = original_pixels();
    assert_ne!(expected[..3], original[..3], "left half turns gray");
    assert_eq!(expected[12 * 4..12 * 4 + 3], original[12 * 4..12 * 4 + 3], "right half is untouched");
    wait_for_pixels(&mut preview, &expected);
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
  fn threshold_on_the_gpu_matches_the_cpu() {
    let mut preview = preview();
    adjustments::color::threshold(100).apply(&mut preview);
    let expected = one_shot(|image| adjustments::color::threshold(100).apply(&mut *image));
    let white = expected.chunks_exact(4).filter(|p| p[0] == 255).count();
    assert!(white > 0 && white < expected.len() / 4, "the test image must have pixels on both sides");
    wait_for_pixels(&mut preview, &expected);
  }

  #[test]
  fn invert_on_the_gpu_matches_the_cpu() {
    let mut preview = preview();
    adjustments::color::invert().apply(&mut preview);
    let expected = one_shot(|image| adjustments::color::invert().apply(&mut *image));
    assert_ne!(expected, original_pixels());
    // Alpha is left alone; only color is flipped.
    assert!(
      expected.chunks_exact(4).zip(original_pixels().chunks_exact(4)).all(|(a, b)| a[3] == b[3] && a[0] == 255 - b[0])
    );
    wait_for_pixels(&mut preview, &expected);
  }

  #[test]
  fn vibrance_on_the_gpu_matches_the_cpu_within_rounding() {
    let mut preview = preview();
    adjustments::levels::vibrance(60).apply(&mut preview);
    let expected = one_shot(|image| adjustments::levels::vibrance(60).apply(&mut *image));
    assert_ne!(expected, original_pixels());
    // Float math on the two sides can round a channel differently by one.
    wait_for_frame_where(&mut preview, |frame| {
      frame.pixels.iter().zip(&expected).all(|(a, b)| a.abs_diff(*b) <= 1)
    });
  }

  #[test]
  fn vibrance_with_saturation_on_the_gpu_matches_the_cpu_within_rounding() {
    let mut preview = preview();
    adjustments::levels::vibrance(40).with_saturation(25).apply(&mut preview);
    let expected = one_shot(|image| adjustments::levels::vibrance(40).with_saturation(25).apply(&mut *image));
    let vibrance_only = one_shot(|image| adjustments::levels::vibrance(40).apply(&mut *image));
    assert_ne!(expected, vibrance_only, "the saturation step must change something");
    wait_for_frame_where(&mut preview, |frame| {
      frame.pixels.iter().zip(&expected).all(|(a, b)| a.abs_diff(*b) <= 1)
    });
  }
}
