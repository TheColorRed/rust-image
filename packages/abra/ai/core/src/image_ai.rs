//! What the AI crates that take a picture share.

use crate::{AiError, ControlParams};
use abra_core::Image;
use std::path::Path;

/// A model that is loaded from an `.onnx` file and processes a picture.
///
/// The crates that wrap [`AiModel`](crate::AiModel) with a manifest of their own (face landmarks, face parsing, body
/// segmentation, person finding) and the image-to-image [`ImageModel`](crate::ImageModel) all work this way. They differ
/// in what comes out, so that is [`Output`](Self::Output).
///
/// ```ignore
/// let output = FaceParser::load("models/resnet18.onnx")?.process(&photo)?;
/// ```
pub trait ImageAi: Sized {
  /// What processing a picture returns.
  type Output;

  /// Loads the model from its `.onnx` file. The library never looks for models, so the caller gives the path.
  ///
  /// - `p_model_path`: Path to the ONNX model file.
  fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError>;

  /// Processes a picture.
  ///
  /// - `p_image`: The picture to process.
  fn process(&self, p_image: &Image) -> Result<Self::Output, AiError>;
}

/// An [`ImageAi`] model that also takes control values, such as how strongly to enhance.
pub trait ImageAiWithControl: ImageAi {
  /// Processes a picture with control values, one for each of the model's controls.
  ///
  /// - `p_image`: The picture to process.
  /// - `p_control`: The control values.
  fn process_with_control(&self, p_image: &Image, p_control: &ControlParams) -> Result<Self::Output, AiError>;
}
