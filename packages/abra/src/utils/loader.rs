//! Parallel image loading utilities using Rayon.

use crate::Image;
use rayon::{ThreadPoolBuilder, prelude::*};
use std::sync::Arc;

/// An image loader that can load images from file paths or existing Arc<Image> instances.\
/// If loading from file paths, images are loaded in sync or in parallel based on the method called.
///
/// If using `FromImages`, the images have then already been loaded and no further loading is necessary. They are simply wrapped in Arc<Image> and returned.
/// ```ignore
/// let paths = vec!["path/to/image1.png", "path/to/image2.jpg"];
/// let loader = ImageLoader::FromPaths(paths).load();
/// ```
/// or
/// ```ignore
/// let images = vec![
///   Image::new_from_path("path/to/image1.png"),
///   Image::new_from_path("path/to/image2.jpg")
/// ];
/// let loader = ImageLoader::FromImages(images).load();
/// ```
pub enum ImageLoader<'a> {
  /// Load images from file path strings.
  FromPaths(Vec<&'a str>),
  /// Load images from existing `Image` instances.
  FromImages(Vec<Image>),
}

impl<'a> ImageLoader<'a> {
  /// Loads images in parallel.
  /// ```ignore
  /// let paths = vec!["path/to/image1.png", "path/to/image2.jpg"];
  /// let loader = ImageLoader::FromPaths(paths).load();
  /// ```
  pub fn load(self) -> LoadedImages {
    match self {
      ImageLoader::FromPaths(paths) => LoadedImages {
        images: load_images_parallel(paths),
      },
      ImageLoader::FromImages(images) => LoadedImages {
        images: images.into_iter().map(|img| Arc::new(img)).collect(),
      },
    }
  }

  /// Load images synchronously (non-parallel).
  /// ```ignore
  /// let paths = vec!["path/to/image1.png", "path/to/image2.jpg"];
  /// let loader = ImageLoader::FromPaths(paths).load_sync();
  /// ```
  pub fn load_sync(self) -> LoadedImages {
    match self {
      ImageLoader::FromPaths(paths) => {
        let images = paths
          .into_iter()
          .map(|path| Arc::new(Image::new_from_path(path)))
          .collect();
        LoadedImages { images }
      }
      ImageLoader::FromImages(images) => LoadedImages {
        images: images.into_iter().map(|img| Arc::new(img)).collect(),
      },
    }
  }
}

impl<'a> Into<LoadedImages> for ImageLoader<'a> {
  fn into(self) -> LoadedImages {
    self.load()
  }
}

/// A trait for converting various types into Arc<Image>.
pub trait IntoImageArc {
  /// Converts the implementing type into an Arc<Image>.
  fn into_image_arc(self) -> Arc<Image>;
}

impl IntoImageArc for &str {
  fn into_image_arc(self) -> Arc<Image> {
    Arc::new(Image::new_from_path(self))
  }
}

impl IntoImageArc for Arc<Image> {
  fn into_image_arc(self) -> Arc<Image> {
    self
  }
}

impl IntoImageArc for Option<Arc<Image>> {
  fn into_image_arc(self) -> Arc<Image> {
    match self {
      Some(image) => image,
      None => Arc::new(Image::new(1, 1)), // Return a default 1x1 image if None
    }
  }
}

/// A user-friendly wrapper around loaded images.
pub struct LoadedImages {
  /// The loaded images.
  images: Vec<Arc<Image>>,
}

impl LoadedImages {
  /// Adds an image to the loaded images.
  /// - `image`: The image to add, which can be a file path or an Arc<Image>.
  /// ```ignore
  /// let mut loader = ImageLoader::FromPaths(image_paths).load();
  /// loader.add("path/to/new_image.png");
  /// ```
  pub fn add<I: IntoImageArc>(&mut self, image: I) -> &mut Self {
    self.images.push(image.into_image_arc());
    self
  }

  /// Removes and returns the first image from the loaded images.
  /// ```ignore
  /// let mut loader = ImageLoader::FromPaths(image_paths).load();
  /// if let Some(image) = loader.shift() {
  ///   // Use the image
  /// }
  /// ```
  pub fn shift(&mut self) -> Option<Arc<Image>> {
    if self.images.is_empty() {
      None
    } else {
      Some(self.images.remove(0))
    }
  }

  /// Removes and returns the last image from the loaded images.
  /// ```ignore
  /// let mut loader = ImageLoader::FromPaths(image_paths).load();
  /// if let Some(image) = loader.pop() {
  ///   // Use the image
  /// }
  /// ```
  pub fn pop(&mut self) -> Option<Arc<Image>> {
    self.images.pop()
  }

  /// Removes an image at the specified index.
  /// - `index`: The index of the image to remove.
  /// ```ignore
  /// let paths = vec!["path/to/image1.png", "path/to/image2.jpg", "path/to/image3.bmp"];
  /// let mut loader = ImageLoader::FromPaths(image_paths).load();
  /// loader.drop(2); // Removes the image at index 2
  /// ```
  pub fn drop(&mut self, index: usize) -> &mut Self {
    if index < self.images.len() {
      self.images.remove(index);
    }
    self
  }

  /// Gets an image at the specified location.
  /// - `index`: The index of the image to retrieve.
  /// ```ignore
  /// let loader = ImageLoader::FromPaths(image_paths).load();
  /// if let Some(image) = loader.at(0) {
  ///   // Use the image
  /// }
  /// ```
  pub fn at(self, index: usize) -> Option<Arc<Image>> {
    self.images.get(index).cloned()
  }

  /// Gets all loaded images.
  /// ```ignore
  /// let loader = ImageLoader::FromPaths(image_paths).load();
  /// let all_images = loader.all();
  /// ```
  pub fn all(&self) -> Vec<Arc<Image>> {
    self.images.clone()
  }

  /// Gets the first loaded image.
  /// ```ignore
  /// let loader = ImageLoader::FromPaths(image_paths).load();
  /// let first_image = loader.first();
  /// ```
  pub fn first(&self) -> Option<Arc<Image>> {
    self.images.first().cloned()
  }

  /// Gets the last loaded image.
  /// ```ignore
  /// let loader = ImageLoader::FromPaths(image_paths).load();
  /// let last_image = loader.last();
  /// ```
  pub fn last(&self) -> Option<Arc<Image>> {
    self.images.last().cloned()
  }
}

/// Loads multiple images in parallel from file paths.
/// - `paths` - A vector of file paths to load images from
/// ```ignore
/// let paths = vec!["path/to/image1.png", "path/to/image2.jpg"];
/// let images = load_images_parallel(paths);
/// ```
pub fn load_images_parallel(paths: Vec<&str>) -> Vec<Arc<Image>> {
  // Limit concurrency to avoid I/O thrash and decoder contention.
  // Empirically, 2-4 threads tends to be faster for mixed HDD/SSD workloads and single-threaded decoders.
  let threads = paths.len().clamp(1, 4);
  load_images_parallel_with_threads(paths, threads)
}

/// Loads multiple images in parallel with a bounded number of threads.
/// This often outperforms unbounded parallelism for file I/O heavy workloads on HDDs/SSDs
/// and when decoders are single-threaded.
pub fn load_images_parallel_with_threads(paths: Vec<&str>, threads: usize) -> Vec<Arc<Image>> {
  let pool = ThreadPoolBuilder::new()
    .num_threads(threads.clamp(1, 4))
    .build()
    .expect("Failed to build rayon thread pool for image loading");

  pool.install(|| {
    paths
      .into_par_iter()
      .map(|path| Arc::new(Image::new_from_path(path)))
      .collect()
  })
}
