/// Why a plugin could not be read.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginError {
  /// The plugin's `plugin.yml` is not valid. The message says what is wrong with it.
  InvalidManifest(String),
  /// A file the plugin names is not in its folder.
  FileNotFound(String),
  /// The folder or a file in it could not be read.
  Io(String),
}

impl PluginError {
  /// An invalid manifest.
  /// - `p_message`: What is wrong with it.
  pub fn invalid_manifest(p_message: impl Into<String>) -> Self {
    PluginError::InvalidManifest(p_message.into())
  }

  /// A file the plugin names that is not there.
  /// - `p_path`: The path as the plugin wrote it, relative to its folder.
  pub fn file_not_found(p_path: impl Into<String>) -> Self {
    PluginError::FileNotFound(p_path.into())
  }

  /// A folder or file that could not be read.
  /// - `p_message`: What went wrong.
  pub fn io(p_message: impl Into<String>) -> Self {
    PluginError::Io(p_message.into())
  }
}

impl std::fmt::Display for PluginError {
  fn fmt(&self, p_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      PluginError::InvalidManifest(message) => write!(p_formatter, "Invalid plugin manifest: {message}"),
      PluginError::FileNotFound(path) => write!(p_formatter, "The plugin names a file that is not there: {path}"),
      PluginError::Io(message) => write!(p_formatter, "Could not read the plugin: {message}"),
    }
  }
}

impl std::error::Error for PluginError {}
