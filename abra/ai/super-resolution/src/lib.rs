//! Super Resolution Models
//!
//! This crate provides pre-trained models for image enhancement and restoration.
//! Models are stored as ONNX files with YAML manifests describing their capabilities.
//!
//! # Available Models
//!
//! - **SCUNet-GAN**: 1x restoration (removes artifacts, noise, blur)
//! - **UltraZoom-2X-Ctrl**: 2x upscale with controllable enhancement
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
//! let image = Image::new_from_path("input.jpg");
//! // Load and run super resolution model
//! let output = SuperResolution::load("UltraZoom-2X-Ctrl")?.process(&image);
//! // Save output
//! output.save("output.png", None);
//! ```
//!
//! # Adding New Models
//!
//! 1. Place the `.onnx` file in the `models/` directory
//! 2. Create a `.yml` manifest with the same base name
//! 3. The model will be automatically discovered
//!
//! See `models/UltraZoom-2X-Ctrl.yml` for an example manifest format.

use abra_ai_core::{discover_models, prelude::*};
use abra_core::Image;

pub mod prelude {
  pub use crate::SuperResolution;
  pub use abra_ai_core::ControlParams;
  pub use abra_ai_core::model::*;
}

pub struct SuperResolution {
  model: ImageModel,
}

impl AiModel for SuperResolution {
  /// Load a model to be used by the AI tool.
  ///
  /// # Arguments
  ///
  /// - `p_name`: The name of the model to load (e.g., "UltraZoom-2X-Ctrl").
  fn load(p_name: impl AsRef<str>) -> Self {
    // Find models in the super-resolution package.
    let models = discover_models("packages/ai/super-resolution/models")
      .expect("Could not find associated models for Super Resolution");

    // Load the model by name.
    let model = ImageModel::new(models)
      .load_by_name(p_name.as_ref())
      .expect("Failed to load the model for Super Resolution");
    Self { model }
  }
}

impl AiProcessModel for SuperResolution {
  /// Processes an input image and returns the enhanced output image.
  ///
  /// # Arguments
  ///
  /// - `p_image`: The input image to be processed.
  fn process(&self, p_image: &Image) -> Image {
    self
      .model
      .process(p_image)
      .expect("Image processing failed for Super Resolution")
  }
}

impl AiProcessModelControl for SuperResolution {
  /// Processes an input image with control parameters and returns the enhanced output image.
  ///
  /// # Arguments
  ///
  /// - `p_image`: The input image to be processed.
  /// - `p_ctrl`: Control parameters.
  fn process_with_control(&self, p_image: &Image, p_ctrl: &ControlParams) -> Image {
    self
      .model
      .process_with_control(p_image, p_ctrl)
      .expect("Image processing with control failed for Super Resolution")
  }
}
