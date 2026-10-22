//! Plugins on this device: where they live, installing one from a URL, and removing one.
//!
//! A plugin is a folder with a `plugin.yml` (see `abra::plugin`). This module keeps them in the app's private folder, next to the AI
//! models, and installs one the way a model is installed: every file is downloaded to a `.part` file, its SHA-256 is checked, and only
//! then does it get its final name. The whole plugin is built in a staging folder and moved into place once it has been read back
//! successfully, so a folder in the plugins folder is always a complete plugin, and reading the folder never sees a half-installed one.
//!
//! The app owns what is installable (the catalog lists each plugin's `plugin.yml` URL and hash). The plugin's `files` list says what else
//! to download, from beside the `plugin.yml`, with the hash of each file.

use std::{fs, path::PathBuf, sync::Arc};

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
      .chain(manifest.scripts.iter().map(|script| &script.entry));
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

/// Removes an installed plugin and its files. It does nothing when the plugin is not installed.
#[uniffi::export]
pub fn delete_plugin(id: String) -> Result<(), AbraError> {
  if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-') {
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
    assert!(delete_plugin("not-installed-anywhere".into()).is_ok());
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
    let text = manifest("install-test-good", &format!("files:
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
    let found = registry.plugin("install-test-good").expect("the plugin is installed");
    assert_eq!(fs::read(found.path("s/run.js")).unwrap(), script);
    assert_eq!(registry.with_capability("test").len(), 1);
    assert_eq!(heard.0.lock().unwrap().last(), Some(&1.0));
    assert!(!plugins_dir().join(".installing").exists(), "the staging folder is gone");

    delete_plugin("install-test-good".into()).unwrap();
    assert!(PluginRegistry::scan(plugins_dir()).plugin("install-test-good").is_none());
  }

  #[test]
  fn a_file_that_does_not_match_its_hash_installs_nothing() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let text = manifest("install-test-tampered", &format!("files:
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
    assert!(PluginRegistry::scan(plugins_dir()).plugin("install-test-tampered").is_none());
    assert!(!plugins_dir().join(".installing").exists());
  }

  #[test]
  fn a_plugin_that_names_a_file_it_does_not_list_is_not_installed() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let text = manifest("install-test-unlisted", "");
    let hash = sha(text.as_bytes());
    let base = serve(HashMap::from([("/p/plugin.yml".to_string(), text.into_bytes())]));
    let heard = Heard(Mutex::new(Vec::new()));
    let error = install(&format!("{base}/p/plugin.yml"), &hash, &heard).unwrap_err();
    assert!(error.contains("does not list it"), "{error}");
    assert!(PluginRegistry::scan(plugins_dir()).plugin("install-test-unlisted").is_none());
  }

  #[test]
  fn the_model_a_plugin_adds_is_found_by_its_id_like_any_other_model() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = plugins_dir().join("model-lookup-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("m.onnx"), b"x").unwrap();
    fs::write(dir.join("m.yml"), b"x").unwrap();
    fs::write(
      dir.join("plugin.yml"),
      "id: model-lookup-test
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
    assert!(found.ends_with("m.onnx") && found.contains("model-lookup-test"), "{found}");
    assert!(crate::models::ai_model_path("nothing-has-this-id".to_string()).is_err());
    delete_plugin("model-lookup-test".into()).unwrap();
    assert!(crate::models::ai_model_path("lookup-test-model".to_string()).is_err());
  }

  #[test]
  fn a_manifest_whose_hash_is_wrong_is_refused() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let text = manifest("install-test-hash", "");
    let base = serve(HashMap::from([("/p/plugin.yml".to_string(), text.into_bytes())]));
    let heard = Heard(Mutex::new(Vec::new()));
    let error = install(&format!("{base}/p/plugin.yml"), &sha(b"another manifest"), &heard).unwrap_err();
    assert!(error.contains("does not match its hash"), "{error}");
  }
}
