//! Tests for `LiveImage`: the same effects applied to a live image and to an image must give the same pixels.

use super::*;
use abra::abra_core::{Channels, Image};

fn preview() -> LiveImage {
  let pixels: Vec<u8> = (0..16 * 16).flat_map(|i| [(i % 251) as u8, 90, 40, 255]).collect();
  LiveImage::new(16, 16, pixels).unwrap()
}

fn wait_for_frame(p_preview: &mut LiveImage) -> LiveFrame {
  p_preview.pixels().unwrap()
}

/// The frame of the chain as it is now, which must satisfy `p_accept`.
fn wait_for_frame_where(p_preview: &mut LiveImage, p_accept: impl Fn(&LiveFrame) -> bool) -> LiveFrame {
  let frame = p_preview.pixels().unwrap();
  assert!(p_accept(&frame), "the frame is not the one expected");
  frame
}

#[test]
fn rejects_mismatched_pixels() {
  assert!(LiveImage::new(4, 4, vec![0; 10]).is_err());
}

#[test]
fn setting_an_effect_by_id_replaces_it_and_matches_a_one_shot() {
  let mut preview = preview();
  let id = preview.new_id();
  abra::adjustments::prelude::levels::brightness(60).apply(preview.slot(id));
  abra::adjustments::prelude::levels::brightness(20).apply(preview.slot(id));
  assert_eq!(preview.ids(), vec![id]);

  let pixels: Vec<u8> = (0..16 * 16).flat_map(|i| [(i % 251) as u8, 90, 40, 255]).collect();
  let mut image = Image::new_from_pixels(16, 16, pixels, Channels::RGBA);
  abra::adjustments::prelude::levels::brightness(20).apply(&mut image);
  let expected = image.into_rgba_vec();
  let close = |frame: &LiveFrame| frame.pixels.iter().zip(&expected).all(|(a, b)| a.abs_diff(*b) <= 2);
  // The replaced brightness(60) may still deliver a frame first; the last state is what must arrive.
  wait_for_frame_where(&mut preview, close);
}

#[test]
fn clear_returns_the_original() {
  let mut preview = preview();
  abra::adjustments::prelude::levels::brightness(60).apply(&mut preview);
  preview.clear();
  let frame = wait_for_frame_where(&mut preview, |frame| frame.pixels[..4] == [0, 90, 40, 255]);
  assert_eq!(frame.pixels[..4], [0, 90, 40, 255]);
}

#[test]
fn exposure_in_a_preview_matches_a_one_shot() {
  let mut preview = preview();
  let effect = || abra::adjustments::prelude::levels::exposure(1.5).with_offset(0.05).with_gamma(1.3);
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
  abra::adjustments::prelude::levels::brightness(30).apply(&mut preview);
  abra::adjustments::prelude::levels::brightness(30).apply(&mut preview);
  assert_eq!(preview.ids().len(), 2);
  let expected = one_shot(|image| {
    abra::adjustments::prelude::levels::brightness(30).apply(&mut *image);
    abra::adjustments::prelude::levels::brightness(30).apply(&mut *image);
  });
  wait_for_pixels(&mut preview, &expected);
}

#[test]
fn removing_an_effect_takes_it_out_of_the_chain() {
  let mut preview = preview();
  abra::adjustments::prelude::levels::brightness(60).apply(&mut preview);
  let keep = preview.new_id();
  abra::adjustments::prelude::levels::exposure(1.0).apply(preview.slot(keep));
  let first = preview.ids()[0];
  assert!(preview.remove(first));
  assert!(!preview.remove(first));
  assert_eq!(preview.ids(), vec![keep]);
  let expected = one_shot(|image| abra::adjustments::prelude::levels::exposure(1.0).apply(&mut *image));
  wait_for_pixels(&mut preview, &expected);
}

#[test]
fn moving_an_effect_changes_the_order_effects_run_in() {
  let mut preview = preview();
  let brightness = preview.new_id();
  abra::adjustments::prelude::levels::brightness(40).apply(preview.slot(brightness));
  let exposure = preview.new_id();
  abra::adjustments::prelude::levels::exposure(0.0).with_offset(0.1).apply(preview.slot(exposure));
  assert_eq!(preview.ids(), vec![brightness, exposure]);

  assert!(preview.move_to(exposure, 0));
  assert_eq!(preview.ids(), vec![exposure, brightness]);
  let expected = one_shot(|image| {
    abra::adjustments::prelude::levels::exposure(0.0).with_offset(0.1).apply(&mut *image);
    abra::adjustments::prelude::levels::brightness(40).apply(&mut *image);
  });
  let original_order = one_shot(|image| {
    abra::adjustments::prelude::levels::brightness(40).apply(&mut *image);
    abra::adjustments::prelude::levels::exposure(0.0).with_offset(0.1).apply(&mut *image);
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
  use abra::options::prelude::ApplyOptions;
  let mut preview = preview();
  abra::adjustments::prelude::levels::brightness(20).apply(&mut preview);
  let id = preview.new_id();
  let options = ApplyOptions::new().with_area(abra::abra_core::Area::rect((2.0, 2.0), (8.0, 8.0)));
  abra::adjustments::prelude::levels::brightness(20).with_options(options).apply(preview.slot(id));

  let plain = preview.options(preview.ids()[0]).unwrap();
  assert!(plain.is_none());
  let kept = preview.options(id).unwrap().as_ref().expect("options are kept");
  assert_eq!(kept.area().map(|areas| areas.len()), Some(1));
  assert!(preview.options(EffectId(999)).is_none());

  // Replacing the effect with one that has no options clears them.
  abra::adjustments::prelude::levels::brightness(20).apply(preview.slot(id));
  assert!(preview.options(id).unwrap().is_none());
}

fn original_pixels() -> Vec<u8> {
  (0..16 * 16).flat_map(|i| [(i % 251) as u8, 90, 40, 255]).collect()
}

#[test]
fn an_area_limits_the_effect_like_it_does_on_an_image() {
  use abra::options::prelude::ApplyOptions;
  let options = || ApplyOptions::new().with_area(abra::abra_core::Area::rect((3.0, 3.0), (8.0, 8.0)).with_feather(3));
  let mut preview = preview();
  abra::adjustments::prelude::levels::brightness(60).with_options(options()).apply(&mut preview);
  let expected = one_shot(|image| abra::adjustments::prelude::levels::brightness(60).with_options(options()).apply(&mut *image));

  let original = original_pixels();
  assert_ne!(expected, original, "the effect must change something inside the area");
  assert_eq!(expected[..4], original[..4], "and leave the corner outside the area alone");
  let frame = wait_for_frame_where(&mut preview, |frame| frame.pixels.iter().zip(&expected).all(|(a, b)| a == b));
  assert_eq!(frame.pixels[..4], original[..4]);
}

#[test]
fn a_mask_limits_the_effect_like_it_does_on_an_image() {
  use abra::options::prelude::ApplyOptions;
  // Left half white (full effect), right half black (no effect).
  let mask_pixels: Vec<u8> =
    (0..16 * 16).flat_map(|i| if i % 16 < 8 { [255, 255, 255, 255] } else { [0, 0, 0, 255] }).collect();
  let mask = || abra::mask::prelude::Mask::from_image(Image::new_from_pixels(16, 16, mask_pixels.clone(), Channels::RGBA));
  let mut preview = preview();
  abra::adjustments::prelude::levels::brightness(60).with_options(ApplyOptions::new().with_mask(mask())).apply(&mut preview);
  let expected = one_shot(|image| {
    abra::adjustments::prelude::levels::brightness(60).with_options(ApplyOptions::new().with_mask(mask())).apply(&mut *image)
  });

  let original = original_pixels();
  let pixel = |pixels: &[u8], x: usize, y: usize| pixels[(y * 16 + x) * 4..(y * 16 + x) * 4 + 3].to_vec();
  assert_ne!(pixel(&expected, 2, 5), pixel(&original, 2, 5));
  assert_eq!(pixel(&expected, 12, 5), pixel(&original, 12, 5));
  wait_for_pixels(&mut preview, &expected);
}

#[test]
fn an_area_on_exposure_matches_a_one_shot() {
  use abra::options::prelude::ApplyOptions;
  let options = || ApplyOptions::new().with_area(abra::abra_core::Area::rect((0.0, 0.0), (8.0, 16.0)));
  let mut preview = preview();
  abra::adjustments::prelude::levels::exposure(1.5).with_options(options()).apply(&mut preview);
  let expected = one_shot(|image| abra::adjustments::prelude::levels::exposure(1.5).with_options(options()).apply(&mut *image));
  wait_for_pixels(&mut preview, &expected);
}

#[test]
fn an_effect_that_only_runs_on_the_cpu_works_in_a_live_image() {
  let mut preview = preview();
  abra::adjustments::prelude::levels::brightness(30).apply(&mut preview);
  abra::adjustments::prelude::color::invert().apply(&mut preview);
  let expected = one_shot(|image| {
    abra::adjustments::prelude::levels::brightness(30).apply(&mut *image);
    abra::adjustments::prelude::color::invert().apply(&mut *image);
  });
  wait_for_pixels(&mut preview, &expected);
}

#[test]
fn an_area_limits_an_effect_that_only_runs_on_the_cpu() {
  use abra::options::prelude::ApplyOptions;
  let options = || ApplyOptions::new().with_area(abra::abra_core::Area::rect((0.0, 0.0), (8.0, 16.0)));
  let mut preview = preview();
  abra::adjustments::prelude::color::grayscale().with_options(options()).apply(&mut preview);
  let expected = one_shot(|image| abra::adjustments::prelude::color::grayscale().with_options(options()).apply(&mut *image));
  let original = original_pixels();
  assert_ne!(expected[..3], original[..3], "left half turns gray");
  assert_eq!(expected[12 * 4..12 * 4 + 3], original[12 * 4..12 * 4 + 3], "right half is untouched");
  wait_for_pixels(&mut preview, &expected);
}

#[test]
fn threshold_on_the_gpu_matches_the_cpu() {
  let mut preview = preview();
  abra::adjustments::prelude::color::threshold(100).apply(&mut preview);
  let expected = one_shot(|image| abra::adjustments::prelude::color::threshold(100).apply(&mut *image));
  let white = expected.chunks_exact(4).filter(|p| p[0] == 255).count();
  assert!(white > 0 && white < expected.len() / 4, "the test image must have pixels on both sides");
  wait_for_pixels(&mut preview, &expected);
}

#[test]
fn a_batch_replaces_the_chain_and_renders_the_result() {
  let mut preview = preview();
  abra::adjustments::prelude::levels::brightness(60).apply(&mut preview);
  preview.batch(|live| {
    live.clear();
    abra::adjustments::prelude::levels::brightness(20).apply(&mut *live);
    abra::adjustments::prelude::color::invert().apply(&mut *live);
  });
  assert_eq!(preview.ids().len(), 2);
  let expected = one_shot(|image| {
    abra::adjustments::prelude::levels::brightness(20).apply(&mut *image);
    abra::adjustments::prelude::color::invert().apply(&mut *image);
  });
  wait_for_pixels(&mut preview, &expected);
}

#[test]
fn invert_on_the_gpu_matches_the_cpu() {
  let mut preview = preview();
  abra::adjustments::prelude::color::invert().apply(&mut preview);
  let expected = one_shot(|image| abra::adjustments::prelude::color::invert().apply(&mut *image));
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
  abra::adjustments::prelude::levels::vibrance(60).apply(&mut preview);
  let expected = one_shot(|image| abra::adjustments::prelude::levels::vibrance(60).apply(&mut *image));
  assert_ne!(expected, original_pixels());
  // Float math on the two sides can round a channel differently by one.
  wait_for_frame_where(&mut preview, |frame| frame.pixels.iter().zip(&expected).all(|(a, b)| a.abs_diff(*b) <= 1));
}

#[test]
fn vibrance_with_saturation_on_the_gpu_matches_the_cpu_within_rounding() {
  let mut preview = preview();
  abra::adjustments::prelude::levels::vibrance(40).with_saturation(25).apply(&mut preview);
  let expected = one_shot(|image| abra::adjustments::prelude::levels::vibrance(40).with_saturation(25).apply(&mut *image));
  let vibrance_only = one_shot(|image| abra::adjustments::prelude::levels::vibrance(40).apply(&mut *image));
  assert_ne!(expected, vibrance_only, "the saturation step must change something");
  wait_for_frame_where(&mut preview, |frame| frame.pixels.iter().zip(&expected).all(|(a, b)| a.abs_diff(*b) <= 1));
}
