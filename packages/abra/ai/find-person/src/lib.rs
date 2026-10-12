//! YOLO26 instance segmentation for selecting people in an image.
//!
//! [`PersonSegmenter`] is [`AiModel`] with the model's manifest built in, so a caller only needs the `.onnx` file.

use abra_ai_core::{AiError, AiModel, AiOutput, Bounds, Instance};
use abra_core::Image;
pub use mask::Mask;
use std::path::Path;

const MANIFEST: &str = include_str!("../models/yolo26n-seg.yml");

/// The bounds of one detected person, in source-image pixels.
pub type PersonBounds = Bounds;

/// One person instance and its soft, per-pixel silhouette mask.
#[derive(Clone, Debug)]
pub struct PersonInstance {
  pub bounds: PersonBounds,
  pub confidence: f32,
  pub mask: Mask,
}

/// Finds person instances using a YOLO26n segmentation model supplied by the caller.
pub struct PersonSegmenter {
  model: AiModel,
}

pub mod prelude {
  pub use crate::{PersonBounds, PersonInstance, PersonSegmenter};
  pub use abra_ai_core::AiError;
  pub use mask::Mask;
}

impl PersonSegmenter {
  /// Loads a YOLO26n-seg ONNX model from `p_model_path`.
  ///
  /// The caller decides where the model lives. It must have this model's input and output layout.
  pub fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
    Ok(Self {
      model: AiModel::load_with_manifest(p_model_path, MANIFEST)?,
    })
  }

  /// Finds people in `p_image`, returning source-sized instance masks.
  pub fn process(&self, p_image: &Image) -> Result<Vec<PersonInstance>, AiError> {
    match self.model.run(p_image)? {
      AiOutput::Instances(people) => Ok(
        people
          .into_iter()
          .map(|Instance { bounds, confidence, mask, .. }| PersonInstance { bounds, confidence, mask })
          .collect(),
      ),
      _ => Err(AiError::inference_failed("The person model did not return instances")),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_built_in_manifest_describes_instances() {
    let manifest = abra_ai_core::Manifest::parse(MANIFEST).unwrap();
    assert_eq!(manifest.decoder(), Some("instance-segmentation"));
    assert!(abra_ai_core::InputSpec::from_manifest(&manifest).is_ok());
    assert!(abra_ai_core::Decoder::from_manifest(&manifest).is_ok());
  }
}
