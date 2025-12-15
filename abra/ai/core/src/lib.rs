//! AI Model Loading and Discovery
//!
//! This crate provides a unified interface for loading and running ONNX models
//! for image processing. Models are self-describing via YAML manifest files,
//! so no model-specific code is needed.
//!
//! # Overview
//!
//! - `ImageModel`: Unified model that loads any ONNX model with a YAML manifest.
//! - `ModelSpec`: Model specification loaded from YAML.
//! - `ControlParams`: Generic control parameters for models that support them.
//! - `discover_models`: Scan a directory for available models.
//! - `tensor`: Utilities for image-to-tensor conversion (NCHW format).
//! - `tiled`: Utilities for tiled inference with weighted blending.
//! - `onnx`: ONNX Runtime session wrapper (requires `onnx` feature).
//!
//! # Model Manifest Format
//!
//! Each ONNX model should have a `.yml` file with the same base name:
//!
//! ```yaml
//! name: My Model
//! description: What the model does
//! scale_factor: 2.0
//! tile_size: 256
//! tile_overlap: 32
//!
//! # Optional control input
//! control:
//!   size: 3
//!   defaults: [0.5, 0.5, 0.5]
//!   parameters:
//!     - name: param1
//!       description: First parameter
//!       index: 0
//! ```
//!
//! # Example
//!
//! ```ignore
//! use abra_ai_core::prelude::*;
//!
//! // Discover all models in a directory
//! let models = discover_models("models/")?;
//! for spec in &models {
//!   println!("{}: {}", spec.name, spec.description);
//! }
//!
//! // Load and use a model
//! let model = ImageModel::load("models/my-model.onnx")?;
//! let output = model.process(&image)?;
//!
//! // Or with control parameters
//! let ctrl = ControlParams::new(&[0.3, 0.5, 0.7]);
//! let output = model.process_with_control(&image, &ctrl)?;
//! ```

mod error;
pub mod model;
pub mod tensor;
pub mod tiled;

#[cfg(feature = "onnx")]
pub mod onnx;

#[cfg(feature = "onnx")]
mod image_model;

pub use error::AiError;

#[cfg(feature = "onnx")]
pub use image_model::{ControlParameter, ControlParams, ControlSpec, ImageModel, ModelSpec, discover_models};

/// Prelude module for convenient imports.
pub mod prelude {
  pub use crate::error::AiError;
  pub use crate::model::*;
  pub use crate::tensor::{image_to_nchw, nchw_to_image};
  pub use crate::tiled::{TileAccumulator, TileConfig, TileInfo, generate_tiles};

  #[cfg(feature = "onnx")]
  pub use crate::image_model::{ControlParameter, ControlParams, ControlSpec, ImageModel, ModelSpec, discover_models};

  #[cfg(feature = "onnx")]
  pub use crate::onnx::{OnnxConfig, OnnxSession};
}
