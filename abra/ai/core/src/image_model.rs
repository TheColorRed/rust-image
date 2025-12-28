//! Unified Image Model API
//!
//! This module provides a single `ImageModel` struct that can load and run
//! any ONNX model for image processing. Models are self-describing via YAML
//! manifest files placed alongside the `.onnx` files.
//!
//! # Model Manifest Format
//!
//! Each model should have a `.yml` file with the same base name as the `.onnx`:
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
//! // Load a model by name from the default models directory.
//! // Add the directory to `settings.yml` if needed.
//! let model = ImageModel::load_by_name("Example-Model")?;
//!
//! // Or load directly from a file path if you have one.
//! let model = ImageModel::load("models/Example-Model.onnx")?;
//!
//! // Process without control (uses defaults if model has control input)
//! let result = model.process(&image)?;
//!
//! // Or with custom control parameters
//! let ctrl = ControlParams::new(&[0.4, 0.2, 0.8]);
//! let result = model.process_with_control(&image, &ctrl)?;
//! ```

use crate::error::AiError;
use crate::onnx::OnnxSession;
use crate::tensor::image_to_nchw;
use crate::tiled::{TileAccumulator, TileConfig, generate_tiles};
use abra_core::Image;
use abra_core::transform::cropped;
use saphyr::{LoadableYamlNode, Yaml};
use std::path::Path;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Model Specification (loaded from YAML manifest)
// ---------------------------------------------------------------------------

/// Specification for a model's capabilities and configuration.
///
/// This is typically loaded from a YAML manifest file alongside the ONNX model.
#[derive(Clone, Debug)]
pub struct ModelSpec {
  /// Path to the ONNX model file.
  pub path: String,
  /// Human-readable name for the model.
  pub name: String,
  /// Description of what the model does.
  pub description: String,
  /// Output scale factor (1.0 = same size, 2.0 = 2x upscale).
  pub scale_factor: f32,
  /// Tile configuration for processing.
  pub tile_config: TileConfig,
  /// Control input configuration (None if model doesn't use control).
  pub control: Option<ControlSpec>,
}

/// Specification for control input parameters.
#[derive(Clone, Debug)]
pub struct ControlSpec {
  /// Number of control parameters.
  pub size: usize,
  /// Default values for each parameter.
  pub defaults: Vec<f32>,
  /// Parameter descriptions (optional).
  pub parameters: Vec<ControlParameter>,
}

/// Description of a single control parameter.
#[derive(Clone, Debug)]
pub struct ControlParameter {
  /// Parameter name.
  pub name: String,
  /// Parameter description.
  pub description: String,
  /// Index in the control vector.
  pub index: usize,
}

impl ModelSpec {
  /// Loads a model specification from a YAML manifest file.
  ///
  /// The manifest should be alongside the ONNX file with the same base name
  /// and a `.yml` extension.
  pub fn load(onnx_path: impl AsRef<Path>) -> Result<Self, AiError> {
    let onnx_path = onnx_path.as_ref();
    let yaml_path = onnx_path.with_extension("yml");

    if !yaml_path.exists() {
      return Err(AiError::model_load_failed(format!("Model manifest not found: {}", yaml_path.display())));
    }

    let yaml_str = std::fs::read_to_string(&yaml_path)
      .map_err(|e| AiError::model_load_failed(format!("Failed to read manifest: {}", e)))?;

    let docs =
      Yaml::load_from_str(&yaml_str).map_err(|e| AiError::model_load_failed(format!("Invalid YAML: {}", e)))?;

    let doc = docs
      .first()
      .ok_or_else(|| AiError::model_load_failed("Empty YAML document"))?;

    Self::from_yaml(doc, onnx_path.to_string_lossy().to_string())
  }

  /// Creates a ModelSpec from a parsed YAML document.
  fn from_yaml(doc: &Yaml, onnx_path: String) -> Result<Self, AiError> {
    let name = doc["name"].as_str().unwrap_or("Unknown Model").to_string();
    let description = doc["description"].as_str().unwrap_or("No description").to_string();

    let params = if doc["params"].is_badvalue() || doc["params"].is_null() {
      doc
    } else {
      &doc["params"]
    };

    let scale_factor = params["scale_factor"]
      .as_floating_point()
      .map(|f| f as f32)
      .unwrap_or(1.0);

    let tile_size = params["tile_size"].as_integer().map(|i| i as u32).unwrap_or(256);
    let tile_overlap = params["tile_overlap"].as_integer().map(|i| i as u32).unwrap_or(32);

    // Parse control specification if present
    let control = if doc["control"].is_badvalue() || doc["control"].is_null() {
      None
    } else {
      let ctrl = &doc["control"];
      let size = ctrl["size"].as_integer().unwrap_or(0) as usize;

      let defaults: Vec<f32> = ctrl["defaults"]
        .as_vec()
        .map(|v| {
          v.iter()
            .filter_map(|x| x.as_floating_point().map(|f| f as f32))
            .collect()
        })
        .unwrap_or_else(|| vec![0.5; size]);

      let parameters: Vec<ControlParameter> = ctrl["parameters"]
        .as_vec()
        .map(|v| {
          v.iter()
            .filter_map(|p| {
              Some(ControlParameter {
                name: p["name"].as_str()?.to_string(),
                description: p["description"].as_str().unwrap_or("").to_string(),
                index: p["index"].as_integer()? as usize,
              })
            })
            .collect()
        })
        .unwrap_or_default();

      if size > 0 {
        Some(ControlSpec {
          size,
          defaults,
          parameters,
        })
      } else {
        None
      }
    };

    Ok(Self {
      path: onnx_path,
      name,
      description,
      scale_factor,
      tile_config: TileConfig::new(tile_size, tile_overlap),
      control,
    })
  }

  /// Creates a minimal spec for a model without a manifest.
  ///
  /// Use this as a fallback when no YAML manifest exists.
  pub fn minimal(path: impl Into<String>) -> Self {
    Self {
      path: path.into(),
      name: "Unknown Model".into(),
      description: "No manifest found".into(),
      scale_factor: 1.0,
      tile_config: TileConfig::new(256, 32),
      control: None,
    }
  }

  /// Returns whether this model accepts control parameters.
  pub fn has_control(&self) -> bool {
    self.control.is_some()
  }

  /// Returns the number of control parameters, or 0 if none.
  pub fn control_size(&self) -> usize {
    self.control.as_ref().map(|c| c.size).unwrap_or(0)
  }

  /// Returns the default control parameters for this model.
  pub fn default_control(&self) -> Option<ControlParams> {
    self.control.as_ref().map(|c| ControlParams::new(&c.defaults))
  }
}

// ---------------------------------------------------------------------------
// Control Parameters
// ---------------------------------------------------------------------------

/// Control parameters for models that support adjustable processing.
///
/// The meaning of each parameter depends on the model. Check the model's
/// YAML manifest for parameter descriptions.
#[derive(Clone, Debug)]
pub struct ControlParams {
  params: Vec<f32>,
}

impl Default for ControlParams {
  fn default() -> Self {
    Self {
      params: vec![0.5, 0.5, 0.5],
    }
  }
}

impl ControlParams {
  /// Creates control parameters from a slice of values.
  ///
  /// Values are clamped to the 0.0-1.0 range.
  pub fn new(params: &[f32]) -> Self {
    Self {
      params: params.iter().map(|v| v.clamp(0.0, 1.0)).collect(),
    }
  }

  /// Returns the parameters as a slice.
  pub fn as_slice(&self) -> &[f32] {
    &self.params
  }

  /// Returns the number of parameters.
  pub fn len(&self) -> usize {
    self.params.len()
  }

  /// Returns true if there are no parameters.
  pub fn is_empty(&self) -> bool {
    self.params.is_empty()
  }

  /// Sets a parameter by index.
  pub fn set(&mut self, index: usize, value: f32) {
    if index < self.params.len() {
      self.params[index] = value.clamp(0.0, 1.0);
    }
  }

  /// Gets a parameter by index.
  pub fn get(&self, index: usize) -> Option<f32> {
    self.params.get(index).copied()
  }
}

// ---------------------------------------------------------------------------
// Unified Image Model
// ---------------------------------------------------------------------------

/// Unified image processing model.
///
/// This struct provides a single interface for loading and running any
/// ONNX model for image processing. Models describe themselves via YAML
/// manifest files, so no model-specific code is needed.
///
/// # Example
///
/// ```ignore
/// use abra_ai_core::prelude::*;
///
/// // Load model (reads .yml manifest automatically)
/// let model = ImageModel::load("models/my-model.onnx")?;
///
/// // Process an image
/// let output = model.process(&image)?;
///
/// // Or with control parameters
/// let ctrl = ControlParams::new(&[0.3, 0.5, 0.7]);
/// let output = model.process_with_control(&image, &ctrl)?;
/// ```
pub struct ImageModel {
  session: Option<OnnxSession>,
  spec: Option<ModelSpec>,
  available_models: Vec<ModelSpec>,
}

impl ImageModel {
  pub fn new(available_models: impl Into<Option<Vec<ModelSpec>>>) -> ImageModel {
    ImageModel {
      session: None,
      spec: None,
      available_models: available_models.into().unwrap_or_default(),
    }
  }
  /// Loads an image model from an ONNX file.
  ///
  /// This automatically looks for a `.yml` manifest file with the same
  /// base name to load the model specification.
  ///
  /// # Arguments
  ///
  /// - `p_path`: Path to the ONNX model file.
  pub fn load(&self, p_path: impl AsRef<Path>) -> Result<Self, AiError> {
    let path = p_path.as_ref();
    let spec = ModelSpec::load(path)?;
    Self::from_spec(spec)
  }
  /// Loads an image model from an ONNX file by name.
  ///
  /// Searches the default models directory for a model with the given name.
  ///
  /// # Arguments
  ///
  /// - `p_name`: The name of the model to load.
  pub fn load_by_name(&self, p_name: impl AsRef<str>) -> Result<Self, AiError> {
    let name = p_name.as_ref();
    let spec = self
      .available_models
      .iter()
      .find(|m| &m.name == name)
      .ok_or_else(|| AiError::model_load_failed(format!("Model not found: {}", name)))?;
    Self::from_spec(spec.clone())
  }
  /// Loads an image model from an ONNX file without requiring a manifest.
  ///
  /// Uses minimal defaults for the specification.
  pub fn load_minimal(path: impl AsRef<Path>) -> Result<Self, AiError> {
    let path = path.as_ref();
    let spec = ModelSpec::minimal(path.to_string_lossy());
    Self::from_spec(spec)
  }
  /// Creates an image model from a pre-built specification.
  pub fn from_spec(spec: ModelSpec) -> Result<Self, AiError> {
    let session = OnnxSession::from_file(&spec.path, None)?;

    println!("✅ Loaded {} (CPU, {} threads)", spec.name, session.num_threads());
    println!("   {}", spec.description);
    println!("   Scale: {}x", spec.scale_factor);
    if let Some(ctrl) = &spec.control {
      println!("   Control: {} parameters", ctrl.size);
      for param in &ctrl.parameters {
        println!("     - {}: {}", param.name, param.description);
      }
    }

    Ok(Self {
      session: Some(session),
      spec: Some(spec),
      available_models: Vec::new(),
    })
  }

  /// Returns the model specification.
  pub fn spec(&self) -> &ModelSpec {
    self.spec.as_ref().expect("Model spec is not loaded")
  }

  /// Returns the model name.
  pub fn name(&self) -> &str {
    &self.spec.as_ref().expect("Model spec is not loaded").name
  }

  /// Returns the output scale factor.
  pub fn scale_factor(&self) -> f32 {
    self.spec.as_ref().expect("Model spec is not loaded").scale_factor
  }

  /// Returns whether the model supports control parameters.
  pub fn has_control(&self) -> bool {
    self.spec.as_ref().expect("Model spec is not loaded").has_control()
  }

  /// Returns the default control parameters for this model.
  pub fn default_control(&self) -> Option<ControlParams> {
    self.spec.as_ref().expect("Model spec is not loaded").default_control()
  }

  /// Processes an image.
  ///
  /// If the model has control input, uses the default control values.
  pub fn process(&self, input: &Image) -> Result<Image, AiError> {
    let control = self.spec.as_ref().expect("Model spec is not loaded").default_control();
    self.process_tiles(input, control.as_ref())
  }

  /// Processes an image with custom control parameters.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The model doesn't support control input
  /// - The control parameter count doesn't match the model's expectations
  pub fn process_with_control(&self, input: &Image, control: &ControlParams) -> Result<Image, AiError> {
    if !self.spec.as_ref().expect("Model spec is not loaded").has_control() {
      return Err(AiError::inference_failed(format!(
        "Model '{}' does not support control parameters",
        self.spec.as_ref().expect("Model spec is not loaded").name
      )));
    }

    let expected = self.spec.as_ref().expect("Model spec is not loaded").control_size();
    if control.len() != expected {
      return Err(AiError::inference_failed(format!(
        "Model '{}' expects {} control parameters, got {}",
        self.spec.as_ref().expect("Model spec is not loaded").name,
        expected,
        control.len()
      )));
    }

    self.process_tiles(input, Some(control))
  }

  /// Internal method to process image in tiles.
  fn process_tiles(&self, input: &Image, control: Option<&ControlParams>) -> Result<Image, AiError> {
    let start = Instant::now();
    let (orig_w, orig_h) = input.dimensions::<u32>();
    let scale = self.spec.as_ref().expect("Model spec is not loaded").scale_factor;

    let out_w = (orig_w as f32 * scale) as u32;
    let out_h = (orig_h as f32 * scale) as u32;

    println!(
      "Processing {}x{} -> {}x{} with {}",
      orig_w,
      orig_h,
      out_w,
      out_h,
      self.spec.as_ref().expect("Model spec is not loaded").name
    );

    if let Some(ctrl) = control {
      println!("  Control: {:?}", ctrl.as_slice());
    }

    let tile_config = &self.spec.as_ref().expect("Model spec is not loaded").tile_config;
    let tiles = generate_tiles(orig_w, orig_h, tile_config);
    println!("  Tiles: {} (size={}, overlap={})", tiles.len(), tile_config.tile_size, tile_config.overlap);

    let mut accumulator = TileAccumulator::new(out_w, out_h);

    for tile_info in &tiles {
      if tile_info.index % 10 == 0 || tile_info.index == tile_info.total - 1 {
        println!("  Tile {}/{}", tile_info.index + 1, tile_info.total);
      }

      // Crop tile from input
      let tile_image = cropped(input, tile_info.x, tile_info.y, tile_info.width, tile_info.height);

      // Convert to tensor
      let tensor = image_to_nchw(&tile_image);
      let tensor_data = tensor.as_standard_layout();
      let image_slice = tensor_data
        .as_slice()
        .ok_or_else(|| AiError::inference_failed("Failed to get tensor slice"))?;

      let image_shape = [1, 3, tile_info.height as usize, tile_info.width as usize];

      // Run inference
      let (out_shape, out_data) = if let Some(ctrl) = control {
        let ctrl_shape = [ctrl.len()];
        self
          .session
          .as_ref()
          .expect("ONNX session is not loaded")
          .run_with_control(image_slice, &image_shape, ctrl.as_slice(), &ctrl_shape)?
      } else {
        self
          .session
          .as_ref()
          .expect("ONNX session is not loaded")
          .run_single(image_slice, &image_shape)?
      };

      // Accumulate output
      let out_tile_h = out_shape
        .get(2)
        .copied()
        .unwrap_or((tile_info.height as f32 * scale) as usize) as u32;
      let out_tile_w = out_shape
        .get(3)
        .copied()
        .unwrap_or((tile_info.width as f32 * scale) as usize) as u32;

      let out_x = (tile_info.x as f32 * scale) as u32;
      let out_y = (tile_info.y as f32 * scale) as u32;

      accumulator.accumulate(out_x, out_y, out_tile_w, out_tile_h, &out_data);
    }

    println!("Complete in {:?}", start.elapsed());

    Ok(accumulator.finalize())
  }
}

// ---------------------------------------------------------------------------
// Model Discovery
// ---------------------------------------------------------------------------

/// Discovers all models in a directory.
///
/// Scans for `.onnx` files with accompanying `.yml` manifests.
pub fn discover_models(dir: impl AsRef<Path>) -> Result<Vec<ModelSpec>, AiError> {
  let dir = dir.as_ref();
  let mut models = Vec::new();

  let entries =
    std::fs::read_dir(dir).map_err(|e| AiError::model_load_failed(format!("Failed to read directory: {}", e)))?;

  for entry in entries.flatten() {
    let path = entry.path();
    if path.extension().map(|e| e == "onnx").unwrap_or(false) {
      // Check if manifest exists
      let manifest = path.with_extension("yml");
      if manifest.exists() {
        match ModelSpec::load(&path) {
          Ok(spec) => models.push(spec),
          Err(e) => eprintln!("Warning: Failed to load {}: {}", path.display(), e),
        }
      }
    }
  }

  Ok(models)
}
