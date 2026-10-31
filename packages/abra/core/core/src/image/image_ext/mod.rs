//! Methods `abra-core` adds to [`Image`]: file I/O, size, and reading pixels inside an area.

mod image_ref;

pub use image_ref::*;

use std::path::Path;

use rayon::prelude::*;

use crate::fs::WriterOptions;
use crate::image::levels::{Levels, copy_clamped};
use crate::{Area, Image, PointF, Rect, Size, polygon_contains, reader, writer};

/// Methods `abra-core` adds to [`Image`].
pub trait ImageExt {
  /// Reads an image, choosing the decoder from its file extension.
  fn read(p_file: impl AsRef<Path>) -> Result<Image, String>;
  /// Reads an image on a rayon thread and gives back a future for it, so the caller keeps running while the file is decoded. The result
  /// is that of [`read`](ImageExt::read), and the read starts when this is called, not when the future is first polled. The future
  /// waits on a channel, so any executor can run it. Dropping it does not stop the decode: it finishes and its result is dropped.
  fn read_async(p_file: impl AsRef<Path>) -> impl std::future::Future<Output = Result<Image, String>> + Send;
  /// Writes the image, choosing the encoder from its file extension.
  fn write(&self, p_file: impl AsRef<Path>, p_options: impl Into<Option<WriterOptions>>) -> Result<(), String>;
  /// The width and height of the image.
  fn size(&self) -> Size;
  /// The RGBA bytes of the pixels whose centers are inside the area, row by row. The area can be any shape.
  fn rgba_in_area(&self, p_area: &Area) -> Vec<u8>;
  /// The rectangle `p_width` x `p_height` whose top left is at `(p_left, p_top)` in the image at `p_level`, as RGBA. Level 0
  /// is the image, and each level above it is the one before halved (see [`level_size`](crate::image::levels::level_size)).
  /// A pixel past an edge is the pixel at that edge, so a rectangle can reach out of the image.
  ///
  /// Only what is asked for is made: the zoomed-out levels the first time they are needed, and an image that is still a
  /// recipe, when it can, only the part asked for. Whatever was made is kept with the image until its pixels change.
  fn region(&self, p_level: u32, p_left: i32, p_top: i32, p_width: u32, p_height: u32) -> Vec<u8>;
}

impl ImageExt for Image {
  fn read(p_file: impl AsRef<Path>) -> Result<Image, String> {
    reader(p_file).load()
  }

  fn read_async(p_file: impl AsRef<Path>) -> impl std::future::Future<Output = Result<Image, String>> + Send {
    let file = p_file.as_ref().to_path_buf();
    let (sender, receiver) = futures::channel::oneshot::channel();
    rayon::spawn(move || {
      // A caller that stopped waiting no longer needs the image.
      let _ = sender.send(<Image as ImageExt>::read(file));
    });
    async move { receiver.await.map_err(|_| "The image read stopped before it finished".to_owned())? }
  }

  fn write(&self, p_file: impl AsRef<Path>, p_options: impl Into<Option<WriterOptions>>) -> Result<(), String> {
    writer(p_file).with_options(p_options).save(self)
  }

  fn size(&self) -> Size {
    let (width, height) = self.dimensions::<u32>();
    Size::new(width, height)
  }

  fn region(&self, p_level: u32, p_left: i32, p_top: i32, p_width: u32, p_height: u32) -> Vec<u8> {
    if let Some(pixels) = self.deferred().and_then(|recipe| recipe.region(p_level, p_left, p_top, p_width, p_height)) {
      return pixels;
    }
    let (width, height) = self.dimensions::<u32>();
    if p_level == 0 {
      return copy_clamped(self.rgba(), width, height, p_left, p_top, p_width, p_height);
    }
    let level = self.derived(|_| Levels::default()).level(self, p_level);
    copy_clamped(&level.pixels, level.width, level.height, p_left, p_top, p_width, p_height)
  }

  fn rgba_in_area(&self, p_area: &Area) -> Vec<u8> {
    let (width, height) = self.dimensions::<u32>();
    let visible = p_area.bounds().intersect(Rect::new((0, 0), (width, height)));
    if visible.is_empty() {
      return Vec::new();
    }
    let (min_x, min_y, max_x, max_y) = visible.edges::<usize>();
    // Flatten once; every pixel is tested against the same polygon.
    let outline = p_area.flatten(0.5);
    let row_stride = width as usize * 4;

    self.rgba()[min_y * row_stride..max_y * row_stride]
      .par_chunks_exact(row_stride)
      .enumerate()
      .map(|(i, row)| {
        let y = (min_y + i) as f32 + 0.5;
        let mut row_pixels = Vec::with_capacity((max_x - min_x) * 4);
        for x in min_x..max_x {
          if polygon_contains(&outline, PointF::new(x as f32 + 0.5, y)) {
            row_pixels.extend_from_slice(&row[x * 4..x * 4 + 4]);
          }
        }
        row_pixels
      })
      .reduce(Vec::new, |mut acc, mut row| {
        acc.append(&mut row);
        acc
      })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn coordinate_image(p_width: u32, p_height: u32) -> Image {
    let mut img = Image::new(p_width, p_height);
    for y in 0..p_height {
      for x in 0..p_width {
        img.set_pixel(x, y, (x as u8, y as u8, 0, 255));
      }
    }
    img
  }

  #[test]
  fn read_async_gives_the_image_that_read_gives_and_the_same_error_for_a_missing_file() {
    let path = std::env::temp_dir().join(format!("abra-read-async-{}.png", std::process::id()));
    let file = path.to_string_lossy().into_owned();
    let written = coordinate_image(5, 3);
    written.write(&file, None).unwrap();

    let read = futures::executor::block_on(Image::read_async(&file)).expect("a written picture reads back");
    assert_eq!(read.dimensions::<u32>(), (5, 3));
    assert_eq!(read.rgba(), Image::read(&file).unwrap().rgba());
    let _ = std::fs::remove_file(path);

    let missing = "missing-read-async-picture.png";
    let error = futures::executor::block_on(Image::read_async(missing)).err().expect("a missing file is an error");
    assert_eq!(error, Image::read(missing).err().unwrap());
  }

  #[test]
  fn rgba_in_area_rect() {
    let img = coordinate_image(4, 3);
    let bytes = img.rgba_in_area(&Area::rect((1.0, 1.0), (2.0, 2.0)));
    // Row y=1 (x=1,2), then row y=2 (x=1,2).
    assert_eq!(bytes, vec![1, 1, 0, 255, 2, 1, 0, 255, 1, 2, 0, 255, 2, 2, 0, 255]);
  }

  #[test]
  fn rgba_in_area_full_image() {
    let img = coordinate_image(3, 2);
    assert_eq!(img.rgba_in_area(&Area::new_from_image(&img)).len(), 3 * 2 * 4);
    assert_eq!(img.size(), Size::new(3, 2));
  }
  #[test]
  fn a_region_is_the_rectangle_of_the_image_at_that_level_and_reaches_past_the_edge() {
    let img = coordinate_image(4, 4);
    assert_eq!(img.region(0, 1, 2, 2, 1), vec![1, 2, 0, 255, 2, 2, 0, 255]);
    // Level 1 is 2 x 2: the mean of each block of four, rounded.
    assert_eq!(img.region(1, 0, 0, 2, 2), vec![1, 1, 0, 255, 3, 1, 0, 255, 1, 3, 0, 255, 3, 3, 0, 255]);
    // Level 2 is one pixel, and a rectangle past it is that pixel.
    assert_eq!(img.region(2, -1, -1, 2, 2), vec![2, 2, 0, 255].repeat(4));
  }

  #[test]
  fn the_levels_are_kept_until_the_pixels_change_and_a_clone_shares_them() {
    let mut img = coordinate_image(4, 4);
    let clone = img.clone();
    assert_eq!(img.region(2, 0, 0, 1, 1), vec![2, 2, 0, 255]);
    let levels = img.derived(|_| Levels::default());
    assert!(std::sync::Arc::ptr_eq(&levels, &clone.derived(|_| Levels::default())), "the clone shares what was made");
    img.set_pixel(0, 0, (255, 255, 255, 255));
    assert!(!std::sync::Arc::ptr_eq(&levels, &img.derived(|_| Levels::default())), "a change starts the levels again");
    assert_eq!(img.region(2, 0, 0, 1, 1)[0], 18, "the new level has the changed pixel in its mean");
    assert_eq!(clone.region(2, 0, 0, 1, 1), vec![2, 2, 0, 255], "the clone did not change");
  }
}
