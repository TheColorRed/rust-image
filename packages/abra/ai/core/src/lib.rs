//! AI Model Loading and Discovery
//!
//! This crate provides a unified interface for loading and running ONNX models
//! for image processing. Models are self-describing via YAML manifest files,
//! so no model-specific code is needed.
//!
//! # Overview
//!
//! - `AiModel`: Any model, described by its manifest: it feeds the picture in, runs the model and reads the result with
//!   the decoder the manifest names. A new model of a known kind needs only its `.onnx` and `.yml`.
//! - `Manifest`, `InputSpec` and `Decoder`: what an `AiModel` is made of (`manifest`, `input` and `decode`).
//! - A model with more than one input, bytes instead of numbers, or a picture for an answer lists its `inputs` in the manifest and
//!   needs no Rust at all (the `graph` module); the object removal models (MI-GAN, LaMa) are two such manifests.
//! - `ImageAi` and `ImageAiWithControl`: what the models that take a picture share, `load` and `process`.
//! - `ReportsProgress`: `model.on(|progress| ..).process(&image)` tells how far along a run is, from `0.0` to `1.0`.
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

mod ai_model;
pub mod decode;
mod error;
mod graph;
pub mod input;
pub mod manifest;
pub mod tensor;
pub mod tiled;

pub mod onnx;

mod image_ai;
mod image_model;
mod progress;

pub use ai_model::AiModel;
pub use decode::{AiOutput, Bounds, Decoder, Instance, LabelMap, Landmark, Landmarks};
pub use error::AiError;
pub use input::{Fit, InputSpec, Layout, Placement};
pub use manifest::{Entry, Manifest};

pub use image_ai::{ImageAi, ImageAiWithControl, Placeable};
pub use image_model::{ControlParameter, ControlParams, ControlSpec, ImageModel, ModelSpec, discover_models};
pub use progress::{Listening, ReportsProgress};

/// Prelude module for convenient imports.
pub mod prelude {
  pub use crate::error::AiError;
  pub use crate::image_ai::{ImageAi, ImageAiWithControl, Placeable};
  pub use crate::tensor::{image_to_nchw, nchw_to_image};
  pub use crate::tiled::{TileAccumulator, TileConfig, TileInfo, generate_tiles};

  pub use crate::image_model::{ControlParameter, ControlParams, ControlSpec, ImageModel, ModelSpec, discover_models};
  pub use crate::progress::{Listening, ReportsProgress};

  pub use crate::onnx::{OnnxConfig, OnnxSession};
}
