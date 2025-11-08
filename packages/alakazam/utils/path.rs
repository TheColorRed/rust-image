/// Get the base name of a path
/// Returns the file name+extension if it exists, otherwise an empty string
pub fn base_name(path: &str) -> String {
  let path = std::path::Path::new(path);
  match path.file_name() {
    Some(name) => name.to_str().unwrap().to_string(),
    None => "".to_string(),
  }
}
/// Get the directory of a path
/// Returns the directory of the path if it exists, otherwise an empty string
pub fn dir_name(path: &str) -> String {
  let path = std::path::Path::new(path);
  match path.parent() {
    Some(name) => name.to_str().unwrap().to_string(),
    None => "".to_string(),
  }
}
/// Get the extension of a path
/// Returns the extension of the path if it exists, otherwise an empty string
/// The extension is returned with the leading dot
/// Example: ".jpg"
pub fn extension(path: &str) -> String {
  let path = std::path::Path::new(path);
  match path.extension() {
    Some(ext) => format!(".{}", ext.to_str().unwrap()),
    None => "".to_string(),
  }
}
/// Get the file name of a path
/// Returns the file name of the file without the extension if it exists, otherwise an empty string
/// Example: "file"
pub fn file_name(path: &str) -> String {
  let path = std::path::Path::new(path);
  match path.file_stem() {
    Some(name) => name.to_str().unwrap().to_string(),
    None => "".to_string(),
  }
}
