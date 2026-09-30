/// Converts a single pattern, `&str`, `String`, or a collection of them into a `Vec<String>`.
pub trait IntoGlobPatterns {
  /// Converts `self` into a `Vec<String>` of patterns.
  fn into_patterns(self) -> Vec<String>;
}

impl IntoGlobPatterns for &str {
  fn into_patterns(self) -> Vec<String> {
    vec![self.to_string()]
  }
}

impl IntoGlobPatterns for String {
  fn into_patterns(self) -> Vec<String> {
    vec![self]
  }
}

impl<T: Into<String>> IntoGlobPatterns for Vec<T> {
  fn into_patterns(self) -> Vec<String> {
    self.into_iter().map(Into::into).collect()
  }
}

impl<T: Into<String>, const N: usize> IntoGlobPatterns for [T; N] {
  fn into_patterns(self) -> Vec<String> {
    self.into_iter().map(Into::into).collect()
  }
}

/// Expands a list of glob patterns into the file paths that match them.
/// ```ignore
/// let paths = get_paths_from_glob("assets/fonts/*.ttf");
/// let paths = get_paths_from_glob(vec!["assets/fonts/*.ttf", "assets/icons/*.svg"]);
/// ```
pub fn get_paths_from_glob(p_patterns: impl IntoGlobPatterns) -> Vec<String> {
  let mut all_paths = vec![];
  for pattern in p_patterns.into_patterns() {
    for entry in globwalk::glob(pattern.as_str()).expect("Failed to read glob pattern") {
      match entry {
        Ok(path) => all_paths.push(path.path().to_str().unwrap().to_string()),
        Err(e) => println!("Error reading path: {:?}", e),
      }
    }
  }
  all_paths
}

/// Recursively (optionally) collects file paths from a list of folders that match a predicate.
/// ```ignore
/// let paths = get_paths_from_folders(vec!["assets/fonts"], true, |ext| ext == "ttf");
/// ```
pub fn get_paths_from_folders(
  p_folders: Vec<impl Into<String>>, p_recursive: bool, p_is_supported: impl Fn(&str) -> bool + Copy,
) -> Vec<String> {
  let mut all_paths = vec![];
  for folder in p_folders {
    if let Ok(entries) = std::fs::read_dir(folder.into()) {
      for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
          let extension = path.extension().unwrap_or_default().to_str().unwrap_or("");
          if let Some(path_str) = path.to_str()
            && p_is_supported(extension)
          {
            all_paths.push(path_str.to_string());
          }
        } else if p_recursive && path.is_dir() {
          let subfolder = path.to_str().unwrap_or("");
          let sub_paths = get_paths_from_folders(vec![subfolder], true, p_is_supported);
          all_paths.extend(sub_paths);
        }
      }
    }
  }
  all_paths
}

// /// Returns the base name of a path.
// pub fn basename(path: impl Into<String>) -> String {
//   let sep = std::path::MAIN_SEPARATOR.to_string();
//   let path = path.into();
//   let parts = path.split(&sep).collect::<Vec<&str>>();
//   parts[parts.len() - 1].to_string()
// }
