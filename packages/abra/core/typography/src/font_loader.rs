use abra_core::LoadedCollection;
use rayon::{ThreadPoolBuilder, prelude::*};
use std::{path::Path, sync::Arc};

/// The execution strategy used when loading multiple fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontLoadMode {
  /// Load fonts one at a time on the calling thread.
  Sync,
  /// Load fonts concurrently using at most `threads` worker threads.
  Parallel { threads: usize },
}

/// A collection of fonts loaded from a [`FontLoader`]. Its methods come from [`LoadedCollection`].
pub struct LoadedFonts {
  fonts: Vec<Arc<crate::Font>>,
}

impl LoadedCollection for LoadedFonts {
  type Item = Arc<crate::Font>;

  fn items(&self) -> &[Arc<crate::Font>] {
    &self.fonts
  }

  fn items_mut(&mut self) -> &mut Vec<Arc<crate::Font>> {
    &mut self.fonts
  }
}

/// A trait for converting font values into shared font instances.
pub trait IntoFontArc {
  /// Converts the implementing type into an `Arc<Font>`.
  fn into_font_arc(self) -> Arc<crate::Font>;
}

impl IntoFontArc for crate::Font {
  fn into_font_arc(self) -> Arc<crate::Font> {
    Arc::new(self)
  }
}

impl IntoFontArc for Arc<crate::Font> {
  fn into_font_arc(self) -> Arc<crate::Font> {
    self
  }
}

pub struct FontLoader {
  fonts: Vec<String>,
}

impl FontLoader {
  /// Gets a list of system-installed font file paths.
  ///
  /// Returns a `FontLoader` containing the discovered font paths.
  ///
  /// # Example
  /// ```ignore
  /// let system_fonts = FontLoader::find_system_fonts();
  /// ```
  pub fn find_system_fonts() -> FontLoader {
    let mut fonts = Vec::new();
    #[cfg(target_os = "windows")]
    {
      fonts.extend(abra_core::get_paths_from_glob("C:/Windows/Fonts/**/*.ttf"));
    }
    #[cfg(target_os = "macos")]
    {
      fonts.extend(abra_core::get_paths_from_glob("/System/Library/Fonts/**/*.ttf"));
    }
    #[cfg(target_os = "linux")]
    {
      fonts.extend(abra_core::get_paths_from_glob("/usr/share/fonts/**/*.ttf"));
    }
    #[cfg(target_os = "android")]
    {
      fonts.extend(abra_core::get_paths_from_glob("/system/fonts/**/*.ttf"));
    }
    FontLoader { fonts }
  }

  /// Finds font file paths from globs or explicit paths.
  /// ```ignore
  /// let paths = FontLoader::find_fonts("assets/fonts/*.ttf");
  /// let paths = FontLoader::find_fonts(vec!["assets/fonts/*.ttf"]);
  /// ```
  pub fn find_fonts(p_patterns: impl abra_core::IntoGlobPatterns) -> FontLoader {
    FontLoader {
      fonts: abra_core::get_paths_from_glob(p_patterns),
    }
  }

  /// Finds a single font from an exact file path.
  ///
  /// This mirrors the loader's constructor style: given a concrete font path, it
  /// returns a one-element loader containing that exact file. It does not accept
  /// a glob pattern.
  pub fn load_font(p_path: &str) -> Option<crate::Font> {
    crate::Font::from_path(p_path).ok()
  }

  /// Loads every discovered font using the requested execution strategy.
  pub fn load_all(&self, p_mode: FontLoadMode) -> LoadedFonts {
    LoadedFonts {
      fonts: load_fonts(self.fonts.clone(), p_mode),
    }
  }

  /// Adds additional font file paths to the `FontLoader`.
  ///
  /// # Example
  /// ```ignore
  /// let mut system_fonts = FontLoader::find_system_fonts();
  /// system_fonts.add_fonts("assets/fonts/CustomFont.ttf");
  /// ```
  pub fn add_fonts(&mut self, p_patterns: impl abra_core::IntoGlobPatterns) {
    self.fonts.extend(FontLoader::find_fonts(p_patterns).fonts);
  }

  /// Returns the list of font file paths contained in this `FontLoader`.
  pub fn fonts(&self) -> &Vec<String> {
    &self.fonts
  }

  /// Loads the first font matching `p_name` and parses it into a [`crate::Font`].
  ///
  /// This is a convenience wrapper around the loader's discovered font list and
  /// falls back to a direct file-path load when the argument is already a path.
  pub fn load(&self, p_name: &str) -> Option<crate::Font> {
    let path = if Path::new(p_name).is_file() {
      p_name.to_string()
    } else {
      self
        .fonts
        .iter()
        .find(|path| {
          let lower_name = p_name.to_lowercase();
          path == &p_name || path.to_lowercase() == lower_name || path.to_lowercase().contains(&lower_name)
        })?
        .clone()
    };
    crate::Font::from_path(&path).ok()
  }

  /// Returns the font file path at the specified index, if it exists.
  ///
  /// # Example
  /// ```ignore
  /// let system_fonts = FontLoader::load_system_fonts();
  /// if let Some(font) = system_fonts.at(0) {
  ///     println!("Found font: {}", font);
  /// }
  /// ```
  pub fn at(&self, p_index: usize) -> Option<&String> {
    self.fonts.get(p_index)
  }
}

/// Loads multiple fonts from file paths using a single execution strategy.
pub fn load_fonts(p_paths: Vec<impl Into<String>>, p_mode: FontLoadMode) -> Vec<Arc<crate::Font>> {
  let p_paths: Vec<String> = p_paths.into_iter().map(Into::into).collect();
  if matches!(p_mode, FontLoadMode::Sync) {
    return p_paths.into_iter().filter_map(|path| crate::Font::from_path(path).ok().map(Arc::new)).collect();
  }

  let FontLoadMode::Parallel { threads } = p_mode else { unreachable!() };
  let pool = ThreadPoolBuilder::new()
    .num_threads(threads.clamp(1, 4))
    .build()
    .expect("Failed to build rayon thread pool for font loading");

  pool.install(|| p_paths.into_par_iter().filter_map(|path| crate::Font::from_path(path).ok().map(Arc::new)).collect())
}
