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
  pub fn with_optimization_level(mut self, p_level: GraphOptimizationLevel) -> Self {
    self.optimization_level = p_level;
    self
  }

  /// Sets the number of threads (None = auto-detect).
  pub fn with_threads(mut self, p_threads: usize) -> Self {
    self.num_threads = Some(p_threads);
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
  /// - `p_path`: Path to the ONNX model file.
  /// - `p_config`: Optional configuration (uses defaults if None).
  ///
  /// # Example
  ///
  /// ```ignore
  /// use abra_ai_core::onnx::{OnnxSession, OnnxConfig};
  ///
  /// let session = OnnxSession::from_file("model.onnx", None)?;
  /// ```
  pub fn from_file(p_path: impl AsRef<Path>, p_config: Option<OnnxConfig>) -> Result<Self, AiError> {
    let model_bytes = std::fs::read(p_path.as_ref())
      .map_err(|e| AiError::model_load_failed(format!("Failed to read model file: {}", e)))?;

    Self::from_bytes(&model_bytes, p_config)
  }

  /// Loads a model from bytes in memory.
  ///
  /// # Arguments
  ///
  /// - `p_bytes`: The ONNX model bytes.
  /// - `p_config`: Optional configuration (uses defaults if None).
  pub fn from_bytes(p_bytes: &[u8], p_config: Option<OnnxConfig>) -> Result<Self, AiError> {
    let p_config = p_config.unwrap_or_default();

    let num_threads =
      p_config.num_threads.unwrap_or_else(|| std::thread::available_parallelism().map(|p| p.get()).unwrap_or(4));

    let session = Session::builder()
      .map_err(|e| AiError::model_load_failed(format!("Failed to create session builder: {}", e)))?
      .with_optimization_level(p_config.optimization_level)
      .map_err(|e| AiError::model_load_failed(format!("Failed to set optimization level: {}", e)))?
      .with_intra_threads(num_threads)
      .map_err(|e| AiError::model_load_failed(format!("Failed to set thread count: {}", e)))?
      .commit_from_memory(p_bytes)
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
  /// - `p_input`: The input tensor data as a contiguous slice.
  /// - `p_shape`: The shape of the input tensor (e.g., `[1, 3, 256, 256]`).
  ///
  /// # Returns
  ///
  /// A tuple of (output_shape, output_data).
  pub fn run_single(&self, p_input: &[f32], p_shape: &[usize]) -> Result<(Vec<usize>, Vec<f32>), AiError> {
    self
      .run_outputs(p_input, p_shape, Some(1))?
      .into_iter()
      .next()
      .ok_or_else(|| AiError::inference_failed("The ONNX model returned no outputs"))
  }

  /// Runs inference with a single input tensor and returns every output in model order.
  ///
  /// This is useful for models, such as instance-segmentation networks, whose outputs jointly describe a prediction.
  pub fn run_multiple(&self, p_input: &[f32], p_shape: &[usize]) -> Result<Vec<(Vec<usize>, Vec<f32>)>, AiError> {
    self.run_outputs(p_input, p_shape, None)
  }

  fn run_outputs(
    &self, p_input: &[f32], p_shape: &[usize], p_limit: Option<usize>,
  ) -> Result<Vec<(Vec<usize>, Vec<f32>)>, AiError> {
    use ort::value::TensorRef;

    let input_value = TensorRef::from_array_view((p_shape, p_input))
      .map_err(|e| AiError::inference_failed(format!("Failed to create input tensor: {}", e)))?;

    let mut session =
      self.session.lock().map_err(|e| AiError::inference_failed(format!("Session lock poisoned: {}", e)))?;

    let outputs = session
      .run(ort::inputs![input_value])
      .map_err(|e| AiError::inference_failed(format!("Inference failed: {}", e)))?;

    let mut tensors = Vec::with_capacity(p_limit.unwrap_or(outputs.len()));
    for (_, output) in outputs.iter().take(p_limit.unwrap_or(outputs.len())) {
      let (out_shape, out_view) = output
        .try_extract_tensor::<f32>()
        .map_err(|e| AiError::inference_failed(format!("Failed to extract output tensor: {}", e)))?;
      tensors.push((out_shape.iter().map(|&dimension| dimension as usize).collect(), out_view.to_vec()));
    }
    Ok(tensors)
  }

  /// Runs inference with an image input and a control vector.
  ///
  /// Used for models that take both image data and control parameters.
  ///
  /// # Arguments
  ///
  /// - `p_image`: The image tensor data as a contiguous slice.
  /// - `p_image_shape`: The shape of the image tensor (e.g., `[1, 3, 256, 256]`).
  /// - `p_control`: The control vector data as a contiguous slice.
  /// - `p_control_shape`: The shape of the control tensor (e.g., `[1, 3]`).
  ///
  /// # Returns
  ///
  /// A tuple of (output_shape, output_data).
  pub fn run_with_control(
    &self, p_image: &[f32], p_image_shape: &[usize], p_control: &[f32], p_control_shape: &[usize],
  ) -> Result<(Vec<usize>, Vec<f32>), AiError> {
    use ort::value::TensorRef;

    let image_value = TensorRef::from_array_view((p_image_shape, p_image))
      .map_err(|e| AiError::inference_failed(format!("Failed to create image tensor: {}", e)))?;

    let control_value = TensorRef::from_array_view((p_control_shape, p_control))
      .map_err(|e| AiError::inference_failed(format!("Failed to create control tensor: {}", e)))?;

    let mut session =
      self.session.lock().map_err(|e| AiError::inference_failed(format!("Session lock poisoned: {}", e)))?;

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
