//! Plugins on this device: where they live, installing one from a URL, and removing one.
//!
//! A plugin is a folder with a `plugin.yml` (see `abra::plugin`). This module keeps them in the app's private folder, next to the AI
//! models, and installs one the way a model is installed: every file is downloaded to a `.part` file, its SHA-256 is checked, and only
//! then does it get its final name. The whole plugin is built in a staging folder and moved into place once it has been read back
//! successfully, so a folder in the plugins folder is always a complete plugin, and reading the folder never sees a half-installed one.
//!
//! The app owns what is installable (the catalog lists each plugin archive URL and hash). The archive contains the manifest and every
//! file it lists, each checked against its declared hash before it can be installed.

use std::{
  fs,
  io::{Read, Write},
  path::PathBuf,
  sync::Arc,
};

use sha2::{Digest, Sha256};

use abra::plugin::{Contribution, Plugin, PluginManifest, PluginRegistry};

use crate::{
  AbraError, image_workers,
  models::{AiModelFile, DOWNLOAD_LOCK, DownloadListener, model_dir},
};

/// Where the plugins are on this device: a folder beside the AI models, so it is in the same private place.
pub(crate) fn plugins_dir() -> PathBuf {
  model_dir().with_file_name("abra-plugins")
}

/// The folder the plugins are kept in, for `scanPlugins`. It does not have to exist yet: a folder that is not there holds no plugins.
#[uniffi::export]
pub fn plugins_folder() -> String {
  plugins_dir().to_string_lossy().into_owned()
}

/// The `.onnx` file of the plugin model with this id, when an installed plugin adds one and its manifest is there too (a plugin's model
/// manifest is always the model's name with `.yml`, which is where the AI runtime reads it from). Works offline.
pub(crate) fn plugin_model_path(p_id: &str) -> Option<PathBuf> {
  let registry = PluginRegistry::scan(plugins_dir());
  registry.contributions().into_iter().find_map(|contribution| match contribution {
    Contribution::Model(plugin, model) if model.id == p_id && plugin.path(&model.manifest).is_file() => {
      Some(plugin.path(&model.model))
    }
    _ => None,
  })
}

/// Tells a listener how far one file of several has got, as a share of the whole.
struct Share<'a> {
  listener: &'a dyn DownloadListener,
  done: usize,
  count: usize,
}

impl DownloadListener for Share<'_> {
  fn update(&self, progress: f32) {
    self.listener.update((self.done as f32 + progress) / self.count as f32);
  }
}

fn is_sha256(p_hash: &str) -> bool {
  p_hash.len() == 64 && p_hash.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// Installs a ZIP downloaded from the catalog after checking its archive hash.
fn install_archive_url(
  p_archive_url: &str,
  p_archive_sha256: &str,
  p_manifest_sha256: &str,
  p_listener: &dyn DownloadListener,
) -> Result<(), String> {
  if !is_sha256(p_archive_sha256) || !is_sha256(p_manifest_sha256) {
    return Err(format!("The catalog hashes for {p_archive_url} are not lowercase SHA-256 values"));
  }
  let _guard = DOWNLOAD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let root = plugins_dir();
  fs::create_dir_all(&root).map_err(|error| format!("Cannot create the plugin folder: {error}"))?;
  let archive_path = root.join(".download.plugin.zip");
  let download = AiModelFile::ensure_file("the plugin package", p_archive_url, p_archive_sha256, &archive_path, Some(p_listener));
  if let Err(error) = download {
    let _ = fs::remove_file(&archive_path);
    return Err(error);
  }
  // install_archive takes the same lock, so release it before calling the shared local archive installer.
  drop(_guard);
  let result = install_archive(&archive_path.to_string_lossy(), Some(p_manifest_sha256), p_listener);
  let _ = fs::remove_file(archive_path);
  result
}

/// Installs the plugin whose `plugin.yml` is at `p_manifest_url`, replacing an installed one with the same id.
fn install(p_manifest_url: &str, p_manifest_sha256: &str, p_listener: &dyn DownloadListener) -> Result<(), String> {
  if !is_sha256(p_manifest_sha256) {
    return Err(format!("The hash for {p_manifest_url} is not a lowercase SHA-256"));
  }
  let base = p_manifest_url
    .rsplit_once('/')
    .map(|(base, _)| base)
    .ok_or_else(|| format!("{p_manifest_url} is not a URL of a plugin.yml"))?;
  let _guard = DOWNLOAD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let root = plugins_dir();
  let staging = root.join(".installing");
  let _ = fs::remove_dir_all(&staging);
  fs::create_dir_all(&staging).map_err(|e| format!("Cannot create the plugin folder: {e}"))?;

  let built = (|| {
    AiModelFile::ensure_file("the plugin manifest", p_manifest_url, p_manifest_sha256, &staging.join("plugin.yml"), None)?;
    let text = fs::read_to_string(staging.join("plugin.yml")).map_err(|e| format!("Cannot read the plugin manifest: {e}"))?;
    let manifest = PluginManifest::parse(&text).map_err(|e| e.to_string())?;

    // Everything the plugin's contributions name must be among the files that are downloaded, or it would not be there to use.
    let named = manifest
      .models
      .iter()
      .flat_map(|model| [&model.model, &model.manifest])
      .chain(manifest.scripts.iter().map(|script| &script.entry))
      .chain(manifest.controls.iter().map(|control| &control.entry));
    for file in named {
      if !manifest.files.iter().any(|listed| &listed.path == file) {
        return Err(format!("The plugin {} names {file} but does not list it in `files`, so it cannot be downloaded", manifest.id));
      }
    }

    let count = manifest.files.len().max(1);
    for (done, file) in manifest.files.iter().enumerate() {
      let destination = staging.join(&file.path);
      if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create the plugin folder: {e}"))?;
      }
      let label = format!("{} of the plugin {}", file.path, manifest.id);
      let share = Share { listener: p_listener, done, count };
      AiModelFile::ensure_file(&label, &format!("{base}/{}", file.path), &file.sha256, &destination, Some(&share))?;
    }

    // Read it back as the app will, so a plugin that cannot be used is never put in place.
    let plugin = Plugin::read(&staging).map_err(|e| e.to_string())?;
    let target = root.join(&plugin.manifest.id);
    match fs::remove_dir_all(&target) {
      Ok(()) => {}
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
      Err(e) => return Err(format!("Cannot replace the installed plugin: {e}")),
    }
    fs::rename(&staging, &target).map_err(|e| format!("Cannot save the plugin: {e}"))
  })();
  if built.is_err() {
    let _ = fs::remove_dir_all(&staging);
  } else {
    p_listener.update(1.0);
  }
  built
}

/// Installs a plugin package chosen from this device. The ZIP contains `plugin.yml` at its root and every file listed by its manifest.
/// Files are copied into staging and checked against their declared SHA-256 before the plugin replaces an installed version.
fn install_archive(p_archive_path: &str, p_manifest_sha256: Option<&str>, p_listener: &dyn DownloadListener) -> Result<(), String> {
  const MAX_PACKAGE_BYTES: u64 = 1024 * 1024 * 1024;

  let _guard = DOWNLOAD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let archive_file = fs::File::open(p_archive_path).map_err(|error| format!("Cannot open the plugin package: {error}"))?;
  let mut archive = zip::ZipArchive::new(archive_file).map_err(|error| format!("This is not a valid plugin ZIP: {error}"))?;
  let mut manifest_text = String::new();
  archive
    .by_name("plugin.yml")
    .map_err(|_| "The ZIP must contain plugin.yml at its root".to_string())?
    .read_to_string(&mut manifest_text)
    .map_err(|error| format!("Cannot read plugin.yml: {error}"))?;
  if let Some(expected) = p_manifest_sha256 {
    let actual = Sha256::digest(manifest_text.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    if actual != expected {
      return Err(format!("The plugin manifest in {p_archive_path} does not match its catalog hash (got {actual})"));
    }
  }
  let manifest = PluginManifest::parse(&manifest_text).map_err(|error| error.to_string())?;

  let named = manifest
    .models
    .iter()
    .flat_map(|model| [&model.model, &model.manifest])
    .chain(manifest.scripts.iter().map(|script| &script.entry))
    .chain(manifest.controls.iter().map(|control| &control.entry));
  for file in named {
    if !manifest.files.iter().any(|listed| &listed.path == file) {
      return Err(format!("The plugin {} names {file} but does not list it in `files`", manifest.id));
    }
  }

  let mut total_bytes = 0u64;
  for file in &manifest.files {
    let size = archive
      .by_name(&file.path)
      .map_err(|_| format!("The plugin package is missing {}", file.path))?
      .size();
    total_bytes = total_bytes.saturating_add(size);
    if total_bytes > MAX_PACKAGE_BYTES {
      return Err("The plugin package is larger than 1 GB".to_string());
    }
  }

  let root = plugins_dir();
  let staging = root.join(".installing");
  let _ = fs::remove_dir_all(&staging);
  fs::create_dir_all(&staging).map_err(|error| format!("Cannot create the plugin folder: {error}"))?;

  let built = (|| {
    fs::write(staging.join("plugin.yml"), manifest_text).map_err(|error| format!("Cannot save plugin.yml: {error}"))?;
    let count = manifest.files.len().max(1);
    let mut written_bytes = 0u64;
    for (done, listed) in manifest.files.iter().enumerate() {
      let mut input = archive
        .by_name(&listed.path)
        .map_err(|_| format!("The plugin package is missing {}", listed.path))?;
      let destination = staging.join(&listed.path);
      if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("Cannot create the plugin folder: {error}"))?;
      }
      let mut output = fs::File::create(&destination).map_err(|error| format!("Cannot save {}: {error}", listed.path))?;
      let mut hasher = Sha256::new();
      let mut buffer = [0u8; 64 * 1024];
      loop {
        let bytes = input.read(&mut buffer).map_err(|error| format!("Cannot read {}: {error}", listed.path))?;
        if bytes == 0 {
          break;
        }
        written_bytes = written_bytes.saturating_add(bytes as u64);
        if written_bytes > MAX_PACKAGE_BYTES {
          return Err("The plugin package is larger than 1 GB".to_string());
        }
        hasher.update(&buffer[..bytes]);
        output
          .write_all(&buffer[..bytes])
          .map_err(|error| format!("Cannot save {}: {error}", listed.path))?;
      }
      let actual = hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
      if actual != listed.sha256 {
        return Err(format!("{} does not match its SHA-256 in plugin.yml", listed.path));
      }
      Share { listener: p_listener, done, count }.update(1.0);
    }

    let plugin = Plugin::read(&staging).map_err(|error| error.to_string())?;
    let target = root.join(&plugin.manifest.id);
    match fs::remove_dir_all(&target) {
      Ok(()) => {}
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
      Err(error) => return Err(format!("Cannot replace the installed plugin: {error}")),
    }
    fs::rename(&staging, &target).map_err(|error| format!("Cannot save the plugin: {error}"))
  })();

  if built.is_err() {
    let _ = fs::remove_dir_all(&staging);
  } else {
    p_listener.update(1.0);
  }
  built
}

/// Installs a plugin from its `plugin.yml` URL and the hash that file must have, unless the same version is already installed. The
/// listener hears how far the download has got, as a share of every file. It is not optional: the bindings generator cannot pass a
/// TypeScript object inside an `Option`.
#[uniffi::export(async_runtime = "tokio")]
pub async fn install_plugin(
  manifest_url: String,
  manifest_sha256: String,
  listener: Arc<dyn DownloadListener>,
) -> Result<(), AbraError> {
  image_workers::run(move || {
    install(&manifest_url, &manifest_sha256, listener.as_ref()).map_err(|message| AbraError::Ai { message })
  })
  .await
}

/// Installs an `.akzx` package selected locally. Every file is checked before the plugin is swapped into place.
#[uniffi::export(async_runtime = "tokio")]
pub async fn install_plugin_archive(
  archive_path: String,
  listener: Arc<dyn DownloadListener>,
) -> Result<(), AbraError> {
  image_workers::run(move || {
    install_archive(&archive_path, None, listener.as_ref()).map_err(|message| AbraError::Ai { message })
  })
  .await
}

/// Downloads and installs a catalog `.akzx` package, verifying its archive SHA-256 and every listed file.
#[uniffi::export(async_runtime = "tokio")]
pub async fn install_plugin_package(
  archive_url: String,
  archive_sha256: String,
  manifest_sha256: String,
  listener: Arc<dyn DownloadListener>,
) -> Result<(), AbraError> {
  image_workers::run(move || {
    install_archive_url(&archive_url, &archive_sha256, &manifest_sha256, listener.as_ref()).map_err(|message| AbraError::Ai { message })
  })
  .await
}

/// The text of a script of an installed plugin, for the app's JavaScript engine to run. `path` is a script path as `scanPlugins` gives it.
/// Only a `.js` file inside the plugins folder is read: a path that leaves it (with `..` or by a link) or any other kind of file is
/// refused, so a plugin cannot use this to read what is not its own.
#[uniffi::export]
pub fn read_plugin_script(path: String) -> Result<String, AbraError> {
  let refuse = |message: String| Err(AbraError::Io { message });
  if !path.ends_with(".js") {
    return refuse(format!("{path} is not a plugin script"));
  }
  let root = fs::canonicalize(plugins_dir()).map_err(|e| AbraError::Io { message: format!("No plugins are installed: {e}") })?;
  let file = fs::canonicalize(&path).map_err(|e| AbraError::Io { message: format!("Cannot find the plugin script {path}: {e}") })?;
  if !file.starts_with(&root) {
    return refuse(format!("{path} is not inside the plugins folder"));
  }
  fs::read_to_string(&file).map_err(|e| AbraError::Io { message: format!("Cannot read the plugin script {path}: {e}") })
}

/// Removes an installed plugin and its files. It does nothing when the plugin is not installed.
#[uniffi::export]
pub fn delete_plugin(id: String) -> Result<(), AbraError> {
  let valid_name = |name: &str| {
    !name.is_empty()
      && name
        .split('-')
        .all(|word| !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit()))
  };
  let mut parts = id.split('.');
  if !matches!((parts.next(), parts.next(), parts.next()), (Some(author), Some(name), None) if valid_name(author) && valid_name(name)) {
    return Err(AbraError::Ai { message: format!("`{id}` is not a plugin id") });
  }
  let _guard = DOWNLOAD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  match fs::remove_dir_all(plugins_dir().join(&id)) {
    Ok(()) => Ok(()),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(e) => Err(AbraError::Ai { message: format!("Cannot delete the plugin {id}: {e}") }),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use sha2::{Digest, Sha256};
  use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    sync::Mutex,
  };

  /// The tests that install share the plugins folder and its staging folder, so they take turns.
  static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

  struct Heard(Mutex<Vec<f32>>);

  impl DownloadListener for Heard {
    fn update(&self, progress: f32) {
      self.0.lock().unwrap().push(progress);
    }
  }

  fn sha(p_bytes: &[u8]) -> String {
    Sha256::digest(p_bytes).iter().map(|byte| format!("{byte:02x}")).collect()
  }

  /// Serves files over HTTP on this computer, by path, and returns the address.
  fn serve(p_files: HashMap<String, Vec<u8>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
      for mut stream in listener.incoming().flatten() {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        reader.read_line(&mut request).unwrap();
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap_or(0) > 2 {
          line.clear();
        }
        let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
        match p_files.get(&path) {
          Some(body) => {
            let _ = write!(stream, "HTTP/1.1 200 OK
Content-Length: {}
Connection: close

", body.len());
            let _ = stream.write_all(body);
          }
          None => {
            let _ = write!(stream, "HTTP/1.1 404 Not Found
Content-Length: 0
Connection: close

");
          }
        }
      }
    });
    format!("http://{address}")
  }

  /// A plugin that adds one script, with the `files` the manifest is given.
  fn manifest(p_id: &str, p_files: &str) -> String {
    format!(
      "id: {p_id}
name: Test
version: 1.0.0
contributes:
  scripts:
    - id: run
      title: Run
      capabilities: [test]
      entry: s/run.js
{p_files}"
    )
  }

  #[test]
  fn a_file_of_several_reports_its_share_of_the_whole() {
    let heard = Heard(Mutex::new(Vec::new()));
    Share { listener: &heard, done: 1, count: 4 }.update(0.5);
    Share { listener: &heard, done: 3, count: 4 }.update(1.0);
    assert_eq!(*heard.0.lock().unwrap(), vec![0.375, 1.0]);
  }

  #[test]
  fn a_bad_hash_or_plugin_id_is_refused_before_anything_is_downloaded() {
    let heard = Heard(Mutex::new(Vec::new()));
    let error = install("https://example.com/plugins/a/plugin.yml", "nothash", &heard).unwrap_err();
    assert!(error.contains("lowercase SHA-256"), "{error}");
    assert!(delete_plugin("../x".into()).is_err());
    assert!(delete_plugin("test.not-installed-anywhere".into()).is_ok());
  }

  #[test]
  fn the_plugins_folder_is_beside_the_models() {
    assert_eq!(plugins_dir().file_name().unwrap(), "abra-plugins");
    assert_eq!(plugins_dir().parent(), model_dir().parent());
  }

  #[test]
  fn a_plugin_is_downloaded_checked_and_found_by_the_registry() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let script = b"export default () => 1;".to_vec();
    let text = manifest("test.install-test-good", &format!("files:
  - path: s/run.js
    sha256: {}
", sha(&script)));
    let hash = sha(text.as_bytes());
    let base = serve(HashMap::from([
      ("/p/plugin.yml".to_string(), text.into_bytes()),
      ("/p/s/run.js".to_string(), script.clone()),
    ]));
    let heard = Heard(Mutex::new(Vec::new()));
    install(&format!("{base}/p/plugin.yml"), &hash, &heard).unwrap();

    let registry = PluginRegistry::scan(plugins_dir());
    let found = registry.plugin("test.install-test-good").expect("the plugin is installed");
    assert_eq!(fs::read(found.path("s/run.js")).unwrap(), script);
    assert_eq!(registry.with_capability("test").len(), 1);
    assert_eq!(heard.0.lock().unwrap().last(), Some(&1.0));
    assert!(!plugins_dir().join(".installing").exists(), "the staging folder is gone");

    delete_plugin("test.install-test-good".into()).unwrap();
    assert!(PluginRegistry::scan(plugins_dir()).plugin("test.install-test-good").is_none());
  }

  #[test]
  fn a_file_that_does_not_match_its_hash_installs_nothing() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let text = manifest("test.install-test-tampered", &format!("files:
  - path: s/run.js
    sha256: {}
", sha(b"what it should be")));
    let hash = sha(text.as_bytes());
    let base = serve(HashMap::from([
      ("/p/plugin.yml".to_string(), text.into_bytes()),
      ("/p/s/run.js".to_string(), b"something else".to_vec()),
    ]));
    let heard = Heard(Mutex::new(Vec::new()));
    let error = install(&format!("{base}/p/plugin.yml"), &hash, &heard).unwrap_err();
    assert!(error.contains("does not match its hash"), "{error}");
    assert!(PluginRegistry::scan(plugins_dir()).plugin("test.install-test-tampered").is_none());
    assert!(!plugins_dir().join(".installing").exists());
  }

  #[test]
  fn a_plugin_that_names_a_file_it_does_not_list_is_not_installed() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let text = manifest("test.install-test-unlisted", "");
    let hash = sha(text.as_bytes());
    let base = serve(HashMap::from([("/p/plugin.yml".to_string(), text.into_bytes())]));
    let heard = Heard(Mutex::new(Vec::new()));
    let error = install(&format!("{base}/p/plugin.yml"), &hash, &heard).unwrap_err();
    assert!(error.contains("does not list it"), "{error}");
    assert!(PluginRegistry::scan(plugins_dir()).plugin("test.install-test-unlisted").is_none());
  }

  #[test]
  fn the_model_a_plugin_adds_is_found_by_its_id_like_any_other_model() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = plugins_dir().join("test.model-lookup-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("m.onnx"), b"x").unwrap();
    fs::write(dir.join("m.yml"), b"x").unwrap();
    fs::write(
      dir.join("plugin.yml"),
      "id: test.model-lookup-test
name: T
version: 1
contributes:
  models:
    - id: lookup-test-model
      title: T
      capabilities: [test]
      model: m.onnx
",
    )
    .unwrap();
    let found = crate::models::ai_model_path("lookup-test-model".to_string()).unwrap();
    assert!(found.ends_with("m.onnx") && found.contains("test.model-lookup-test"), "{found}");
    assert!(crate::models::ai_model_path("nothing-has-this-id".to_string()).is_err());
    delete_plugin("test.model-lookup-test".into()).unwrap();
    assert!(crate::models::ai_model_path("lookup-test-model".to_string()).is_err());
  }

  #[test]
  fn only_a_script_inside_the_plugins_folder_can_be_read() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = plugins_dir().join("test.script-read-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("run.js"), "exports.default = () => 1;").unwrap();
    fs::write(dir.join("notes.txt"), "not a script").unwrap();
    let script = dir.join("run.js").to_string_lossy().into_owned();
    assert_eq!(read_plugin_script(script).unwrap(), "exports.default = () => 1;");
    // Not a script, outside the folder, a way out by `..`, and a file that is not there are all refused.
    assert!(read_plugin_script(dir.join("notes.txt").to_string_lossy().into_owned()).is_err());
    let outside = std::env::temp_dir().join("abra-outside-script.js");
    fs::write(&outside, "exports.default = () => 2;").unwrap();
    assert!(read_plugin_script(outside.to_string_lossy().into_owned()).is_err());
    let climbing = dir.join("..").join("..").join("abra-outside-script.js");
    assert!(read_plugin_script(climbing.to_string_lossy().into_owned()).is_err());
    assert!(read_plugin_script(dir.join("missing.js").to_string_lossy().into_owned()).is_err());
    let _ = fs::remove_file(&outside);
    delete_plugin("test.script-read-test".into()).unwrap();
  }

  #[test]
  fn a_manifest_whose_hash_is_wrong_is_refused() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let text = manifest("test.install-test-hash", "");
    let base = serve(HashMap::from([("/p/plugin.yml".to_string(), text.into_bytes())]));
    let heard = Heard(Mutex::new(Vec::new()));
    let error = install(&format!("{base}/p/plugin.yml"), &sha(b"another manifest"), &heard).unwrap_err();
    assert!(error.contains("does not match its hash"), "{error}");
  }
}
