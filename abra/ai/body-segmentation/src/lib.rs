//! MediaPipe selfie skin segmentation using the ONNX Runtime.
//!
//! The model predicts per-pixel probabilities for background, hair, body skin,
//! face skin, clothes, and accessories. [`BodySegmentation`] combines the two
//! skin classes into an Abra [`Mask`].

use abra_ai_core::{AiError, onnx::OnnxSession};
use abra_core::{Channels, Image, ResizeTarget, Size, Transform, TransformAlgorithm};
pub use mask::Mask;
use std::sync::OnceLock;

const MODEL_BYTES: &[u8] = include_bytes!("../models/selfie_multiclass_256x256.onnx");

const MODEL_SIZE: u32 = 256;
const CLASS_COUNT: usize = 6;
const BODY_SKIN_CLASS: usize = 2;
const FACE_SKIN_CLASS: usize = 3;

/// Finds face and body skin regions in an image.
pub struct BodySegmentation {
  session: OnnxSession,
}

static BODY_SEGMENTATION: OnceLock<Result<BodySegmentation, String>> = OnceLock::new();

/// Segments face and body skin in an image, loading and caching the model on first use.
pub fn segment_skin(p_image: &Image) -> Result<Mask, AiError> {
  let segmenter = BODY_SEGMENTATION
    .get_or_init(|| BodySegmentation::load().map_err(|error| error.to_string()))
    .as_ref()
    .map_err(|error| AiError::model_load_failed(error.clone()))?;
  segmenter.process(p_image)
}

pub mod prelude {
  pub use crate::{BodySegmentation, Mask, segment_skin};
  pub use abra_ai_core::AiError;
}

impl BodySegmentation {
  /// Loads the bundled MediaPipe Selfie Multiclass ONNX model.
  pub fn load() -> Result<Self, AiError> {
    let session = OnnxSession::from_bytes(MODEL_BYTES, None)?;
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
    let rgba_mask: Vec<u8> = gray_mask.into_iter().flat_map(|value| [value, value, value, 255]).collect();
    let mut mask_image = Image::new_from_pixels(MODEL_SIZE, MODEL_SIZE, rgba_mask, Channels::RGBA);
    mask_image.resize(ResizeTarget::Exact(Size::new(width, height)), TransformAlgorithm::Bilinear);
    Ok(Mask::from_image(mask_image))
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
  fn public_helper_returns_a_mask_at_the_input_dimensions() {
    let image = Image::new(32, 24);
    let mask = segment_skin(&image).expect("segmentation should succeed");

    assert_eq!(mask.image().dimensions::<u32>(), (32, 24));
  }
}
