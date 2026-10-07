//! AI models for the app: which exist, where they live, downloading them, and loading them for the Abra AI crates.
//!
//! The Abra libraries only run a model from a path they are given. This module reads the list of models from the
//! catalog in the bucket, stores models in the app's private folder, downloads them on demand and hands the libraries
//! the path. Everything about a model (title, description, URL, hash) comes from the catalog, and its size from S3. The
//! only thing the app knows is which roles it needs a model for.
//!
//! The catalog is a YAML file with one list of models per category:
//!
//! ```yaml
//! models:
//!   - role: skin-segmentation
//!     title: Body detail
//!     description: What the model does.
//!     uri: https://.../models/file.onnx
//!     sha256: <64 hex characters>
//! ```

use std::{
  fs,
  io::{Read, Write},
  path::{Path, PathBuf},
  sync::{Arc, Mutex},
};

use abra_body_segmentation::BodySegmentation;
use abra_find_person::PersonSegmenter;
use saphyr::{LoadableYamlNode, Yaml};
use sha2::{Digest, Sha256};

use crate::{
  AbraError,
  consts::{MODEL_CATALOG_URL, PERSON_DETECTION_ROLE, SKIN_SEGMENTATION_ROLE},
  image_workers,
};

/// One model listed in the catalog.
#[derive(Clone)]
struct ModelEntry {
  /// What the model is used for; identifies the model.
  role: String,
  /// Name shown to people.
  title: String,
  /// What the model does.
  description: String,
  /// Where the file is downloaded from.
  url: String,
  /// Lowercase hex SHA-256 of the file.
  sha256: String,
  /// Size of the file in bytes as reported by S3, or 0 if S3 did not say.
  size_bytes: u64,
}

/// The model list from the last catalog fetched in this run of the app.
static CATALOG: Mutex<Vec<ModelEntry>> = Mutex::new(Vec::new());
static DOWNLOAD_LOCK: Mutex<()> = Mutex::new(());
static SKIN_SEGMENTER: Mutex<Option<Arc<BodySegmentation>>> = Mutex::new(None);
static PERSON_SEGMENTER: Mutex<Option<Arc<PersonSegmenter>>> = Mutex::new(None);

/// Android: the app's private `files` folder, found from the process name (the package id).
#[cfg(target_os = "android")]
fn model_dir() -> PathBuf {
  let package = fs::read("/proc/self/cmdline")
    .ok()
    .and_then(|bytes| String::from_utf8(bytes).ok())
    .map(|name| name.trim_end_matches('\0').to_string())
    .filter(|name| !name.is_empty());
  match package {
    Some(package) => PathBuf::from("/data/data").join(package).join("files/abra-models"),
    None => std::env::temp_dir().join("abra-models"),
  }
}

/// iOS: the app sandbox's Caches folder.
#[cfg(target_os = "ios")]
fn model_dir() -> PathBuf {
  match std::env::var_os("HOME") {
    Some(home) => PathBuf::from(home).join("Library/Caches/abra-models"),
    None => std::env::temp_dir().join("abra-models"),
  }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn model_dir() -> PathBuf {
  std::env::temp_dir().join("abra-models")
}

/// Reads the `models` category of a catalog. Sizes are not in the catalog; they are asked of S3 separately.
fn parse_catalog(p_text: &str) -> Result<Vec<ModelEntry>, String> {
  let docs = Yaml::load_from_str(p_text).map_err(|e| format!("The model catalog is not valid YAML: {e}"))?;
  let items = docs
    .first()
    .and_then(|doc| doc.as_mapping_get("models")?.as_vec())
    .ok_or_else(|| "The model catalog has no `models` list".to_string())?;
  items
    .iter()
    .map(|item| {
      let text = |key: &str| {
        item
          .as_mapping_get(key)
          .and_then(Yaml::as_str)
          .map(str::to_string)
          .ok_or_else(|| format!("A model in the catalog has no `{key}`"))
      };
      let url = text("uri")?;
      let sha256 = text("sha256")?.to_lowercase();
      if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("The catalog hash for {url} is not a SHA-256"));
      }
      Ok(ModelEntry {
        role: text("role")?,
        title: text("title")?,
        description: item.as_mapping_get("description").and_then(Yaml::as_str).unwrap_or("").to_string(),
        sha256,
        size_bytes: 0,
        url,
      })
    })
    .collect()
}

/// Downloads the catalog from S3, asks S3 for each file's size and replaces the model list with the result.
fn fetch_catalog() -> Result<Vec<ModelEntry>, String> {
  let mut response = ureq::get(MODEL_CATALOG_URL).call().map_err(|e| format!("Failed to download the model catalog: {e}"))?;
  let text = response.body_mut().read_to_string().map_err(|e| format!("Failed to read the model catalog: {e}"))?;
  let mut models = parse_catalog(&text)?;
  for model in &mut models {
    model.size_bytes = ureq::head(&model.url)
      .call()
      .ok()
      .and_then(|response| response.headers().get("content-length")?.to_str().ok()?.parse().ok())
      .unwrap_or(0);
  }
  *CATALOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = models.clone();
  Ok(models)
}

fn find_model(p_role: &str) -> Result<ModelEntry, AbraError> {
  let catalog = CATALOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  catalog.iter().find(|model| model.role == p_role).cloned().ok_or_else(|| AbraError::Ai {
    message: format!("The model for {p_role} is not in the model list; refresh it first"),
  })
}

/// Every downloaded file for `p_role`, found by name so it works without the network. Models are stored as
/// `<role>-<12 hash characters>.onnx`.
fn downloaded_files(p_role: &str) -> Vec<fs::DirEntry> {
  let prefix = format!("{p_role}-");
  fs::read_dir(model_dir())
    .into_iter()
    .flatten()
    .filter_map(Result::ok)
    .filter(|entry| {
      let name = entry.file_name().to_string_lossy().into_owned();
      name.len() == prefix.len() + 12 + ".onnx".len() && name.starts_with(&prefix) && name.ends_with(".onnx")
    })
    .collect()
}

/// The downloaded file for `p_role`. When several versions exist the newest wins.
fn find_downloaded(p_role: &str) -> Result<PathBuf, AbraError> {
  downloaded_files(p_role)
    .into_iter()
    .max_by_key(|entry| entry.metadata().and_then(|metadata| metadata.modified()).ok())
    .map(|entry| entry.path())
    .ok_or_else(|| AbraError::Ai {
      message: format!("The model for {p_role} is not downloaded"),
    })
}

impl ModelEntry {
  /// Where the model is stored on this device. The name includes the start of the hash, so a new version of a model
  /// never collides with an old one.
  fn local_path(&self) -> PathBuf {
    model_dir().join(format!("{}-{}.onnx", self.role, &self.sha256[..12]))
  }

  /// Size of the downloaded file, or `None` when it is not downloaded. Files only get their final name after passing
  /// the hash check, so existing means valid.
  fn size_on_disk(&self) -> Option<u64> {
    fs::metadata(self.local_path()).ok().filter(|metadata| metadata.is_file()).map(|metadata| metadata.len())
  }

  /// Returns the local path of the model, downloading it first if needed.
  fn ensure(&self) -> Result<PathBuf, AbraError> {
    let _guard = DOWNLOAD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = self.local_path();
    if path.exists() {
      match hash_file(&path) {
        Ok(hash) if hash == self.sha256 => return Ok(path),
        _ => {
          let _ = fs::remove_file(&path);
        }
      }
    }

    let partial = path.with_extension("part");
    let result = fs::create_dir_all(model_dir())
      .map_err(|e| format!("Cannot create the model folder: {e}"))
      .and_then(|_| self.download(&partial))
      .and_then(|_| fs::rename(&partial, &path).map_err(|e| format!("Cannot save {}: {e}", self.title)));
    match result {
      Ok(()) => {
        // Older versions of this model are no longer used.
        for entry in downloaded_files(&self.role) {
          if entry.path() != path {
            let _ = fs::remove_file(entry.path());
          }
        }
        Ok(path)
      }
      Err(message) => {
        let _ = fs::remove_file(&partial);
        Err(AbraError::Ai { message })
      }
    }
  }

  /// Downloads the file to `p_partial` and checks its hash.
  fn download(&self, p_partial: &Path) -> Result<(), String> {
    let response = ureq::get(&self.url).call().map_err(|e| format!("Failed to download {}: {e}", self.title))?;
    let mut reader = response.into_body().into_reader();
    let mut file = fs::File::create(p_partial).map_err(|e| format!("Cannot create {p_partial:?}: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
      let read = reader.read(&mut buffer).map_err(|e| format!("Download of {} was interrupted: {e}", self.title))?;
      if read == 0 {
        break;
      }
      hasher.update(&buffer[..read]);
      file.write_all(&buffer[..read]).map_err(|e| format!("Cannot write {}: {e}", self.title))?;
    }
    file.flush().map_err(|e| format!("Cannot write {}: {e}", self.title))?;

    let hash = hex(&hasher.finalize());
    if hash != self.sha256 {
      return Err(format!("Downloaded {} does not match its hash (got {hash})", self.title));
    }
    Ok(())
  }
}

fn hash_file(p_path: &Path) -> std::io::Result<String> {
  let mut file = fs::File::open(p_path)?;
  let mut hasher = Sha256::new();
  let mut buffer = [0u8; 64 * 1024];
  loop {
    let read = file.read(&mut buffer)?;
    if read == 0 {
      break;
    }
    hasher.update(&buffer[..read]);
  }
  Ok(hex(&hasher.finalize()))
}

fn hex(p_bytes: &[u8]) -> String {
  p_bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The skin segmenter, loaded on first use from the downloaded model. Fails if the model is not downloaded. A failure
/// is not cached, so the next call tries again.
pub(crate) fn skin_segmenter() -> Result<Arc<BodySegmentation>, AbraError> {
  let mut cached = SKIN_SEGMENTER.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  if let Some(segmenter) = cached.as_ref() {
    return Ok(Arc::clone(segmenter));
  }
  let segmenter = Arc::new(BodySegmentation::load(find_downloaded(SKIN_SEGMENTATION_ROLE)?).map_err(|error| AbraError::Ai {
    message: error.to_string(),
  })?);
  *cached = Some(Arc::clone(&segmenter));
  Ok(segmenter)
}

/// The person segmenter, loaded on first use from the downloaded model. Fails if the model is not downloaded. A failure
/// is not cached.
pub(crate) fn person_segmenter() -> Result<Arc<PersonSegmenter>, AbraError> {
  let mut cached = PERSON_SEGMENTER.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  if let Some(segmenter) = cached.as_ref() {
    return Ok(Arc::clone(segmenter));
  }
  let segmenter = Arc::new(PersonSegmenter::load(find_downloaded(PERSON_DETECTION_ROLE)?).map_err(|error| AbraError::Ai {
    message: error.to_string(),
  })?);
  *cached = Some(Arc::clone(&segmenter));
  Ok(segmenter)
}

/// One AI model and whether it is downloaded to this device.
#[derive(Clone, Debug, uniffi::Record)]
pub struct AiModel {
  /// What the model is used for. Identifies the model in `download_ai_model` and `delete_ai_model`.
  pub id: String,
  /// Name shown to people.
  pub title: String,
  /// What the model does.
  pub description: String,
  /// Whether the model file is on this device.
  pub downloaded: bool,
  /// Size of the downloaded file in bytes, or 0 when it is not downloaded.
  pub size_bytes: u64,
  /// Size of the file to download in bytes, as reported by S3.
  pub download_size_bytes: u64,
}

/// Whether the model that finds people in a photo is on this device. Works offline.
#[uniffi::export]
pub fn person_detection_downloaded() -> bool {
  find_downloaded(PERSON_DETECTION_ROLE).is_ok()
}

fn describe_models(p_models: Vec<ModelEntry>) -> Vec<AiModel> {
  p_models
    .into_iter()
    .map(|model| {
      let size = model.size_on_disk();
      AiModel {
        downloaded: size.is_some(),
        size_bytes: size.unwrap_or(0),
        download_size_bytes: model.size_bytes,
        id: model.role,
        title: model.title,
        description: model.description,
      }
    })
    .collect()
}

/// Lists the AI models from the last catalog fetched in this run of the app. Empty until `refresh_ai_models` succeeds.
#[uniffi::export]
pub fn ai_models() -> Vec<AiModel> {
  describe_models(CATALOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone())
}

/// Fetches the model catalog from S3 and lists the AI models. Fails when S3 can't be reached.
#[uniffi::export(async_runtime = "tokio")]
pub async fn refresh_ai_models() -> Result<Vec<AiModel>, AbraError> {
  image_workers::run(|| fetch_catalog().map(describe_models).map_err(|message| AbraError::Ai { message })).await
}

/// Downloads an AI model unless it is already on this device.
#[uniffi::export(async_runtime = "tokio")]
pub async fn download_ai_model(id: String) -> Result<(), AbraError> {
  let model = find_model(&id)?;
  image_workers::run(move || model.ensure().map(|_| ())).await
}

/// Deletes a downloaded AI model. It downloads again the next time a feature needs it. A model already loaded in
/// memory keeps working until the app restarts.
#[uniffi::export]
pub fn delete_ai_model(id: String) -> Result<(), AbraError> {
  let model = find_model(&id)?;
  let _guard = DOWNLOAD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  match fs::remove_file(model.local_path()) {
    Ok(()) => Ok(()),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(e) => Err(AbraError::Ai {
      message: format!("Cannot delete {}: {e}", model.title),
    }),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const CATALOG_TEXT: &str = "models:
  - role: skin-segmentation
    title: Body detail
    description: Maps a body.
    uri: https://example.com/models/a.onnx
    sha256: 35ec1ecd9ee7f85073c99c00020b7f6751b69506eeacf683bc8665f6117f85b0
";

  #[test]
  fn parses_the_models_category() {
    let models = parse_catalog(CATALOG_TEXT).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].role, "skin-segmentation");
    assert_eq!(models[0].title, "Body detail");
  }

  #[test]
  fn rejects_a_catalog_with_a_bad_model() {
    assert!(parse_catalog(&CATALOG_TEXT.replace("35ec1ecd", "zzzzzzzz")).is_err());
    assert!(parse_catalog(&CATALOG_TEXT.replace("    sha256", "    other")).is_err());
    assert!(parse_catalog("other: []").is_err());
  }
}
