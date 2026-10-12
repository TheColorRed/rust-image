//! Pixel-level parsing of facial features.
//!
//! [`FaceParser`] labels a face crop as skin, eyes, eyebrows, ears, nose, mouth, lips, hair, and
//! related regions. Use the returned [`FaceSegmentation`] to create a mask for any one feature.
//! It is [`AiModel`] with the model's manifest built in.

use abra_ai_core::{AiError, AiModel, AiOutput, LabelMap};
use abra_core::Image;
pub use mask::Mask;
use std::path::Path;

const MANIFEST: &str = include_str!("../models/resnet18.yml");

/// Regions labelled by the CelebAMask-HQ face-parsing model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FaceRegion {
  Background = 0,
  Skin = 1,
  LeftEyebrow = 2,
  RightEyebrow = 3,
  LeftEye = 4,
  RightEye = 5,
  Eyeglasses = 6,
  LeftEar = 7,
  RightEar = 8,
  Earring = 9,
  Nose = 10,
  Mouth = 11,
  UpperLip = 12,
  LowerLip = 13,
  Neck = 14,
  Neckwear = 15,
  Clothing = 16,
  Hair = 17,
  Hat = 18,
}

/// Per-pixel face-region labels at the input image's dimensions.
#[derive(Clone, Debug)]
pub struct FaceSegmentation {
  labels: LabelMap,
}

impl FaceSegmentation {
  /// Returns the label at a pixel, or `None` when it is outside the mask.
  pub fn region_at(&self, p_x: u32, p_y: u32) -> Option<FaceRegion> {
    self.labels.label_at(p_x, p_y).and_then(region_from_label)
  }

  /// Returns a hard mask containing only `p_region`.
  pub fn mask(&self, p_region: FaceRegion) -> Mask {
    self.labels.mask(p_region as u8)
  }
}

/// Labels facial features in a face crop with a BiSeNet face-parsing model.
pub struct FaceParser {
  model: AiModel,
}

pub mod prelude {
  pub use crate::{FaceParser, FaceRegion, FaceSegmentation, Mask};
  pub use abra_ai_core::AiError;
}

impl FaceParser {
  /// Loads a 19-class BiSeNet face-parsing ONNX model from `p_model_path`.
  pub fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
    Ok(Self {
      model: AiModel::load_with_manifest(p_model_path, MANIFEST)?,
    })
  }

  /// Labels facial features in `p_face`, returning masks at its original dimensions.
  ///
  /// The parser is trained for a close face crop. Calling it on a full photograph is supported but
  /// produces the best result only when the face occupies most of the input.
  pub fn process(&self, p_face: &Image) -> Result<FaceSegmentation, AiError> {
    match self.model.run(p_face)? {
      AiOutput::Labels(labels) => Ok(FaceSegmentation { labels }),
      _ => Err(AiError::inference_failed("The face-parsing model did not return labels")),
    }
  }
}

fn region_from_label(p_label: u8) -> Option<FaceRegion> {
  match p_label {
    0 => Some(FaceRegion::Background),
    1 => Some(FaceRegion::Skin),
    2 => Some(FaceRegion::LeftEyebrow),
    3 => Some(FaceRegion::RightEyebrow),
    4 => Some(FaceRegion::LeftEye),
    5 => Some(FaceRegion::RightEye),
    6 => Some(FaceRegion::Eyeglasses),
    7 => Some(FaceRegion::LeftEar),
    8 => Some(FaceRegion::RightEar),
    9 => Some(FaceRegion::Earring),
    10 => Some(FaceRegion::Nose),
    11 => Some(FaceRegion::Mouth),
    12 => Some(FaceRegion::UpperLip),
    13 => Some(FaceRegion::LowerLip),
    14 => Some(FaceRegion::Neck),
    15 => Some(FaceRegion::Neckwear),
    16 => Some(FaceRegion::Clothing),
    17 => Some(FaceRegion::Hair),
    18 => Some(FaceRegion::Hat),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_built_in_manifest_names_every_region() {
    let manifest = abra_ai_core::Manifest::parse(MANIFEST).unwrap();
    assert_eq!(manifest.decoder(), Some("label-map"));
    assert_eq!(manifest.classes().len(), 19);
    assert!(abra_ai_core::InputSpec::from_manifest(&manifest).is_ok());
    assert!(abra_ai_core::Decoder::from_manifest(&manifest).is_ok());
  }

  #[test]
  fn a_region_is_read_from_its_label() {
    let segmentation = FaceSegmentation {
      labels: LabelMap {
        width: 2,
        height: 1,
        labels: vec![FaceRegion::Nose as u8, 200],
        classes: Vec::new(),
      },
    };
    assert_eq!(segmentation.region_at(0, 0), Some(FaceRegion::Nose));
    assert_eq!(segmentation.region_at(1, 0), None, "a label outside the 19 regions");
    assert_eq!(segmentation.region_at(2, 0), None, "outside the picture");
    assert_eq!(segmentation.mask(FaceRegion::Nose).values(), &[255, 0]);
  }

  #[test]
  fn runs_the_bundled_model() {
    let model = concat!(env!("CARGO_MANIFEST_DIR"), "/models/resnet18.onnx");
    // Without Git LFS the file is a small pointer, not a model.
    if std::fs::metadata(model).map(|metadata| metadata.len()).unwrap_or(0) < 1_000_000 {
      return;
    }
    let segmentation = FaceParser::load(model).unwrap().process(&Image::new(32, 24)).unwrap();

    assert_eq!(segmentation.mask(FaceRegion::Skin).dimensions::<u32>(), (32, 24));
  }
}
