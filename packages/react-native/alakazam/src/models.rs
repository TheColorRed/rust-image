//! AI models for the app: where they live on this device, downloading them, and loading them for the Abra AI crates.
//!
//! The Abra libraries only run a model from a path they are given. This module stores models in the app's private folder,
//! downloads a model when it is told what to download, and hands the libraries the path. It does not know which models
//! exist: the app owns the catalog (title, description, URL, hash) and tells this module about one model at a time, in an
//! [`AiModelFile`]. The only thing it knows is which roles it needs a model for.
//!
//! A model with a manifest is stored as `<role>-<12 hash characters>.onnx` with `<role>-<12 hash characters>.yml` beside
//! it, which is where `AiModel::load` looks for the manifest.

use std::{
  fs,
  io::{Read, Write},
  path::{Path, PathBuf},
  sync::{Arc, Mutex},
};

use abra_body_segmentation::prelude::ImageAi;
use abra_body_segmentation::{BodyParser, BodySegmentation};
use abra_find_person::PersonSegmenter;
use sha2::{Digest, Sha256};

use crate::{
  AbraError,
  consts::{PERSON_DETECTION_ROLE, SKIN_SEGMENTATION_ROLE},
  image_workers,
};

/// One model file to download, as the app's catalog describes it. The app owns the catalog and passes this when it wants the
/// model downloaded or asks whether it is on this device.
#[derive(Clone, Debug, uniffi::Record)]
pub struct AiModelFile {
  /// What the model is used for; names the file on this device.
  pub role: String,
  /// Where the file is downloaded from.
  pub url: String,
  /// Lowercase hex SHA-256 of the file.
  pub sha256: String,
  /// Where the manifest is downloaded from, and its lowercase hex SHA-256. Both or neither.
  pub manifest_url: Option<String>,
  pub manifest_sha256: Option<String>,
}

static DOWNLOAD_LOCK: Mutex<()> = Mutex::new(());

/// A model that is loaded the first time it is used, from the file downloaded for its role, and kept for every use after. It is the
/// whole of what a model needs here: any type that is an [`ImageAi`] (every Abra AI crate's model is) fits in one, so adding a model is
/// one `static` line and a worker in `image_workers.rs`, with no loader of its own.
///
/// ```ignore
/// pub(crate) static BODY_PARSER: ModelSlot<BodyParser> = ModelSlot::new(SKIN_SEGMENTATION_ROLE);
/// let parts = BODY_PARSER.get()?.process(&image)?;
/// ```
pub(crate) struct ModelSlot<T> {
  role: &'static str,
  cached: Mutex<Option<Arc<T>>>,
}

impl<T: ImageAi> ModelSlot<T> {
  /// A slot for the model downloaded for this role. Nothing is loaded until [`ModelSlot::get`].
  pub(crate) const fn new(p_role: &'static str) -> Self {
    Self {
      role: p_role,
      cached: Mutex::new(None),
    }
  }

  /// The model, loaded on first use from the downloaded file. Fails if the model is not downloaded or cannot be loaded. A failure is not
  /// cached, so the next call tries again, which is what is wanted when the download finishes in between.
  pub(crate) fn get(&self) -> Result<Arc<T>, AbraError> {
    let mut cached = self.cached.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(model) = cached.as_ref() {
      return Ok(Arc::clone(model));
    }
    let model = Arc::new(T::load(find_downloaded(self.role)?)?);
    *cached = Some(Arc::clone(&model));
    Ok(model)
  }

  /// Lets go of the loaded model and the memory it holds, so the next [`ModelSlot::get`] loads it again. A job that is using it keeps
  /// it until that job ends.
  pub(crate) fn unload(&self) {
    self.cached.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
  }
}

/// The skin segmenter, which gives the skin of the photo as one mask.
pub(crate) static SKIN_SEGMENTER: ModelSlot<BodySegmentation> = ModelSlot::new(SKIN_SEGMENTATION_ROLE);
/// The body parser, which labels the hair, skin, clothes and accessories. It is the same model file as the skin segmenter, read with
/// another manifest, so it is downloaded once.
pub(crate) static BODY_PARSER: ModelSlot<BodyParser> = ModelSlot::new(SKIN_SEGMENTATION_ROLE);
/// The person segmenter, which finds each person in the photo and their body.
pub(crate) static PERSON_SEGMENTER: ModelSlot<PersonSegmenter> = ModelSlot::new(PERSON_DETECTION_ROLE);

/// Lets go of every loaded model, and the memory they hold, once the work that needed them is done. Each is loaded again the next
/// time it is used.
#[uniffi::export]
pub fn unload_ai_models() {
  SKIN_SEGMENTER.unload();
  BODY_PARSER.unload();
  PERSON_SEGMENTER.unload();
}

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

impl AiModelFile {
  /// Checks what the app passed: a role that is safe in a file name, SHA-256 hashes, and a manifest that has both its URL
  /// and its hash or neither.
  fn check(&self) -> Result<(), AbraError> {
    let invalid = |message: String| Err(AbraError::Ai { message });
    if self.role.is_empty()
      || !self.role.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
      return invalid(format!("`{}` is not a model role", self.role));
    }
    let is_sha256 = |hash: &str| hash.len() == 64 && hash.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
    if !is_sha256(&self.sha256) {
      return invalid(format!("The hash for {} is not a lowercase SHA-256", self.url));
    }
    match (&self.manifest_url, &self.manifest_sha256) {
      (None, None) => Ok(()),
      (Some(_), Some(hash)) if is_sha256(hash) => Ok(()),
      (Some(url), Some(_)) => invalid(format!("The hash for {url} is not a lowercase SHA-256")),
      _ => invalid(format!("The model {} needs both `manifest_url` and `manifest_sha256`, or neither", self.url)),
    }
  }

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

  /// Where the manifest is stored: beside the model, with the same name and a `.yml` extension, which is where
  /// `AiModel::load` reads it from.
  fn local_manifest_path(&self) -> PathBuf {
    self.local_path().with_extension("yml")
  }

  /// Whether the manifest is on this device, or the model has none. Like the model, a manifest only gets its final name
  /// after passing the hash check.
  fn manifest_on_disk(&self) -> bool {
    self.manifest_url.is_none() || self.local_manifest_path().is_file()
  }

  /// Returns the local path of the model, downloading it and its manifest first if needed.
  fn ensure(&self) -> Result<PathBuf, AbraError> {
    self.check()?;
    let _guard = DOWNLOAD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = self.local_path();

    let result = fs::create_dir_all(model_dir())
      .map_err(|e| format!("Cannot create the model folder: {e}"))
      .and_then(|_| self.ensure_file(&self.url, &self.sha256, &path))
      .and_then(|_| match (&self.manifest_url, &self.manifest_sha256) {
        (Some(url), Some(sha256)) => self.ensure_file(url, sha256, &self.local_manifest_path()),
        _ => Ok(()),
      });
    match result {
      Ok(()) => {
        // Older versions of this model are no longer used.
        for entry in downloaded_files(&self.role) {
          if entry.path() != path {
            let _ = fs::remove_file(entry.path().with_extension("yml"));
            let _ = fs::remove_file(entry.path());
          }
        }
        Ok(path)
      }
      Err(message) => Err(AbraError::Ai { message }),
    }
  }

  /// Makes sure `p_path` holds the file at `p_url` with hash `p_sha256`: keeps a file that already matches, otherwise
  /// downloads it.
  fn ensure_file(&self, p_url: &str, p_sha256: &str, p_path: &Path) -> Result<(), String> {
    if p_path.exists() {
      match hash_file(p_path) {
        Ok(hash) if hash == p_sha256 => return Ok(()),
        _ => {
          let _ = fs::remove_file(p_path);
        }
      }
    }

    let partial = p_path.with_extension("part");
    let result = self
      .download(p_url, p_sha256, &partial)
      .and_then(|_| fs::rename(&partial, p_path).map_err(|e| format!("Cannot save the {} model: {e}", self.role)));
    if result.is_err() {
      let _ = fs::remove_file(&partial);
    }
    result
  }

  /// Downloads the file at `p_url` to `p_partial` and checks its hash.
  fn download(&self, p_url: &str, p_sha256: &str, p_partial: &Path) -> Result<(), String> {
    let response = ureq::get(p_url).call().map_err(|e| format!("Failed to download the {} model: {e}", self.role))?;
    let mut reader = response.into_body().into_reader();
    let mut file = fs::File::create(p_partial).map_err(|e| format!("Cannot create {p_partial:?}: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
      let read =
        reader.read(&mut buffer).map_err(|e| format!("Download of the {} model was interrupted: {e}", self.role))?;
      if read == 0 {
        break;
      }
      hasher.update(&buffer[..read]);
      file.write_all(&buffer[..read]).map_err(|e| format!("Cannot write the {} model: {e}", self.role))?;
    }
    file.flush().map_err(|e| format!("Cannot write the {} model: {e}", self.role))?;

    let hash = hex(&hasher.finalize());
    if hash != p_sha256 {
      return Err(format!("The downloaded {} model does not match its hash (got {hash})", self.role));
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

/// Whether the model that finds people in a photo is on this device. Works offline.
#[uniffi::export]
pub fn person_detection_downloaded() -> bool {
  find_downloaded(PERSON_DETECTION_ROLE).is_ok()
}

/// The size in bytes of a downloaded model, or 0 when the model, or its manifest, is not on this device. Works offline. The
/// exact file for `model.sha256` has to be there, so a new version in the catalog counts as not downloaded.
#[uniffi::export]
pub fn downloaded_ai_model_bytes(model: AiModelFile) -> u64 {
  if model.check().is_err() || !model.manifest_on_disk() {
    return 0;
  }
  model.size_on_disk().unwrap_or(0)
}

/// Downloads an AI model, and its manifest, unless they are already on this device.
#[uniffi::export(async_runtime = "tokio")]
pub async fn download_ai_model(model: AiModelFile) -> Result<(), AbraError> {
  image_workers::run(move || model.ensure().map(|_| ())).await
}

/// Where a downloaded AI model is on this device, for `AiModel.load`. Its manifest is beside it. Works offline. Fails when
/// the model is not downloaded, or only its `.onnx` is (an older download): `AiModel.load` needs the manifest, and
/// `download_ai_model` fetches it.
#[uniffi::export]
pub fn ai_model_path(id: String) -> Result<String, AbraError> {
  let path = find_downloaded(&id)?;
  if !path.with_extension("yml").is_file() {
    return Err(AbraError::Ai {
      message: format!("The model for {id} has no manifest on this device; download it again"),
    });
  }
  Ok(path.to_string_lossy().into_owned())
}

/// Deletes the downloaded files of an AI model, every version of it. It downloads again the next time a feature needs it. A
/// model already loaded in memory keeps working until the app restarts.
#[uniffi::export]
pub fn delete_ai_model(role: String) -> Result<(), AbraError> {
  let _guard = DOWNLOAD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  for entry in downloaded_files(&role) {
    let _ = fs::remove_file(entry.path().with_extension("yml"));
    match fs::remove_file(entry.path()) {
      Ok(()) => {}
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
      Err(e) => {
        return Err(AbraError::Ai {
          message: format!("Cannot delete the {role} model: {e}"),
        });
      }
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn file(role: &str) -> AiModelFile {
    AiModelFile {
      role: role.to_string(),
      url: "https://example.com/models/a.onnx".to_string(),
      sha256: "0123456789abcdef".repeat(4),
      manifest_url: Some("https://example.com/models/a.yml".to_string()),
      manifest_sha256: Some("fedcba9876543210".repeat(4)),
    }
  }

  /// A model that counts how many times it was loaded, to see what a [`ModelSlot`] does.
  struct Counted;

  static LOADS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

  impl ImageAi for Counted {
    type Output = ();

    fn load(p_model_path: impl AsRef<Path>) -> Result<Self, abra_body_segmentation::prelude::AiError> {
      LOADS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
      // A file with nothing in it is a model that cannot be loaded.
      if fs::metadata(p_model_path).map(|metadata| metadata.len()).unwrap_or(0) == 0 {
        return Err(abra_body_segmentation::prelude::AiError::model_load_failed("empty file"));
      }
      Ok(Counted)
    }

    fn process(&self, _p_image: &abra::prelude::Image) -> Result<(), abra_body_segmentation::prelude::AiError> {
      Ok(())
    }
  }

  #[test]
  fn a_model_slot_loads_on_first_use_keeps_the_model_and_does_not_keep_a_failure() {
    const ROLE: &str = "slot-test-model";
    static SLOT: ModelSlot<Counted> = ModelSlot::new(ROLE);
    let file = model_dir().join(format!("{ROLE}-0123456789ab.onnx"));
    fs::create_dir_all(model_dir()).unwrap();
    let _ = fs::remove_file(&file);
    let loads = || LOADS.load(std::sync::atomic::Ordering::SeqCst);
    let before = loads();

    // Not downloaded: an error that says so, and nothing is loaded.
    let missing = SLOT.get().err().expect("no file yet");
    assert!(matches!(&missing, AbraError::Ai { message } if message.contains("not downloaded")), "{missing}");
    assert_eq!(loads(), before);

    // A file that is not a model: the load fails, and the failure is not kept.
    fs::write(&file, b"").unwrap();
    assert!(matches!(SLOT.get().err(), Some(AbraError::Ai { .. })));
    assert_eq!(loads(), before + 1);

    // The download finishes: the next call loads it, and every call after gets the same model without loading again.
    fs::write(&file, b"a model").unwrap();
    let first = SLOT.get().expect("loads now");
    let second = SLOT.get().expect("is kept");
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(loads(), before + 2);
    let _ = fs::remove_file(&file);
  }

  #[test]
  fn a_model_file_needs_a_safe_role_real_hashes_and_both_manifest_parts_or_neither() {
    assert!(file("skin-segmentation").check().is_ok());
    assert!(
      AiModelFile {
        manifest_url: None,
        manifest_sha256: None,
        ..file("a")
      }
      .check()
      .is_ok()
    );
    assert!(file("").check().is_err());
    assert!(file("../escape").check().is_err());
    assert!(
      AiModelFile {
        sha256: "nothex".to_string(),
        ..file("a")
      }
      .check()
      .is_err()
    );
    assert!(
      AiModelFile {
        sha256: "A".repeat(64),
        ..file("a")
      }
      .check()
      .is_err(),
      "the hash is lowercase"
    );
    assert!(
      AiModelFile {
        manifest_sha256: None,
        ..file("a")
      }
      .check()
      .is_err()
    );
    assert!(
      AiModelFile {
        manifest_url: None,
        ..file("a")
      }
      .check()
      .is_err()
    );
    assert!(
      AiModelFile {
        manifest_sha256: Some("x".to_string()),
        ..file("a")
      }
      .check()
      .is_err()
    );
  }

  #[test]
  fn a_model_path_needs_the_manifest_beside_the_model() {
    let role = "path-test-role";
    fs::create_dir_all(model_dir()).unwrap();
    let onnx = model_dir().join(format!("{role}-0123456789ab.onnx"));
    fs::write(&onnx, b"model").unwrap();
    let _ = fs::remove_file(onnx.with_extension("yml"));

    assert!(ai_model_path(role.to_string()).is_err(), "an ONNX-only download cannot be loaded as an AiModel");
    fs::write(onnx.with_extension("yml"), b"name: x\n").unwrap();
    assert_eq!(ai_model_path(role.to_string()).unwrap(), onnx.to_string_lossy());
    assert!(ai_model_path("no-such-model".to_string()).is_err());

    let _ = fs::remove_file(&onnx);
    let _ = fs::remove_file(onnx.with_extension("yml"));
  }

  #[test]
  fn a_model_is_downloaded_when_its_exact_file_and_manifest_are_here_and_delete_removes_every_version() {
    let model = file("bytes-test-role");
    assert_eq!(downloaded_ai_model_bytes(model.clone()), 0, "nothing is here yet");

    fs::create_dir_all(model_dir()).unwrap();
    fs::write(model.local_path(), b"model").unwrap();
    assert_eq!(downloaded_ai_model_bytes(model.clone()), 0, "the manifest is missing");
    fs::write(model.local_manifest_path(), b"name: x\n").unwrap();
    assert_eq!(downloaded_ai_model_bytes(model.clone()), 5);
    assert_eq!(
      downloaded_ai_model_bytes(AiModelFile {
        sha256: "f".repeat(64),
        ..model.clone()
      }),
      0,
      "a different version in the catalog is not downloaded"
    );
    assert_eq!(
      downloaded_ai_model_bytes(AiModelFile {
        sha256: "bad".to_string(),
        ..model.clone()
      }),
      0
    );

    let old = model_dir().join("bytes-test-role-ffffffffffff.onnx");
    fs::write(&old, b"old").unwrap();
    delete_ai_model("bytes-test-role".to_string()).unwrap();
    assert!(!model.local_path().exists() && !model.local_manifest_path().exists() && !old.exists());
    delete_ai_model("bytes-test-role".to_string()).unwrap();
  }
}
