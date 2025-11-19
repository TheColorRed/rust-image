use core::Image;
use image::Canvas;

/// Trait that all plugins must implement.
pub trait Plugin {
  /// The name of the plugin.
  fn name(&self) -> &str;
  /// A brief description about the plugin.
  fn description(&self) -> &str;
  /// Applies the plugin logic to the given context.
  fn apply(&mut self) -> Result<PluginResult, PluginError>;
}

/// Context passed to plugins containing the tools they can use.
pub struct PluginContext {
  /// The canvas the plugin is operating on (if any).
  pub canvas: Option<Canvas>,
  /// The images the plugin can access.
  pub images: Vec<Image>,
  /// The parameters passed to the plugin.
  pub parameters: PluginParameters,
}

/// Parameters that plugins can accept (key-value map).
pub type PluginParameters = std::collections::HashMap<String, ParameterValue>;

#[derive(Clone, Debug)]
/// Different types of parameter values that can be passed to plugins.
pub enum ParameterValue {
  /// A string value.
  String(String),
  /// An integer value.
  Integer(i32),
  /// A float value.
  Float(f32),
  /// A boolean value.
  Bool(bool),
  /// A file path.
  Path(String),
}

/// Output from a plugin execution.
pub enum PluginResult {
  /// The resulting canvas.
  Canvases(Vec<Canvas>),
  /// The resulting images.
  Images(Vec<Image>),
}

#[derive(Debug)]
/// Errors that can occur during plugin execution.
pub enum PluginError {
  /// The plugin failed to execute.
  ExecutionFailed(String),
  /// The parameters provided to the plugin were invalid.
  InvalidParameters(String),
  /// A required file was not found.
  FileNotFound(String),
}
