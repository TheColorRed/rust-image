//! MediaPipe selfie skin segmentation using the ONNX Runtime.
//!
//! The model predicts per-pixel probabilities for background, hair, body skin,
//! face skin, clothes, and accessories. [`BodySegmentation`] combines the two
//! skin classes into an Abra [`Mask`]. It is [`AiModel`] with the model's manifest built in, so a caller only needs
//! the `.onnx` file; the manifest says how the model is fed and read.
//!
//! [`BodyParser`] reads the same model another way: it labels every pixel as one of the six, and the [`BodySegments`] it gives has a
//! mask for any [`BodyRegion`], so the hair, the clothes or the accessories can be picked out as well as the skin.

use abra_ai_core::{AiError, AiModel, AiOutput, ImageAi, LabelMap, Placeable, ReportsProgress};
use abra_core::Image;
pub use mask::Mask;
use std::path::Path;

const MANIFEST: &str = include_str!("../models/selfie_multiclass_256x256.yml");
const REGIONS_MANIFEST: &str = include_str!("../models/selfie_multiclass_256x256_regions.yml");

/// The regions the selfie model tells apart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BodyRegion {
  Background = 0,
  Hair = 1,
  BodySkin = 2,
  FaceSkin = 3,
  Clothes = 4,
  /// Glasses, jewelry and other things worn that are not clothes.
  Accessories = 5,
}

/// The region of every pixel of a picture, at the size of the picture.
#[derive(Clone, Debug)]
pub struct BodySegments {
  labels: LabelMap,
}

impl BodySegments {
  /// The region at a pixel, or `None` outside the picture and where no region was found, such as outside the part of a picture
  /// the model was run on (see [`ImageAi::process_within`]).
  pub fn region_at(&self, p_x: u32, p_y: u32) -> Option<BodyRegion> {
    self.labels.label_at(p_x, p_y).and_then(region_from_label)
  }

  /// A hard mask that is `255` where the region is and `0` elsewhere.
  pub fn mask(&self, p_region: BodyRegion) -> Mask {
    self.labels.mask(p_region as u8)
  }

  /// A hard mask that is `255` where any of the regions is and `0` elsewhere, such as the body skin and the face skin together.
  pub fn mask_of(&self, p_regions: &[BodyRegion]) -> Mask {
    let classes: Vec<u8> = p_regions.iter().map(|region| *region as u8).collect();
    self.labels.mask_of(&classes)
  }
}

impl Placeable for BodySegments {
  /// The pixels outside the part have no region.
  fn placed(&self, p_width: u32, p_height: u32, p_x: u32, p_y: u32) -> Self {
    Self {
      labels: self.labels.placed(p_width, p_height, p_x, p_y),
    }
  }
}

/// Labels the regions of a person in a picture: the background, the hair, the skin of the body and of the face, the clothes and the
/// accessories. It is [`AiModel`] with the model's manifest built in, and takes the same `.onnx` file as [`BodySegmentation`].
pub struct BodyParser {
  model: AiModel,
}

fn region_from_label(p_label: u8) -> Option<BodyRegion> {
  match p_label {
    0 => Some(BodyRegion::Background),
    1 => Some(BodyRegion::Hair),
    2 => Some(BodyRegion::BodySkin),
    3 => Some(BodyRegion::FaceSkin),
    4 => Some(BodyRegion::Clothes),
    5 => Some(BodyRegion::Accessories),
    _ => None,
  }
}

/// Finds face and body skin regions in an image.
pub struct BodySegmentation {
  model: AiModel,
}

pub mod prelude {
  pub use crate::{BodyParser, BodyRegion, BodySegmentation, BodySegments, Mask};
  pub use abra_ai_core::{AiError, ImageAi, Placeable, ReportsProgress};
}

impl ImageAi for BodyParser {
  type Output = BodySegments;

  /// Loads a MediaPipe Selfie Multiclass ONNX model from `p_model_path`.
  ///
  /// The caller decides where the model lives. It must have this model's input and output layout.
  fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
    Ok(Self {
      model: AiModel::load_with_manifest(p_model_path, REGIONS_MANIFEST)?,
    })
  }

  /// Labels every pixel of `p_image` with its region.
  ///
  /// The model is designed for selfie-like images and internally processes a stretched 256x256 RGB image, so a person who is a small
  /// part of the picture is only a few pixels to it. [`ImageAi::process_within`] runs it on the part with the person in it.
  fn process(&self, p_image: &Image) -> Result<BodySegments, AiError> {
    self.process_reporting(p_image, &|_| {})
  }
}

impl ReportsProgress for BodyParser {
  fn process_reporting(&self, p_image: &Image, p_progress: &dyn Fn(f32)) -> Result<BodySegments, AiError> {
    match self.model.run_reporting(p_image, p_progress)? {
      AiOutput::Labels(labels) => Ok(BodySegments { labels }),
      _ => Err(AiError::inference_failed("The body model did not return labels")),
    }
  }
}

impl ImageAi for BodySegmentation {
  type Output = Mask;

  /// Loads a MediaPipe Selfie Multiclass ONNX model from `p_model_path`.
  ///
  /// The caller decides where the model lives. It must have this model's input and output layout.
  fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
    Ok(Self {
      model: AiModel::load_with_manifest(p_model_path, MANIFEST)?,
    })
  }

  /// Returns a soft grayscale mask for the face and body skin in `p_image`.
  ///
  /// The model is designed for selfie-like images and internally processes a
  /// stretched 256x256 RGB image. The returned mask is resized to the input
  /// image dimensions.
  fn process(&self, p_image: &Image) -> Result<Mask, AiError> {
    self.process_reporting(p_image, &|_| {})
  }
}

impl ReportsProgress for BodySegmentation {
  fn process_reporting(&self, p_image: &Image, p_progress: &dyn Fn(f32)) -> Result<Mask, AiError> {
    match self.model.run_reporting(p_image, p_progress)? {
      AiOutput::Mask(mask) => Ok(mask),
      _ => Err(AiError::inference_failed("The skin model did not return a mask")),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_built_in_manifest_describes_a_semantic_mask() {
    let manifest = abra_ai_core::Manifest::parse(MANIFEST).unwrap();
    assert_eq!(manifest.decoder(), Some("semantic-mask"));
    assert!(abra_ai_core::InputSpec::from_manifest(&manifest).is_ok());
    assert!(abra_ai_core::Decoder::from_manifest(&manifest).is_ok());
  }

  #[test]
  fn the_regions_manifest_describes_a_label_map_of_the_six_regions() {
    let manifest = abra_ai_core::Manifest::parse(REGIONS_MANIFEST).unwrap();
    assert_eq!(manifest.decoder(), Some("label-map"));
    assert_eq!(manifest.classes().len(), 6);
    assert!(abra_ai_core::InputSpec::from_manifest(&manifest).is_ok());
    assert!(abra_ai_core::Decoder::from_manifest(&manifest).is_ok());
  }

  #[test]
  fn a_region_is_read_from_its_label_and_a_mask_is_made_for_one_or_several() {
    let segments = BodySegments {
      labels: LabelMap {
        width: 6,
        height: 1,
        labels: vec![0, 1, 2, 3, 4, 5],
        classes: Vec::new(),
      },
    };
    assert_eq!(segments.region_at(1, 0), Some(BodyRegion::Hair));
    assert_eq!(segments.region_at(3, 0), Some(BodyRegion::FaceSkin));
    assert_eq!(segments.region_at(6, 0), None);
    assert_eq!(segments.mask(BodyRegion::Clothes).values(), &[0, 0, 0, 0, 255, 0]);
    // The two skins together.
    assert_eq!(segments.mask_of(&[BodyRegion::BodySkin, BodyRegion::FaceSkin]).values(), &[0, 0, 255, 255, 0, 0]);
  }

  #[test]
  fn regions_found_in_a_part_have_none_outside_it() {
    let part = BodySegments {
      labels: LabelMap {
        width: 2,
        height: 1,
        labels: vec![1, 4],
        classes: Vec::new(),
      },
    };
    let whole = part.placed(5, 2, 2, 1);
    assert_eq!(whole.region_at(2, 1), Some(BodyRegion::Hair));
    assert_eq!(whole.region_at(3, 1), Some(BodyRegion::Clothes));
    assert_eq!(whole.region_at(0, 0), None);
    assert_eq!(whole.mask(BodyRegion::Background).values(), &[0; 10]);
  }

  #[test]
  fn labels_every_pixel_and_works_on_a_part_of_a_picture() {
    let model = concat!(env!("CARGO_MANIFEST_DIR"), "/models/selfie_multiclass_256x256.onnx");
    // Without Git LFS the file is a small pointer, not a model.
    if std::fs::metadata(model).map(|metadata| metadata.len()).unwrap_or(0) < 1_000_000 {
      return;
    }
    let parser = BodyParser::load(model).expect("model should load");
    let image = Image::new(64, 48);
    let whole = parser.process(&image).expect("parsing should succeed");
    assert!(whole.region_at(0, 0).is_some() && whole.region_at(63, 47).is_some());
    // Run on a part, the pixels outside it have no region.
    let part = parser
      .process_within(&image, abra_core::Rect::new((16, 12), (32, 24)), 0.0)
      .expect("parsing a part should succeed");
    assert!(part.region_at(20, 20).is_some());
    assert_eq!(part.region_at(0, 0), None);
  }

  #[test]
  fn returns_a_mask_at_the_input_dimensions() {
    let model = concat!(env!("CARGO_MANIFEST_DIR"), "/models/selfie_multiclass_256x256.onnx");
    // Without Git LFS the file is a small pointer, not a model.
    if std::fs::metadata(model).map(|metadata| metadata.len()).unwrap_or(0) < 1_000_000 {
      return;
    }
    let image = Image::new(32, 24);
    let mask =
      BodySegmentation::load(model).expect("model should load").process(&image).expect("segmentation should succeed");

    assert_eq!(mask.dimensions::<u32>(), (32, 24));
  }
}
