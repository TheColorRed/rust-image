//! The plugins that ship in this repository (`packages/abra/ai/<crate>/plugins/<id>/plugin.yml`) are valid, and say the truth about their
//! files: every file a contribution names is listed to download, exists, and has the SHA-256 the plugin lists. A plugin that is wrong
//! here would fail only on a phone, when someone tries to install it, so it is checked where it is written.

use abra::plugin::PluginManifest;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn hash(p_path: &std::path::Path) -> String {
  Sha256::digest(fs::read(p_path).unwrap()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Every `plugin.yml` of a plugin that ships: its path, and the `models` folder of its crate, where its files may be instead.
fn shipped() -> Vec<(PathBuf, PathBuf)> {
  let ai = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ai");
  let mut found = Vec::new();
  for crate_dir in fs::read_dir(&ai).unwrap().flatten().map(|entry| entry.path()) {
    let Ok(plugins) = fs::read_dir(crate_dir.join("plugins")) else { continue };
    for plugin_dir in plugins.flatten().map(|entry| entry.path()) {
      if plugin_dir.join("plugin.yml").is_file() {
        found.push((plugin_dir, crate_dir.join("models")));
      }
    }
  }
  found.sort();
  found
}

#[test]
fn every_shipped_plugin_is_valid_and_its_files_are_what_it_lists() {
  let plugins = shipped();
  assert!(plugins.len() >= 8, "the shipped plugins are found: {}", plugins.len());
  for (dir, models) in plugins {
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    let manifest = PluginManifest::parse(&fs::read_to_string(dir.join("plugin.yml")).unwrap())
      .unwrap_or_else(|error| panic!("{name}: {error}"));
    // The folder a plugin sits in does not matter to it, here or where it is downloaded from: its id is what names it. A folder of
    // plugins that add AI models starts with `ai-` by convention, so they can be told apart.
    if !manifest.models.is_empty() {
      assert!(name.starts_with("ai-"), "the folder {name} holds a plugin of AI models, so it starts with `ai-`");
    }
    assert!(!manifest.files.is_empty(), "{name} lists no files to download");

    // Everything a contribution names can be downloaded.
    let named = manifest
      .models
      .iter()
      .flat_map(|model| [&model.model, &model.manifest])
      .chain(manifest.scripts.iter().map(|script| &script.entry));
    for file in named {
      assert!(manifest.files.iter().any(|listed| &listed.path == file), "{name} names {file} but does not list it in `files`");
    }

    for listed in &manifest.files {
      let Some(path) = [dir.join(&listed.path), models.join(&listed.path)].into_iter().find(|path| path.is_file()) else {
        // A file too big for git (LaMa's) is downloaded, not committed.
        assert!(listed.path.ends_with(".onnx"), "{name}: {} is not in the plugin's folder or in the crate's models", listed.path);
        continue;
      };
      assert_eq!(hash(&path), listed.sha256, "{name}: {} is not the file the plugin.yml lists", listed.path);
    }
  }
}
