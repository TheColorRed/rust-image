/// Returns the directory name of a path.
pub fn dirname(path: &str) -> String {
  let sep = std::path::MAIN_SEPARATOR.to_string();
  let mut parts = path.split(&sep).collect::<Vec<&str>>();
  parts.pop();
  parts.join(&sep)
}

/// Returns the base name of a path.
pub fn basename(path: &str) -> String {
  let sep = std::path::MAIN_SEPARATOR.to_string();

  let parts = path.split(&sep).collect::<Vec<&str>>();
  parts[parts.len() - 1].to_string()
}
