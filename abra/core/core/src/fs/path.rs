/// Returns the directory name of a path.
pub fn dirname(path: impl Into<String>) -> String {
  let sep = std::path::MAIN_SEPARATOR.to_string();
  let path = path.into();
  let mut parts = path.split(&sep).collect::<Vec<&str>>();
  parts.pop();
  parts.join(&sep)
}

// /// Returns the base name of a path.
// pub fn basename(path: impl Into<String>) -> String {
//   let sep = std::path::MAIN_SEPARATOR.to_string();
//   let path = path.into();
//   let parts = path.split(&sep).collect::<Vec<&str>>();
//   parts[parts.len() - 1].to_string()
// }
