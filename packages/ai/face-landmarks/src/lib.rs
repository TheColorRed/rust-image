//! Dense 3D face landmarks for portrait editing.
//!
//! [`FaceLandmarker`] accepts a square crop containing one face and returns MediaPipe's 478-point
//! mesh in that crop's pixel coordinates. A face detector is intentionally kept separate: callers
//! can use the detector appropriate for their workflow, then pass its square face crop here.
//! It is [`AiModel`] with the model's manifest built in.

use abra_ai_core::{AiError, AiModel, AiOutput, ImageAi, ReportsProgress};
pub use abra_ai_core::{Landmark as FaceLandmark, Landmarks as FaceLandmarks};
use abra_core::Image;
use std::path::Path;

const MANIFEST: &str = include_str!("../models/face_landmarks_detector.yml");

/// Extracts MediaPipe's 478-point facial mesh from a square face crop.
pub struct FaceLandmarker {
  model: AiModel,
}

pub mod prelude {
  pub use crate::{FaceLandmark, FaceLandmarker, FaceLandmarks};
  pub use abra_ai_core::{AiError, ImageAi, ReportsProgress};
}

impl ImageAi for FaceLandmarker {
  type Output = FaceLandmarks;

  /// Loads a 478-point MediaPipe Face Landmarker ONNX model from `p_model_path`.
  fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
    Ok(Self {
      model: AiModel::load_with_manifest(p_model_path, MANIFEST)?,
    })
  }

  /// Finds the 478-point mesh in one square face crop.
  ///
  /// Crop the face with a little surrounding area before calling this method. The model uses the
  /// crop's coordinate system, so the returned points are easy to map back to the original photo.
  fn process(&self, p_face: &Image) -> Result<FaceLandmarks, AiError> {
    self.process_reporting(p_face, &|_| {})
  }
}

impl ReportsProgress for FaceLandmarker {
  fn process_reporting(&self, p_face: &Image, p_progress: &dyn Fn(f32)) -> Result<FaceLandmarks, AiError> {
    let (width, height) = p_face.dimensions::<u32>();
    if width != height {
      return Err(AiError::invalid_input("Face Landmarker requires a square face crop"));
    }
    match self.model.run_reporting(p_face, p_progress)? {
      AiOutput::Landmarks(landmarks) => Ok(landmarks),
      _ => Err(AiError::inference_failed("The landmark model did not return landmarks")),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_built_in_manifest_describes_landmarks() {
    let manifest = abra_ai_core::Manifest::parse(MANIFEST).unwrap();
    assert_eq!(manifest.decoder(), Some("landmarks"));
    assert!(abra_ai_core::InputSpec::from_manifest(&manifest).is_ok());
    assert!(abra_ai_core::Decoder::from_manifest(&manifest).is_ok());
  }

  #[test]
  fn runs_the_bundled_model() {
    let model = concat!(env!("CARGO_MANIFEST_DIR"), "/models/face_landmarks_detector.onnx");
    // Without Git LFS the file is a small pointer, not a model.
    if std::fs::metadata(model).map(|metadata| metadata.len()).unwrap_or(0) < 1_000_000 {
      return;
    }
    let landmarker = FaceLandmarker::load(model).unwrap();
    let landmarks = landmarker.process(&Image::new(256, 256)).unwrap();

    assert_eq!(landmarks.landmarks.len(), 478);
    assert!((0.0..=1.0).contains(&landmarks.presence));
    assert!(landmarker.process(&Image::new(256, 128)).is_err(), "a crop must be square");
  }
}
