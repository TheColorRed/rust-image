//! MediaPipe selfie skin segmentation using the ONNX Runtime.
//!
//! The model predicts per-pixel probabilities for background, hair, body skin,
//! face skin, clothes, and accessories. [`BodySegmentation`] combines the two
//! skin classes into an Abra [`Mask`]. It is [`AiModel`] with the model's manifest built in, so a caller only needs
//! the `.onnx` file; the manifest says how the model is fed and read.

use abra_ai_core::{AiError, AiModel, AiOutput};
use abra_core::Image;
pub use mask::Mask;
use std::path::Path;

const MANIFEST: &str = include_str!("../models/selfie_multiclass_256x256.yml");

/// Finds face and body skin regions in an image.
pub struct BodySegmentation {
  model: AiModel,
}

pub mod prelude {
  pub use crate::{BodySegmentation, Mask};
  pub use abra_ai_core::AiError;
}

impl BodySegmentation {
  /// Loads a MediaPipe Selfie Multiclass ONNX model from `p_model_path`.
  ///
  /// The caller decides where the model lives. It must have this model's input and output layout.
  pub fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
    Ok(Self {
      model: AiModel::load_with_manifest(p_model_path, MANIFEST)?,
    })
  }

  /// Returns a soft grayscale mask for the face and body skin in `p_image`.
  ///
  /// The model is designed for selfie-like images and internally processes a
  /// stretched 256x256 RGB image. The returned mask is resized to the input
  /// image dimensions.
  pub fn process(&self, p_image: &Image) -> Result<Mask, AiError> {
    match self.model.run(p_image)? {
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
  fn returns_a_mask_at_the_input_dimensions() {
    let model = concat!(env!("CARGO_MANIFEST_DIR"), "/models/selfie_multiclass_256x256.onnx");
    // Without Git LFS the file is a small pointer, not a model.
    if std::fs::metadata(model).map(|metadata| metadata.len()).unwrap_or(0) < 1_000_000 {
      return;
    }
    let image = Image::new(32, 24);
    let mask = BodySegmentation::load(model).expect("model should load").process(&image).expect("segmentation should succeed");

    assert_eq!(mask.dimensions::<u32>(), (32, 24));
  }
}
