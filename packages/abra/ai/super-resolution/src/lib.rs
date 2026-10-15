//! Super Resolution Models
//!
//! This crate provides pre-trained models for image enhancement and restoration.
//! Models are stored as ONNX files with YAML manifests describing their capabilities.
//!
//! # Available Models
//!
//! - **SCUNet-GAN**: 1x restoration (removes artifacts, noise, blur)
//! - **MewZoom-V0-2X-Ctrl**: 2x upscale with controllable enhancement
//! - **MewZoom-V1-2X**: 2x upscale with the current MewZoom architecture
//!
//! # Usage
//!
//! This crate simply provides the model files. Use `abra-ai-core` to load and run them:
//!
//! ```ignore
//! use abra::prelude::*;
//! use abra_super_resolution::prelude::*;
//!
//! // Load image
//! let image = Image::read("input.jpg")?;
//! // Load and run super resolution model
//! let output = SuperResolution::load("packages/abra/ai/super-resolution/models/MewZoom-V1-2X.onnx")?.process(&image)?;
//! // Save output
//! output.write("output.png", None)?;
//! ```
//!
//! # Adding New Models
//!
//! 1. Place the `.onnx` file in the `models/` directory
//! 2. Create a `.yml` manifest with the same base name
//! 3. Pass the `.onnx` path to `SuperResolution::load`
//!
//! See `models/MewZoom-V1-2X.yml` for an example manifest format.

use abra_ai_core::{AiError, ControlParams, ImageAi, ImageAiWithControl, ImageModel, ReportsProgress};
use abra_core::Image;
use std::path::Path;

pub mod prelude {
  pub use crate::SuperResolution;
  pub use abra_ai_core::{AiError, ControlParams, ImageAi, ImageAiWithControl, ReportsProgress};
}

/// An [`ImageModel`] for upscaling and restoring pictures.
pub struct SuperResolution {
  model: ImageModel,
}

impl ImageAi for SuperResolution {
  type Output = Image;

  /// Loads a super-resolution model from its `.onnx` file and the `.yml` manifest beside it.
  ///
  /// - `p_model_path`: Path to the ONNX model file (e.g. `models/MewZoom-V1-2X.onnx`).
  fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
    Ok(Self {
      model: ImageModel::load(p_model_path)?,
    })
  }

  /// Processes an input image and returns the enhanced output image.
  ///
  /// - `p_image`: The input image to be processed.
  fn process(&self, p_image: &Image) -> Result<Image, AiError> {
    self.model.process(p_image)
  }
}

impl ImageAiWithControl for SuperResolution {
  /// Processes an input image with control parameters and returns the enhanced output image.
  ///
  /// - `p_image`: The input image to be processed.
  /// - `p_control`: Control parameters.
  fn process_with_control(&self, p_image: &Image, p_control: &ControlParams) -> Result<Image, AiError> {
    self.model.process_with_control(p_image, p_control)
  }
}

impl ReportsProgress for SuperResolution {
  fn process_reporting(&self, p_input: &Image, p_progress: &dyn Fn(f32)) -> Result<Image, AiError> {
    self.model.process_reporting(p_input, p_progress)
  }
}
