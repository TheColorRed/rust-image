//! Parallel image loading utilities using Rayon.

use crate::fs::path::{get_paths_from_folders as get_paths_from_folders_filtered, get_paths_from_glob};
use crate::{Image, LoadedCollection, image::image_ext::*};
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
        println!("Found {} images in folders.", all_paths.len());
        LoadedImages {
          images: load_images(all_paths, p_mode),
        }
      }
      ImageLoader::FromGlob(patterns) => {
        let all_paths = get_paths_from_glob(patterns);
        println!("Found {} images from glob patterns.", all_paths.len());
        LoadedImages {
          images: load_images(all_paths, p_mode),
        }
      }
    }
  }
}

impl<'a> Into<LoadedImages> for ImageLoader<'a> {
  fn into(self) -> LoadedImages {
    self.load(LoadMode::Parallel { threads: 4 })
  }
}

/// A trait for converting various types into Arc<Image>.
pub trait IntoImageArc {
  /// Converts the implementing type into an Arc<Image>.
  fn into_image_arc(self) -> Arc<Image>;
}

impl IntoImageArc for &str {
  fn into_image_arc(self) -> Arc<Image> {
    Arc::new(Image::read(self).expect("Failed to load image"))
  }
}

impl IntoImageArc for Arc<Image> {
  fn into_image_arc(self) -> Arc<Image> {
    self
  }
}

impl IntoImageArc for Option<Arc<Image>> {
  fn into_image_arc(self) -> Arc<Image> {
    self.unwrap_or_else(|| Arc::new(Image::new(1, 1)))
  }
}

/// A user-friendly wrapper around loaded images.
pub struct LoadedImages {
  images: Vec<Arc<Image>>,
}

impl LoadedImages {
  /// Adds an image to the loaded images.
  pub fn add<I: IntoImageArc>(&mut self, p_image: I) -> &mut Self {
    self.images.push(p_image.into_image_arc());
    self
  }

  /// Removes and returns the first image from the loaded images.
  pub fn shift(&mut self) -> Option<Arc<Image>> {
    if self.images.is_empty() { None } else { Some(self.images.remove(0)) }
  }

  /// Removes and returns the last image from the loaded images.
  pub fn pop(&mut self) -> Option<Arc<Image>> {
    self.images.pop()
  }

  /// Removes an image at the specified index.
  pub fn drop(&mut self, p_index: usize) -> &mut Self {
    if p_index < self.images.len() {
      self.images.remove(p_index);
    }
    self
  }

  /// Gets an image at the specified location.
  pub fn at(&self, p_index: impl Into<u32>) -> Option<Arc<Image>> {
    self.images.get(p_index.into() as usize).cloned()
  }

  /// Gets all loaded images.
  pub fn all(&self) -> Vec<Arc<Image>> {
    self.images.clone()
  }

  /// Gets the first loaded image.
  pub fn first(&self) -> Option<Arc<Image>> {
    self.images.first().cloned()
  }

  /// Gets the last loaded image.
  pub fn last(&self) -> Option<Arc<Image>> {
    self.images.last().cloned()
  }
}

impl LoadedCollection for LoadedImages {
  type Item = Arc<Image>;

  fn add(&mut self, p_item: Self::Item) -> &mut Self {
    self.images.push(p_item);
    self
  }

  fn shift(&mut self) -> Option<Self::Item> {
    if self.images.is_empty() { None } else { Some(self.images.remove(0)) }
  }
  fn pop(&mut self) -> Option<Self::Item> {
    self.images.pop()
  }
  fn drop(&mut self, p_index: usize) -> &mut Self {
    if p_index < self.images.len() {
      self.images.remove(p_index);
    }
    self
  }
  fn at(&self, p_index: usize) -> Option<Self::Item> {
    self.images.get(p_index).cloned()
  }
  fn all(&self) -> Vec<Self::Item> {
    self.images.clone()
  }
  fn first(&self) -> Option<Self::Item> {
    self.images.first().cloned()
  }
  fn last(&self) -> Option<Self::Item> {
    self.images.last().cloned()
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
