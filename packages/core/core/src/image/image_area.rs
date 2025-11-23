//! This implements methods related to image areas.

use crate::{Area, Image};

use rayon::prelude::*;

impl Image {
  /// Retrieves RGBA pixel data within the specified area.
  /// Note: Areas can be any shape; this function extracts pixels within that shape.
  /// - `p_area`: The area from which to retrieve pixel data.
  pub fn get_rgba_in_area(&self, p_area: &Area) -> Vec<u8> {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = p_area.bounds::<i32>();
    let (width, height) = self.size().to_tuple::<i32>();

    // Clamp bounds to image size to avoid unnecessary checks in the inner loop.
    if min_x < 0 {
      min_x = 0;
    }
    if min_y < 0 {
      min_y = 0;
    }
    if max_x > width {
      max_x = width;
    }
    if max_y > height {
      max_y = height;
    }

    // Early return if clipped area is empty.
    if min_x >= max_x || min_y >= max_y {
      return Vec::new();
    }

    let row_stride = (width * 4) as usize;
    let start_byte = min_y as usize * row_stride;
    let end_byte = max_y as usize * row_stride;

    // Work only on the vertical slice that intersects the area.
    // Obtain a slice of the underlying colors buffer to avoid cloning the whole image.
    let rgba = self.rgba_slice();
    let rows_slice = &rgba[start_byte..end_byte];

    // Iterate rows in parallel, producing per-row buffers, then append.
    rows_slice
      .par_chunks_exact(row_stride)
      .enumerate()
      .map(|(i, row)| {
        let y = (min_y + i as i32) as f32 + 0.5;
        let x_start = min_x as i32;
        let x_end = max_x as i32;
        let mut row_pixels: Vec<u8> = Vec::with_capacity(((x_end - x_start) as usize) * 4);

        // Avoid repeated casts inside the loop by preparing the starting f32 x coordinate.
        let mut xf = x_start as f32 + 0.5;

        for x in x_start..x_end {
          if p_area.contains((xf, y)) {
            let idx = (x * 4) as usize;
            row_pixels.extend_from_slice(&row[idx..idx + 4]);
          }
          xf += 1.0;
        }

        row_pixels
      })
      // Use reduce to avoid creating an intermediate Vec<Vec<u8>>; append is efficient.
      .reduce(
        || Vec::new(),
        |mut acc, mut v| {
          acc.append(&mut v);
          acc
        },
      )
  }
  /// Gets the pixels of the image. If an area is provided, only pixels within that area are returned.
  /// If no area is provided, the entire image's pixels are returned.
  /// - `p_area`: Optional area to restrict pixel retrieval.
  pub fn get_selective_rgba(&self, p_area: Option<&Area>) -> Vec<u8> {
    match p_area {
      Some(a) => self.get_rgba_in_area(a),
      None => self.rgba_slice().to_vec(),
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::{Area, Image};

  #[test]
  fn get_rgba_in_area_rect() {
    // Create a 4x3 image where each pixel holds (x, y, 0, 255)
    let mut img = Image::new(4, 3);
    for y in 0..3 {
      for x in 0..4 {
        let r = x as u8;
        let g = y as u8;
        img.set_pixel(x as u32, y as u32, (r, g, 0, 255));
      }
    }

    let area = Area::rect((1.0, 1.0), (2.0, 2.0)); // x:[1,3), y:[1,3)
    let bytes = img.get_rgba_in_area(&area);

    // Expect 2x2 pixels = 4 pixels, RGBA each => 16 bytes
    assert_eq!(bytes.len(), 16);
    // Expected order: row y=1 (x=1,2), row y=2 (x=1,2)
    let expected: Vec<u8> = vec![
      1, 1, 0, 255, // (1,1)
      2, 1, 0, 255, // (2,1)
      1, 2, 0, 255, // (1,2)
      2, 2, 0, 255, // (2,2)
    ];
    assert_eq!(bytes, expected);
  }

  #[test]
  fn get_rgba_in_area_full_image() {
    let mut img = Image::new(3, 2);
    for y in 0..2 {
      for x in 0..3 {
        img.set_pixel(x as u32, y as u32, (x as u8, y as u8, 0, 255));
      }
    }
    let area = Area::new_from_image(&img);
    let bytes = img.get_rgba_in_area(&area);
    assert_eq!(bytes.len(), 3 * 2 * 4);
  }
}
