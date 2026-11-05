//! What a plugin keeps for itself across projects: named text files in `abra-plugin-saves/<plugin-id>/`, a sibling of the plugins folder.
//!
//! A plugin made of its own tool (a particle maker, say) makes things that other projects use later, so they cannot live in the project
//! that made them. They are kept beside the plugins folder and not inside it, so removing a plugin does not remove what it made: a plugin
//! that is installed again finds its saves where it left them. A plugin reaches only its own folder. Nothing manages the saves yet.
//!
//! A save is text (a plugin writes JSON), with a plain name, and is written to a `.part` file that is renamed into place, so a save that is
//! read is always a whole one.

use std::{fs, path::PathBuf};

use super::install::{is_plugin_id, plugins_dir};
use crate::AbraError;

/// The most a save may hold. A save is a description of something, not a picture.
const MAX_BYTES: usize = 5 * 1024 * 1024;

/// Where the saves are on this device: a folder beside the plugins folder.
fn saves_dir() -> PathBuf {
  plugins_dir().with_file_name("abra-plugin-saves")
}

fn invalid(p_message: String) -> AbraError {
  AbraError::Io { message: p_message }
}

/// The folder of a plugin's saves, for a plugin id such as `alakazam.particle-systems`. It may not exist yet.
fn plugin_folder(p_plugin_id: &str) -> Result<PathBuf, AbraError> {
  if is_plugin_id(p_plugin_id) {
    Ok(saves_dir().join(p_plugin_id))
  } else {
    Err(invalid(format!("`{p_plugin_id}` is not a plugin id")))
  }
}

/// A save's name is a file name: letters, digits, spaces, `-`, `_` and `.`, not starting with a dot, and nothing that leaves the folder.
fn save_path(p_plugin_id: &str, p_name: &str) -> Result<PathBuf, AbraError> {
  let plain = !p_name.trim().is_empty()
    && p_name.len() <= 100
    && !p_name.starts_with('.')
    && !p_name.contains("..")
    && !p_name.ends_with(".part")
    && p_name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'-' | b'_' | b'.'));
  if plain { Ok(plugin_folder(p_plugin_id)?.join(p_name)) } else { Err(invalid(format!("`{p_name}` is not a save name"))) }
}

/// The names of the plugin's saves, in order. A plugin that has saved nothing has none.
#[uniffi::export]
pub fn plugin_save_list(plugin_id: String) -> Result<Vec<String>, AbraError> {
  let folder = plugin_folder(&plugin_id)?;
  let entries = match fs::read_dir(&folder) {
    Ok(entries) => entries,
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
    Err(e) => return Err(invalid(format!("Cannot list the saves of {plugin_id}: {e}"))),
  };
  let mut names: Vec<String> = entries
    .filter_map(Result::ok)
    .filter(|entry| entry.path().is_file())
    .map(|entry| entry.file_name().to_string_lossy().into_owned())
    .filter(|name| !name.ends_with(".part"))
    .collect();
  names.sort();
  Ok(names)
}

/// The text of a save, or nothing when the plugin has no save with that name.
#[uniffi::export]
pub fn plugin_save_read(plugin_id: String, name: String) -> Result<Option<String>, AbraError> {
  match fs::read_to_string(save_path(&plugin_id, &name)?) {
    Ok(text) => Ok(Some(text)),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
    Err(e) => Err(invalid(format!("Cannot read the save {name} of {plugin_id}: {e}"))),
  }
}

/// Keeps `text` under `name`, replacing what was there.
#[uniffi::export]
pub fn plugin_save_write(plugin_id: String, name: String, text: String) -> Result<(), AbraError> {
  if text.len() > MAX_BYTES {
    return Err(invalid(format!("The save {name} is {} bytes, and a save can hold at most {MAX_BYTES}", text.len())));
  }
  let file = save_path(&plugin_id, &name)?;
  let folder = plugin_folder(&plugin_id)?;
  fs::create_dir_all(&folder).map_err(|e| invalid(format!("Cannot make the saves folder of {plugin_id}: {e}")))?;
  let part = file.with_file_name(format!("{name}.part"));
  fs::write(&part, text).map_err(|e| invalid(format!("Cannot write the save {name} of {plugin_id}: {e}")))?;
  fs::rename(&part, &file).map_err(|e| invalid(format!("Cannot keep the save {name} of {plugin_id}: {e}")))
}

/// Removes a save. It does nothing when there is none with that name.
#[uniffi::export]
pub fn plugin_save_remove(plugin_id: String, name: String) -> Result<(), AbraError> {
  match fs::remove_file(save_path(&plugin_id, &name)?) {
    Ok(()) => Ok(()),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(e) => Err(invalid(format!("Cannot remove the save {name} of {plugin_id}: {e}"))),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A plugin id no other test uses, and a clean folder for it.
  fn fresh(p_id: &str) -> String {
    let _ = fs::remove_dir_all(saves_dir().join(p_id));
    p_id.to_string()
  }

  #[test]
  fn a_save_is_written_read_listed_and_removed() {
    let id = fresh("tests.saves-roundtrip");
    assert_eq!(plugin_save_list(id.clone()).unwrap(), Vec::<String>::new());
    assert_eq!(plugin_save_read(id.clone(), "fire.json".into()).unwrap(), None);
    plugin_save_write(id.clone(), "fire.json".into(), "{\"a\":1}".into()).unwrap();
    plugin_save_write(id.clone(), "Water 2.json".into(), "{}".into()).unwrap();
    assert_eq!(plugin_save_read(id.clone(), "fire.json".into()).unwrap().as_deref(), Some("{\"a\":1}"));
    assert_eq!(plugin_save_list(id.clone()).unwrap(), vec!["Water 2.json".to_string(), "fire.json".to_string()]);
    // Writing again replaces it, and no half-written file is left listed.
    plugin_save_write(id.clone(), "fire.json".into(), "{\"a\":2}".into()).unwrap();
    assert_eq!(plugin_save_read(id.clone(), "fire.json".into()).unwrap().as_deref(), Some("{\"a\":2}"));
    assert_eq!(plugin_save_list(id.clone()).unwrap().len(), 2);
    plugin_save_remove(id.clone(), "fire.json".into()).unwrap();
    plugin_save_remove(id.clone(), "fire.json".into()).unwrap();
    assert_eq!(plugin_save_list(id.clone()).unwrap(), vec!["Water 2.json".to_string()]);
    let _ = fs::remove_dir_all(saves_dir().join(&id));
  }

  #[test]
  fn a_plugin_only_reaches_its_own_saves() {
    let first = fresh("tests.saves-first");
    let second = fresh("tests.saves-second");
    plugin_save_write(first.clone(), "mine".into(), "1".into()).unwrap();
    assert_eq!(plugin_save_read(second.clone(), "mine".into()).unwrap(), None);
    assert_eq!(plugin_save_list(second.clone()).unwrap(), Vec::<String>::new());
    let _ = fs::remove_dir_all(saves_dir().join(&first));
  }

  #[test]
  fn a_name_or_id_that_leaves_the_folder_is_refused() {
    let id = fresh("tests.saves-refuse");
    for name in ["../other", "a/b", "a\\b", "..", ".hidden", "", "  ", "x.part", &"n".repeat(101)] {
      assert!(plugin_save_write(id.clone(), name.into(), "x".into()).is_err(), "{name:?} was accepted");
      assert!(plugin_save_read(id.clone(), name.into()).is_err(), "{name:?} was read");
    }
    for plugin in ["", "no-dot", "a.b.c", "../x.y", "A.B", "x/y.z"] {
      assert!(plugin_save_list(plugin.into()).is_err(), "{plugin:?} was accepted");
    }
  }

  #[test]
  fn a_save_that_is_too_big_is_refused() {
    let id = fresh("tests.saves-big");
    assert!(plugin_save_write(id.clone(), "big".into(), "x".repeat(MAX_BYTES + 1)).is_err());
    assert_eq!(plugin_save_list(id).unwrap(), Vec::<String>::new());
  }

  #[test]
  fn the_saves_are_beside_the_plugins_and_not_inside_them() {
    assert_eq!(saves_dir().parent(), plugins_dir().parent());
    assert!(!saves_dir().starts_with(plugins_dir()));
  }
}
