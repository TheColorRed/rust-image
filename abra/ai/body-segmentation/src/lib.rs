//! MediaPipe selfie skin segmentation using the ONNX Runtime.
//!
//! The model predicts per-pixel probabilities for background, hair, body skin,
//! face skin, clothes, and accessories. [`BodySegmentation`] combines the two
//! skin classes into an Abra [`Mask`].

use abra_ai_core::{AiError, onnx::OnnxSession};
use abra_core::{Image, ResizeTarget, Size, Transform, TransformAlgorithm};
pub use mask::Mask;
use std::path::Path;

const MODEL_SIZE: u32 = 256;
const CLASS_COUNT: usize = 6;
const BODY_SKIN_CLASS: usize = 2;
const FACE_SKIN_CLASS: usize = 3;

/// Finds face and body skin regions in an image.
pub struct BodySegmentation {
  session: OnnxSession,
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
    let session = OnnxSession::from_file(p_model_path, None)?;
    Ok(Self { session })
  }

  /// Returns a soft grayscale mask for the face and body skin in `p_image`.
  ///
  /// The model is designed for selfie-like images and internally processes a
  /// stretched 256x256 RGB image. The returned mask is resized to the input
  /// image dimensions.
  pub fn process(&self, p_image: &Image) -> Result<Mask, AiError> {
    let (width, height) = p_image.dimensions::<u32>();
    if width == 0 || height == 0 {
      return Err(AiError::invalid_input("Cannot segment an empty image"));
    }

    let mut model_image = p_image.clone();
    model_image.resize(ResizeTarget::Exact(Size::new(MODEL_SIZE, MODEL_SIZE)), TransformAlgorithm::Bilinear);
    let mut input = Vec::with_capacity((MODEL_SIZE * MODEL_SIZE * 3) as usize);
    for pixel in model_image.rgba().chunks_exact(4) {
      input.extend([
        pixel[0] as f32 / 255.0,
        pixel[1] as f32 / 255.0,
        pixel[2] as f32 / 255.0,
      ]);
    }

    let (output_shape, logits) = self.session.run_single(&input, &[1, MODEL_SIZE as usize, MODEL_SIZE as usize, 3])?;
    let expected_shape = [1, MODEL_SIZE as usize, MODEL_SIZE as usize, CLASS_COUNT];
    if output_shape != expected_shape {
      return Err(AiError::inference_failed(format!("Unexpected segmentation output shape: {output_shape:?}")));
    }

    let gray_mask = skin_mask_values(&logits)?;
    Ok(Mask::from_values(MODEL_SIZE, MODEL_SIZE, gray_mask).resized(width, height, TransformAlgorithm::Bilinear))
  }
}

fn skin_mask_values(p_logits: &[f32]) -> Result<Vec<u8>, AiError> {
  let expected_len = (MODEL_SIZE * MODEL_SIZE) as usize * CLASS_COUNT;
  if p_logits.len() != expected_len {
    return Err(AiError::inference_failed(format!(
      "Expected {expected_len} segmentation logits, got {}",
      p_logits.len()
    )));
  }

  Ok(
    p_logits
      .chunks_exact(CLASS_COUNT)
      .map(|pixel_logits| {
        let confidence = skin_confidence(pixel_logits);
        (confidence * 255.0).round().clamp(0.0, 255.0) as u8
      })
      .collect(),
  )
}

fn skin_confidence(p_logits: &[f32]) -> f32 {
  let max_logit = p_logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
  let exp_sum: f32 = p_logits.iter().map(|logit| (logit - max_logit).exp()).sum();
  let skin_sum = (p_logits[BODY_SKIN_CLASS] - max_logit).exp() + (p_logits[FACE_SKIN_CLASS] - max_logit).exp();
  skin_sum / exp_sum
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn combines_body_and_face_skin_probabilities() {
    let logits = [0.0, 0.0, 2.0, 3.0, 0.0, 0.0];
    let confidence = skin_confidence(&logits);
    let expected = (2.0_f32.exp() + 3.0_f32.exp()) / (2.0_f32.exp() + 3.0_f32.exp() + 4.0);

    assert!((confidence - expected).abs() < 1e-6);
  }

  #[test]
  fn rejects_logits_with_an_unexpected_length() {
    assert!(skin_mask_values(&[]).is_err());
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
