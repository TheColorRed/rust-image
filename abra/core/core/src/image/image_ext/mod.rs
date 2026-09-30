//! Methods `abra-core` adds to [`Image`]: file I/O, size, and reading pixels inside an area.

mod image_ref;

pub use image_ref::*;

use std::path::Path;

use rayon::prelude::*;

use crate::fs::WriterOptions;
use crate::{Area, Image, PointF, Rect, Size, polygon_contains, reader, writer};

/// Methods `abra-core` adds to [`Image`].
pub trait ImageExt {
  /// Reads an image, choosing the decoder from its file extension.
  fn read(p_file: impl AsRef<Path>) -> Result<Image, String>;
  /// Writes the image, choosing the encoder from its file extension.
  fn write(&self, p_file: impl AsRef<Path>, p_options: impl Into<Option<WriterOptions>>) -> Result<(), String>;
  /// The width and height of the image.
  fn size(&self) -> Size;
  /// The RGBA bytes of the pixels whose centers are inside the area, row by row. The area can be any shape.
  fn rgba_in_area(&self, p_area: &Area) -> Vec<u8>;
}

impl ImageExt for Image {
  fn read(p_file: impl AsRef<Path>) -> Result<Image, String> {
    reader(p_file).load()
  }

  fn write(&self, p_file: impl AsRef<Path>, p_options: impl Into<Option<WriterOptions>>) -> Result<(), String> {
    writer(p_file).with_options(p_options).save(self)
  }

  fn size(&self) -> Size {
    let (width, height) = self.dimensions::<u32>();
    Size::new(width, height)
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
}
