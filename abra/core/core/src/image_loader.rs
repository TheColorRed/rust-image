//! Parallel image loading utilities using Rayon.

use crate::fs::path::{get_paths_from_folders as get_paths_from_folders_filtered, get_paths_from_glob};
use crate::{Image, ImageExt, LoadedCollection};
use rayon::{ThreadPoolBuilder, prelude::*};
use std::sync::Arc;

/// An image loader that can load images from file paths or existing Arc<Image> instances.
pub enum ImageLoader<'a> {
  /// Load images from file path strings.
  FromPaths(Vec<&'a str>),
  /// Load images from existing `Image` instances.
  FromImages(Vec<Image>),
  /// Load images from a list of glob patterns.
  FromGlob(Vec<&'a str>),
  /// Load images from a list of folders.
  FromFolders(Vec<&'a str>, bool),
}

/// The execution strategy used by [`ImageLoader`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadMode {
  /// Load images one at a time on the calling thread.
  Sync,
  /// Load images concurrently using at most `threads` worker threads.
  Parallel { threads: usize },
}

impl<'a> ImageLoader<'a> {
  /// Loads images using the requested execution strategy.
  pub fn load(self, p_mode: LoadMode) -> LoadedImages {
    match self {
      ImageLoader::FromPaths(paths) => LoadedImages {
        images: load_images(paths, p_mode),
      },
      ImageLoader::FromImages(images) => LoadedImages {
        images: images.into_iter().map(Arc::new).collect(),
      },
      ImageLoader::FromFolders(folders, recursive) => {
        let all_paths = get_paths_from_folders(folders, recursive);
        LoadedImages {
          images: load_images(all_paths, p_mode),
        }
      }
      ImageLoader::FromGlob(patterns) => {
        let all_paths = get_paths_from_glob(patterns);
        LoadedImages {
          images: load_images(all_paths, p_mode),
        }
      }
    }
  }
}

impl<'a> From<ImageLoader<'a>> for LoadedImages {
  fn from(p_loader: ImageLoader<'a>) -> Self {
    p_loader.load(LoadMode::Parallel { threads: 4 })
  }
}

/// A user-friendly wrapper around loaded images. Its methods come from [`LoadedCollection`].
#[derive(Clone, Default)]
pub struct LoadedImages {
  images: Vec<Arc<Image>>,
}

impl LoadedCollection for LoadedImages {
  type Item = Arc<Image>;

  fn items(&self) -> &[Arc<Image>] {
    &self.images
  }

  fn items_mut(&mut self) -> &mut Vec<Arc<Image>> {
    &mut self.images
  }
}

/// Loads multiple images from file paths using a single execution strategy.
pub fn load_images(p_paths: Vec<impl Into<String>>, p_mode: LoadMode) -> Vec<Arc<Image>> {
  let p_paths: Vec<String> = p_paths.into_iter().map(Into::into).collect();
  if matches!(p_mode, LoadMode::Sync) {
    return p_paths.into_iter().map(|path| Arc::new(Image::read(path).expect("Failed to load image"))).collect();
  }

  let LoadMode::Parallel { threads } = p_mode else { unreachable!() };
  let pool = ThreadPoolBuilder::new()
    .num_threads(threads.clamp(1, 4))
    .build()
    .expect("Failed to build rayon thread pool for image loading");
  pool.install(|| {
    p_paths.into_par_iter().map(|path| Arc::new(Image::read(path).expect("Failed to load image"))).collect()
  })
}

fn get_paths_from_folders(p_folders: Vec<impl Into<String>>, p_recursive: bool) -> Vec<String> {
  get_paths_from_folders_filtered(p_folders, p_recursive, |extension| is_supported_image_extension(extension))
}

pub fn is_supported_image_extension(p_extension: impl Into<String>) -> bool {
  crate::fs::ImageFormat::from_extension(&p_extension.into()).is_some()
}
