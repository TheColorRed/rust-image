//! Fonts for the app: where they live on this device and downloading them.
//!
//! The Abra library only loads a font from a path it is given (`Font.fromPath`). This module stores the font files the app
//! downloads in the app's private folder, one folder for each family, and tells the app where they are. It does not know where fonts
//! come from: the app owns the list (Google Fonts) and passes one file's URL at a time.
//!
//! A family is stored as `abra-fonts/<family>/<file name>`, next to the `abra-models` folder.

use std::{fs, path::PathBuf, sync::Arc};

use abra::typography::prelude::{Font, TextSize};

use crate::{
  AbraError, image_workers,
  models::{AiModelFile, DownloadListener, model_dir},
};

/// A family of fonts that is on this device.
#[derive(Clone, Debug, uniffi::Record)]
pub struct DownloadedFont {
  /// The family's name, such as `Roboto Mono`.
  pub family: String,
  /// The path of each font file of the family, by file name, to give to `Font.fromPath`.
  pub files: Vec<String>,
  /// Size of all the family's files in bytes.
  pub bytes: u64,
}

fn fonts_dir() -> PathBuf {
  model_dir().with_file_name("abra-fonts")
}

fn invalid(p_message: String) -> AbraError {
  AbraError::Io { message: p_message }
}

/// A family name is a folder name, so it can hold letters, digits, spaces, `-` and `_` and nothing that leaves the folder.
fn family_dir(p_family: &str) -> Result<PathBuf, AbraError> {
  let safe = !p_family.trim().is_empty()
    && p_family.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'-' | b'_'));
  if safe { Ok(fonts_dir().join(p_family)) } else { Err(invalid(format!("`{p_family}` is not a font family name"))) }
}

/// A font file name is a `.ttf` or `.otf` file with a plain name.
fn font_file_name(p_name: &str) -> Result<&str, AbraError> {
  let lower = p_name.to_ascii_lowercase();
  let safe = !p_name.starts_with('.')
    && p_name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    && (lower.ends_with(".ttf") || lower.ends_with(".otf"));
  if safe { Ok(p_name) } else { Err(invalid(format!("`{p_name}` is not a font file name"))) }
}

/// Downloads one font file of a family, unless it is already on this device, and returns its path. The listener hears how far the
/// file has got, from 0 to 1. Files get their final name only when complete, so a file that is there is whole.
#[uniffi::export(async_runtime = "tokio")]
pub async fn download_font_file(
  family: String, file_name: String, url: String, listener: Arc<dyn DownloadListener>,
) -> Result<String, AbraError> {
  let path = family_dir(&family)?.join(font_file_name(&file_name)?);
  image_workers::run(move || {
    if fs::metadata(&path).is_ok_and(|metadata| metadata.len() > 0) {
      listener.update(1.0);
    } else {
      let folder = path.parent().expect("a font file is in its family's folder");
      fs::create_dir_all(folder).map_err(|e| invalid(format!("Cannot create {folder:?}: {e}")))?;
      AiModelFile::ensure_file(&format!("the font {family}"), &url, None, &path, Some(listener.as_ref()))
        .map_err(|message| invalid(message))?;
      listener.update(1.0);
    }
    Ok(path.to_string_lossy().into_owned())
  })
  .await
}

/// A picture of a family's name set in its own font, as a PNG in this color, and the picture's path. It uses the family's files when they
/// are on this device, and otherwise downloads just the one file at `url` into a cache of previews. A preview that was made before is
/// returned at once, so a list can ask for each row as it comes into view.
#[uniffi::export(async_runtime = "tokio")]
pub async fn font_preview(family: String, url: String, red: u8, green: u8, blue: u8) -> Result<String, AbraError> {
  let folder = family_dir(&family)?;
  let previews = fonts_dir().with_file_name("abra-font-previews");
  let picture = previews.join(format!("{family}-{red:02x}{green:02x}{blue:02x}.png"));
  image_workers::run(move || {
    if picture.is_file() {
      return Ok(picture.to_string_lossy().into_owned());
    }
    let installed = fs::read_dir(&folder)
      .into_iter()
      .flatten()
      .filter_map(Result::ok)
      .map(|entry| entry.path())
      .filter(|path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| font_file_name(name).is_ok()))
      .min_by_key(|path| path.file_stem().is_none_or(|stem| stem != "regular"));
    let file = match installed {
      Some(file) => file,
      None => {
        let file = previews.join(format!("{family}.ttf"));
        fs::create_dir_all(&previews).map_err(|e| invalid(format!("Cannot create {previews:?}: {e}")))?;
        if !fs::metadata(&file).is_ok_and(|metadata| metadata.len() > 0) {
          AiModelFile::ensure_file(&format!("the preview of {family}"), &url, None, &file, None).map_err(invalid)?;
        }
        file
      }
    };
    let font = Font::from_path(&file).map_err(|e| invalid(format!("Cannot read the font {family}: {e}")))?;
    let color = abra::abra_core::Color::from_rgba(red, green, blue, 255);
    let image: abra::abra_core::Image =
      font.text(family).with_size(TextSize::Pixels(40.0)).with_fill(abra::abra_core::Fill::solid(color)).into();
    abra::abra_core::writer(&picture).save(&image).map_err(invalid)?;
    Ok(picture.to_string_lossy().into_owned())
  })
  .await
}

/// Every family of fonts on this device, by name. Works offline.
#[uniffi::export]
pub fn downloaded_fonts() -> Vec<DownloadedFont> {
  let mut families: Vec<DownloadedFont> = fs::read_dir(fonts_dir())
    .into_iter()
    .flatten()
    .filter_map(Result::ok)
    .filter(|entry| entry.path().is_dir())
    .filter_map(|entry| {
      let family = entry.file_name().to_string_lossy().into_owned();
      let mut files: Vec<(String, u64)> = fs::read_dir(entry.path())
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|file| file.file_name().to_str().is_some_and(|name| font_file_name(name).is_ok()))
        .map(|file| (file.path().to_string_lossy().into_owned(), file.metadata().map_or(0, |metadata| metadata.len())))
        .collect();
      files.sort();
      (!files.is_empty()).then(|| DownloadedFont {
        family,
        bytes: files.iter().map(|(_, bytes)| bytes).sum(),
        files: files.into_iter().map(|(path, _)| path).collect(),
      })
    })
    .collect();
  families.sort_by(|a, b| a.family.to_lowercase().cmp(&b.family.to_lowercase()));
  families
}

/// Deletes every file of a family from this device. A family that is not there is not an error.
#[uniffi::export]
pub fn delete_font(family: String) -> Result<(), AbraError> {
  match fs::remove_dir_all(family_dir(&family)?) {
    Ok(()) => Ok(()),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(e) => Err(invalid(format!("Cannot delete the font {family}: {e}"))),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_family_or_file_name_cannot_leave_its_folder() {
    assert!(family_dir("Roboto Mono").is_ok());
    for family in ["", "  ", "..", "a/b", "a\\b", "Ünï"] {
      assert!(family_dir(family).is_err(), "{family}");
    }
    assert!(font_file_name("700italic.ttf").is_ok());
    for name in ["", "../a.ttf", "a/b.ttf", ".hidden.ttf", "a.woff2", "a.ttf.part"] {
      assert!(font_file_name(name).is_err(), "{name}");
    }
  }

  #[test]
  fn a_family_is_listed_with_its_files_until_it_is_deleted() {
    let family = format!("Test Family {}", std::process::id());
    let folder = family_dir(&family).unwrap();
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("regular.ttf"), [1, 2, 3]).unwrap();
    fs::write(folder.join("700.ttf"), [1, 2]).unwrap();
    fs::write(folder.join("700.ttf.part"), [9]).unwrap();
    let found = downloaded_fonts().into_iter().find(|font| font.family == family).expect("listed");
    assert_eq!((found.files.len(), found.bytes), (2, 5));
    delete_font(family.clone()).unwrap();
    assert!(downloaded_fonts().iter().all(|font| font.family != family));
    delete_font(family).unwrap();
  }
}
