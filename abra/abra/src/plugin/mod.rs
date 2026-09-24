use abra_core::Image;
use canvas::{Canvas, Layer};

// TODO: Expand plugins to different types (filters, effects, generators, etc.)
// These types will each have their own entry points and context structures.
// For now, we keep it simple with a single Plugin trait.

/// A trait that defines the interface for image processing plugins.
/// Plugins implementing this trait can be integrated into the Abra framework.
/// They can manipulate/generate images, canvases, and layers.
/// The apply method is the main entry point for plugin logic.
pub trait Plugin<'a> {
  /// The name of the plugin.
  fn name(&self) -> &str;
  /// A brief description about the plugin.
  fn description(&self) -> &str;
  /// Applies the plugin logic to the given context.
  fn apply(&mut self) -> Result<PluginResult<'a>, PluginError>;
}

/// Context passed to plugins containing the tools they can use.
pub struct PluginContext<'a> {
  /// The canvas the plugin is operating on (if any).
  pub canvas: Option<Canvas<'a>>,
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
pub struct PluginResult<'a> {
  /// The resulting canvas.
  canvases: Vec<Canvas<'a>>,
  /// The resulting images.
  images: Vec<Image>,
  /// The resulting layers.
  layers: Vec<Layer<'a>>,
}

impl<'a> PluginResult<'a> {
  /// Creates a new PluginResult with empty vectors.
  pub fn new() -> Self {
    Self {
      canvases: Vec::new(),
      images: Vec::new(),
      layers: Vec::new(),
    }
  }
  /// Checks if the PluginResult is empty (no canvases, images, or layers).
  pub fn is_empty(&self) -> bool {
    self.canvases.is_empty() && self.images.is_empty() && self.layers.is_empty()
  }
  /// Adds a canvas to the result.
  /// - `p_canvas`: The canvas to add to the result.
  pub fn add_canvas(&mut self, p_canvas: Canvas<'a>) -> &mut Self {
    self.canvases.push(p_canvas);
    self
  }
  /// Adds an image to the result.
  /// - `p_image`: The image to add to the result.
  pub fn add_image(&mut self, p_image: Image) -> &mut Self {
    self.images.push(p_image);
    self
  }
  /// Adds a layer to the result.
  /// - `p_layer`: The layer to add to the result.
  pub fn add_layer(&mut self, p_layer: Layer<'a>) -> &mut Self {
    self.layers.push(p_layer);
    self
  }
  /// Retrieves a canvas at the specified index by reference.
  /// - `p_index`: The index of the canvas to retrieve.
  pub fn canvas_at(&self, p_index: usize) -> Option<&Canvas<'a>> {
    self.canvases.get(p_index)
  }

  /// Takes (removes) a canvas from the result by index and returns it owned.
  /// This avoids lifetime-borrow issues when consuming a canvas from a `PluginResult`.
  pub fn take_canvas_at(&mut self, p_index: usize) -> Option<Canvas<'a>> {
    if p_index < self.canvases.len() { Some(self.canvases.remove(p_index)) } else { None }
  }
  /// Retrieves an image at the specified index.
  /// - `p_index`: The index of the image to retrieve.
  pub fn image_at(&self, p_index: usize) -> Option<&Image> {
    self.images.get(p_index)
  }
  /// Retrieves a layer at the specified index.
  /// - `p_index`: The index of the layer to retrieve.
  pub fn layer_at(&self, p_index: usize) -> Option<&Layer<'a>> {
    self.layers.get(p_index)
  }
}

// Use an owned `String` for error messages so callers can provide either
// a `String` or `&str` and convert to `String` easily with `.into()`.

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

impl PluginError {
  /// Helper constructor that accepts any type convertible into `String`.
  ///
  /// Example:
  /// - `PluginError::execution_failed("something went wrong")`
  /// - `PluginError::execution_failed(String::from("something"))`
  pub fn execution_failed(p_s: impl Into<String>) -> Self {
    PluginError::ExecutionFailed(p_s.into())
  }
  /// Helper constructor for invalid parameters.
  ///
  /// Example:
  /// - `PluginError::invalid_parameters("missing required field")`
  /// - `PluginError::invalid_parameters(String::from("invalid value"))`
  pub fn invalid_parameters(p_s: impl Into<String>) -> Self {
    PluginError::InvalidParameters(p_s.into())
  }
  /// Helper constructor for file not found errors.
  ///
  /// Example:
  /// - `PluginError::file_not_found("config.json not found")`
  /// - `PluginError::file_not_found(String::from("data.csv missing"))`
  pub fn file_not_found(p_s: impl Into<String>) -> Self {
    PluginError::FileNotFound(p_s.into())
  }
}
