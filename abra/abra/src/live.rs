//! Effect descriptions shared by one-shot edits and bindings, and (with the `live` feature) interactive rendering.
//!
//! [`EffectSpec`] maps a parameter list onto effects, for any image or [`LiveImage`]. With the `live` feature,
//! [`LiveImage`] holds an image and re-renders it as effects change, on the GPU when one is available.

use abra_core::{Color, ColorStop, Gradient};
use adjustments::color::LinearGradientEffect;
use options::{Apply, ApplyTarget};

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
}

impl EffectSpec {
  /// Applies this effect to `p_target`, an image or a [`LiveImage`], exactly as the effect's own `apply` would.
  /// This is the one place a binding maps its parameter list onto effects.
  pub fn apply<T>(&self, p_target: T)
  where
    T: ApplyTarget<adjustments::levels::Brightness>
      + ApplyTarget<adjustments::levels::Contrast>
      + ApplyTarget<adjustments::levels::Saturation>
      + ApplyTarget<filters::blur::GaussianBlur>
      + ApplyTarget<adjustments::levels::Exposure>
      + ApplyTarget<LinearGradientEffect>,
  {
    match self {
      EffectSpec::Brightness { amount } => adjustments::levels::brightness(*amount).apply(p_target),
      EffectSpec::Contrast { amount } => adjustments::levels::contrast(*amount).apply(p_target),
      EffectSpec::Saturation { amount } => adjustments::levels::saturation(*amount).apply(p_target),
      EffectSpec::GaussianBlur { radius } => filters::blur::gaussian_blur(*radius).apply(p_target),
      EffectSpec::Exposure { exposure, offset, gamma_correction } => {
        adjustments::levels::exposure(*exposure).with_offset(*offset).with_gamma(*gamma_correction).apply(p_target)
      }
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
        LinearGradientEffect::angle(&gradient, *angle).with_opacity(*opacity).apply(p_target)
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
    wait_for_frame_where(p_preview, |frame| frame.pixels.iter().zip(p_expected).all(|(a, b)| a.abs_diff(*b) <= 3));
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
    let frame = wait_for_frame_where(&mut preview, |frame| frame.pixels.iter().zip(&expected).all(|(a, b)| a.abs_diff(*b) <= 3));
    assert_eq!(frame.pixels[..4], original[..4]);
  }

  #[test]
  fn a_mask_limits_the_effect_like_it_does_on_an_image() {
    use options::ApplyOptions;
    // Left half white (full effect), right half black (no effect).
    let mask_pixels: Vec<u8> = (0..16 * 16).flat_map(|i| if i % 16 < 8 { [255, 255, 255, 255] } else { [0, 0, 0, 255] }).collect();
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
}
