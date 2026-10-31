//! The object removal plugins that ship in `packages/ai/object-removal/plugins` are found by their capability, and every one with the
//! `inpaint` capability does what it says. A new remover is a new folder there, and these tests run it without being changed.

use super::error::PluginError;
use super::{Contribution, PluginRegistry};
use abra_ai_core::{AiModel, AiOutput};
use abra_core::{Channels, Image};
use mask::Mask;

const PLUGINS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ai/object-removal/plugins");

/// A smooth picture with a black square in the middle, and the mask of that square.
fn damaged(side: u32) -> (Image, Mask) {
  let mut pixels: Vec<u8> =
    (0..side * side).flat_map(|index| [((index % side) * 255 / side) as u8, 120, 200, 255]).collect();
  let mut values = vec![0u8; (side * side) as usize];
  let (from, to) = (side * 2 / 5, side * 3 / 5);
  for y in from..to {
    for x in from..to {
      let at = (y * side + x) as usize;
      pixels[at * 4..at * 4 + 3].copy_from_slice(&[0, 0, 0]);
      values[at] = 255;
    }
  }
  (Image::new_from_pixels(side, side, pixels, Channels::RGBA), Mask::from_values(side, side, values))
}

fn removed(model: &AiModel, photo: &Image, mask: &Mask) -> Image {
  match model.run_guided(photo, Some(mask), &|_| {}).unwrap() {
    AiOutput::Image(image) => image,
    other => panic!("expected a picture, got {other:?}"),
  }
}

/// The model of a plugin contribution.
fn model_of(p_contribution: &Contribution) -> AiModel {
  let Contribution::Model(plugin, model) = p_contribution else { panic!("not a model") };
  AiModel::load(plugin.path(&model.model)).unwrap()
}

#[test]
fn the_plugins_here_are_found_by_the_capability_they_provide() {
  let registry = PluginRegistry::scan(PLUGINS);
  // A plugin whose big file is not downloaded (LaMa's is left out of git) is reported, not hidden.
  for problem in registry.problems() {
    assert!(matches!(problem.error, PluginError::FileNotFound(_)), "{problem:?}");
  }
  let found = registry.with_capability("inpaint");
  assert!(found.iter().any(|contribution| contribution.id() == "object-removal"), "MI-GAN is here");
  assert!(found.iter().all(|contribution| matches!(contribution, Contribution::Model(..))));
}

/// Every remover found, whichever it is: a picture of the same size comes back, only the masked pixels change, and the black square is
/// filled with what is round it. A new remover plugin is covered by this without being named.
#[test]
fn every_plugin_with_the_inpaint_capability_removes_what_the_mask_covers_and_keeps_the_rest() {
  let registry = PluginRegistry::scan(PLUGINS);
  for contribution in registry.with_capability("inpaint") {
    let model = model_of(&contribution);
    // Bigger than the 512 a fixed-size model fills, so the part round the mask is scaled both ways.
    let (photo, mask) = damaged(900);
    let result = removed(&model, &photo, &mask);
    assert_eq!(result.dimensions::<u32>(), (900, 900), "{}", contribution.id());
    let (inside, outside) = (result.get_pixel(450, 450).unwrap(), result.get_pixel(20, 20).unwrap());
    assert_eq!(outside, photo.get_pixel(20, 20).unwrap(), "{}: outside the mask is kept", contribution.id());
    // The picture is a smooth blend, so the middle of the filled square should be near the blend there: about (127, 120, 200).
    assert!(
      inside.0.abs_diff(127) < 80 && inside.1.abs_diff(120) < 80 && inside.2.abs_diff(200) < 80,
      "{}: the square is filled with what is round it: {inside:?}",
      contribution.id()
    );
  }
}

#[test]
fn a_photo_that_is_not_square_comes_back_the_same_size() {
  let registry = PluginRegistry::scan(PLUGINS);
  let found = registry.with_capability("inpaint");
  let model = model_of(&found[0]);
  let pixels: Vec<u8> =
    (0..300u32 * 200).flat_map(|index| [(index % 300) as u8, (index / 300) as u8, 128, 255]).collect();
  let photo = Image::new_from_pixels(300, 200, pixels, Channels::RGBA);
  let mut values = vec![0u8; 300 * 200];
  for y in 80..120 {
    for x in 130..170 {
      values[y * 300 + x] = 255;
    }
  }
  let result = removed(&model, &photo, &Mask::from_values(300, 200, values));
  assert_eq!(result.dimensions::<u32>(), (300, 200));
}

#[test]
fn a_mask_of_another_size_is_an_error_and_so_is_no_mask() {
  let registry = PluginRegistry::scan(PLUGINS);
  let model = model_of(&registry.with_capability("inpaint")[0]);
  let photo = Image::new_from_color(8, 8, abra_core::Color::from_rgba(0, 0, 0, 255));
  assert!(model.run_guided(&photo, Some(&Mask::empty(4, 4)), &|_| {}).is_err());
  assert!(model.run_guided(&photo, None, &|_| {}).is_err());
}
