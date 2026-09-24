//! Error types for AI model operations.
//!
//! This module provides error types with helper constructors for common
//! failure scenarios in AI model loading and inference.

/// Errors that can occur during AI model operations.
#[derive(Debug)]
pub enum AiError {
  /// Failed to load the model file.
  ModelLoadFailed(String),
  /// Failed during inference/processing.
  InferenceFailed(String),
  /// Invalid input provided to the model.
  InvalidInput(String),
  /// The requested model was not found in the registry.
  ModelNotFound(String),
  /// Configuration error.
  ConfigError(String),
  /// Generic/other error.
  Other(String),
}

impl AiError {
  /// Creates a model load failure error.
  ///
  /// Example:
  /// - `AiError::model_load_failed("Failed to read ONNX file")`
  pub fn model_load_failed(p_msg: impl Into<String>) -> Self {
    AiError::ModelLoadFailed(p_msg.into())
  }

  /// Creates an inference failure error.
  ///
  /// Example:
  /// - `AiError::inference_failed("Tensor shape mismatch")`
  pub fn inference_failed(p_msg: impl Into<String>) -> Self {
    AiError::InferenceFailed(p_msg.into())
  }

  /// Creates an invalid input error.
  ///
  /// Example:
  /// - `AiError::invalid_input("Image dimensions too small")`
  pub fn invalid_input(p_msg: impl Into<String>) -> Self {
    AiError::InvalidInput(p_msg.into())
  }

  /// Creates a model not found error.
  ///
  /// Example:
  /// - `AiError::model_not_found("super-res-v2")`
  pub fn model_not_found(p_name: impl Into<String>) -> Self {
    AiError::ModelNotFound(p_name.into())
  }

  /// Creates a configuration error.
  ///
  /// Example:
  /// - `AiError::config_error("Invalid tile size")`
  pub fn config_error(p_msg: impl Into<String>) -> Self {
    AiError::ConfigError(p_msg.into())
  }

  /// Creates a generic error.
  ///
  /// Example:
  /// - `AiError::other("Unexpected condition")`
  pub fn other(p_msg: impl Into<String>) -> Self {
    AiError::Other(p_msg.into())
  }
}

impl std::fmt::Display for AiError {
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AiError::ModelLoadFailed(msg) => write!(p_f, "Model load failed: {}", msg),
      AiError::InferenceFailed(msg) => write!(p_f, "Inference failed: {}", msg),
      AiError::InvalidInput(msg) => write!(p_f, "Invalid input: {}", msg),
      AiError::ModelNotFound(name) => write!(p_f, "Model not found: {}", name),
      AiError::ConfigError(msg) => write!(p_f, "Configuration error: {}", msg),
      AiError::Other(msg) => write!(p_f, "Error: {}", msg),
    }
  }
}

impl std::error::Error for AiError {}
