//! ONNX Runtime session utilities.
//!
//! This module provides a wrapper around ONNX Runtime sessions with
//! sensible defaults for image processing models.

use crate::error::AiError;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use std::path::Path;
use std::sync::Mutex;

/// Configuration for ONNX session creation.
#[derive(Debug)]
pub struct OnnxConfig {
  /// Graph optimization level (default: Level3 for maximum optimization).
  pub optimization_level: GraphOptimizationLevel,
  /// Number of threads for intra-op parallelism (default: auto-detect).
  pub num_threads: Option<usize>,
}

impl Default for OnnxConfig {
  fn default() -> Self {
    Self {
      optimization_level: GraphOptimizationLevel::Level3,
      num_threads: None,
    }
  }
}

impl OnnxConfig {
  /// Creates a new config with default settings.
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the optimization level.
  pub fn with_optimization_level(mut self, level: GraphOptimizationLevel) -> Self {
    self.optimization_level = level;
    self
  }

  /// Sets the number of threads (None = auto-detect).
  pub fn with_threads(mut self, threads: usize) -> Self {
    self.num_threads = Some(threads);
    self
  }
}

/// A thread-safe wrapper around an ONNX Runtime session.
///
/// Provides convenient methods for loading models and running inference
/// with sensible defaults for image processing workloads.
pub struct OnnxSession {
  session: Mutex<Session>,
  num_threads: usize,
}

impl OnnxSession {
  /// Loads a model from a file path.
  ///
  /// # Arguments
  ///
  /// - `path`: Path to the ONNX model file.
  /// - `config`: Optional configuration (uses defaults if None).
  ///
  /// # Example
  ///
  /// ```ignore
  /// use abra_ai_core::onnx::{OnnxSession, OnnxConfig};
  ///
  /// let session = OnnxSession::from_file("model.onnx", None)?;
  /// ```
  pub fn from_file(path: impl AsRef<Path>, config: Option<OnnxConfig>) -> Result<Self, AiError> {
    let model_bytes = std::fs::read(path.as_ref())
      .map_err(|e| AiError::model_load_failed(format!("Failed to read model file: {}", e)))?;

    Self::from_bytes(&model_bytes, config)
  }

  /// Loads a model from bytes in memory.
  ///
  /// # Arguments
  ///
  /// - `bytes`: The ONNX model bytes.
  /// - `config`: Optional configuration (uses defaults if None).
  pub fn from_bytes(bytes: &[u8], config: Option<OnnxConfig>) -> Result<Self, AiError> {
    let config = config.unwrap_or_default();

    let num_threads = config
      .num_threads
      .unwrap_or_else(|| std::thread::available_parallelism().map(|p| p.get()).unwrap_or(4));

    let session = Session::builder()
      .map_err(|e| AiError::model_load_failed(format!("Failed to create session builder: {}", e)))?
      .with_optimization_level(config.optimization_level)
      .map_err(|e| AiError::model_load_failed(format!("Failed to set optimization level: {}", e)))?
      .with_intra_threads(num_threads)
      .map_err(|e| AiError::model_load_failed(format!("Failed to set thread count: {}", e)))?
      .commit_from_memory(bytes)
      .map_err(|e| AiError::model_load_failed(format!("Failed to load ONNX model: {}", e)))?;

    Ok(Self {
      session: Mutex::new(session),
      num_threads,
    })
  }

  /// Returns the number of threads used for inference.
  pub fn num_threads(&self) -> usize {
    self.num_threads
  }

  /// Runs inference with a single input tensor and returns the first output.
  ///
  /// # Arguments
  ///
  /// - `input`: The input tensor data as a contiguous slice.
  /// - `shape`: The shape of the input tensor (e.g., `[1, 3, 256, 256]`).
  ///
  /// # Returns
  ///
  /// A tuple of (output_shape, output_data).
  pub fn run_single(&self, input: &[f32], shape: &[usize]) -> Result<(Vec<usize>, Vec<f32>), AiError> {
    use ort::value::TensorRef;

    let input_value = TensorRef::from_array_view((shape, input))
      .map_err(|e| AiError::inference_failed(format!("Failed to create input tensor: {}", e)))?;

    let mut session = self
      .session
      .lock()
      .map_err(|e| AiError::inference_failed(format!("Session lock poisoned: {}", e)))?;

    let outputs = session
      .run(ort::inputs![input_value])
      .map_err(|e| AiError::inference_failed(format!("Inference failed: {}", e)))?;

    // Get first output using index
    let output = &outputs[0];

    let (out_shape, out_view) = output
      .try_extract_tensor::<f32>()
      .map_err(|e| AiError::inference_failed(format!("Failed to extract output tensor: {}", e)))?;

    let out_shape_vec: Vec<usize> = out_shape.iter().map(|&d| d as usize).collect();
    let out_data: Vec<f32> = out_view.iter().copied().collect();

    Ok((out_shape_vec, out_data))
  }

  /// Runs inference with an image input and a control vector.
  ///
  /// Used for models like UltraZoom that take both image data and control parameters.
  ///
  /// # Arguments
  ///
  /// - `image`: The image tensor data as a contiguous slice.
  /// - `image_shape`: The shape of the image tensor (e.g., `[1, 3, 256, 256]`).
  /// - `control`: The control vector data as a contiguous slice.
  /// - `control_shape`: The shape of the control tensor (e.g., `[1, 3]`).
  ///
  /// # Returns
  ///
  /// A tuple of (output_shape, output_data).
  pub fn run_with_control(
    &self, image: &[f32], image_shape: &[usize], control: &[f32], control_shape: &[usize],
  ) -> Result<(Vec<usize>, Vec<f32>), AiError> {
    use ort::value::TensorRef;

    let image_value = TensorRef::from_array_view((image_shape, image))
      .map_err(|e| AiError::inference_failed(format!("Failed to create image tensor: {}", e)))?;

    let control_value = TensorRef::from_array_view((control_shape, control))
      .map_err(|e| AiError::inference_failed(format!("Failed to create control tensor: {}", e)))?;

    let mut session = self
      .session
      .lock()
      .map_err(|e| AiError::inference_failed(format!("Session lock poisoned: {}", e)))?;

    let outputs = session
      .run(ort::inputs![image_value, control_value])
      .map_err(|e| AiError::inference_failed(format!("Inference failed: {}", e)))?;

    // Get first output using index
    let output = &outputs[0];

    let (out_shape, out_view) = output
      .try_extract_tensor::<f32>()
      .map_err(|e| AiError::inference_failed(format!("Failed to extract output tensor: {}", e)))?;

    let out_shape_vec: Vec<usize> = out_shape.iter().map(|&d| d as usize).collect();
    let out_data: Vec<f32> = out_view.iter().copied().collect();

    Ok((out_shape_vec, out_data))
  }
}
