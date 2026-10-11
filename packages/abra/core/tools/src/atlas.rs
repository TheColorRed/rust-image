use abra_core::Image;
use drawing::{FillTarget, fill};

use crate::Tool;

/// Represents the dimension configuration of an atlas.
/// If `Rows` is used, the atlas will have a fixed number of rows and the columns will be determined accordingly.
/// If `Cols` is used, the atlas will have a fixed number of columns and the rows will be determined accordingly.
/// If `Fixed` is used, its dimensions are specified as `(cols, rows)`. Images beyond the grid capacity are omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlasDimension {
  Rows(usize),
  Cols(usize),
  /// Fixed dimensions in `(cols, rows)` or `(x, y)` order.
  Fixed(usize, usize),
}

impl From<usize> for AtlasDimension {
  fn from(p_cols: usize) -> Self {
    Self::Cols(p_cols)
  }
}

/// An atlas is a collection of images arranged in a grid.
/// The grid is built from left to right, top to bottom, according to the specified number of columns and images added.
#[derive(Debug, Clone)]
pub struct Atlas {
  dimension: AtlasDimension,
  trim: bool,
  images: Vec<Image>,
}

impl Atlas {
  /// Sets how images are arranged in the atlas.
  pub fn with_dimensions(mut self, p_dimension: impl Into<AtlasDimension>) -> Self {
    self.dimension = p_dimension.into();
    self
  }
  /// Sets the number of columns. Unused cells remain blank unless trimming is enabled.
  /// `0` places all images in one row.
  pub fn with_cols(self, p_cols: usize) -> Self {
    self.with_dimensions(AtlasDimension::Cols(p_cols))
  }
  /// Sets the number of rows. The number of columns is calculated from the image count.
  /// Unused cells remain blank unless trimming is enabled.
  pub fn with_rows(self, p_rows: usize) -> Self {
    self.with_dimensions(AtlasDimension::Rows(p_rows))
  }
  /// Sets a fixed grid size as `(cols, rows)`. Images beyond the grid capacity are omitted.
  pub fn with_fixed_dimensions(self, p_cols: usize, p_rows: usize) -> Self {
    self.with_dimensions(AtlasDimension::Fixed(p_cols, p_rows))
  }
  /// Trims unused rows and columns from the configured atlas grid.
  pub fn with_trim(mut self, p_trim: bool) -> Self {
    self.trim = p_trim;
    self
  }
  /// Adds an image to the end of the atlas.
  pub fn append(&mut self, p_image: Image) -> &mut Self {
    self.images.push(p_image);
    self
  }
  /// Adds an image at the specified index in the atlas.
  pub fn set(&mut self, p_index: usize, p_image: Image) -> &mut Self {
    if p_index < self.images.len() {
      self.images[p_index] = p_image;
    }
    self
  }
  /// Adds an image to the beginning of the atlas.
  pub fn prepend(&mut self, p_image: Image) -> &mut Self {
    self.images.insert(0, p_image);
    self
  }
  /// Removes the image at the specified index from the atlas.
  pub fn remove(&mut self, p_index: usize) -> &mut Self {
    if p_index < self.images.len() {
      self.images.remove(p_index);
    }
    self
  }
}

impl FillTarget for Atlas {
  fn draw_fill(&mut self, p_image: &Image, _p_position: (i32, i32)) {
    self.append(p_image.clone());
  }
}

impl Tool for Atlas {
  fn create(&self) -> Image {
    if self.images.is_empty() {
      return Image::new(0, 0);
    }

    let (rows, cols, image_count) = match self.dimension {
      AtlasDimension::Rows(rows) => {
        let rows = rows.max(1);
        let cols = self.images.len().div_ceil(rows);
        (rows, cols, self.images.len())
      }
      AtlasDimension::Cols(cols) => {
        let cols = if cols == 0 { self.images.len() } else { cols };
        (self.images.len().div_ceil(cols), cols, self.images.len())
      }
      AtlasDimension::Fixed(cols, rows) => {
        let rows = rows.max(1);
        let cols = cols.max(1);
        (rows, cols, self.images.len().min(rows.saturating_mul(cols)))
      }
    };

    let images = &self.images[..image_count];
    let (cell_width, cell_height) = images.iter().fold((0u32, 0u32), |(max_width, max_height), img| {
      let (width, height) = img.dimensions::<u32>();
      (max_width.max(width), max_height.max(height))
    });
    let used_rows = image_count.div_ceil(cols);
    let used_cols = image_count.min(cols);
    let grid_rows = if self.trim { used_rows } else { rows };
    let grid_cols = if self.trim { used_cols } else { cols };
    let mut atlas =
      Image::new(cell_width.saturating_mul(grid_cols as u32), cell_height.saturating_mul(grid_rows as u32));

    for (index, img) in images.iter().enumerate() {
      let col = index % cols;
      let row = index / cols;
      fill(img, img.clone())
        .with_position(((col as u32 * cell_width) as f32, (row as u32 * cell_height) as f32))
        .apply(&mut atlas);
    }

    atlas
  }
}

/// Creates a new atlas from the supplied images. Defaults to one row containing all images.
pub fn atlas_with(p_images: Vec<Image>) -> Atlas {
  Atlas {
    dimension: AtlasDimension::Cols(0),
    trim: false,
    images: p_images,
  }
}
/// Creates an empty atlas with no images.
pub fn atlas() -> Atlas {
  Atlas {
    dimension: AtlasDimension::Cols(0),
    trim: false,
    images: Vec::new(),
  }
}

pub fn atlas_with_size(p_size: usize) -> Atlas {
  Atlas {
    dimension: AtlasDimension::Cols(0),
    trim: false,
    images: Vec::with_capacity(p_size),
  }
}

impl Into<Image> for Atlas {
  fn into(self) -> Image {
    self.create()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Color;

  fn images() -> Vec<Image> {
    vec![
      Image::new_from_color(2, 1, Color::from_rgba(255, 0, 0, 255)),
      Image::new_from_color(1, 2, Color::from_rgba(0, 255, 0, 255)),
      Image::new_from_color(3, 1, Color::from_rgba(0, 0, 255, 255)),
      Image::new_from_color(1, 1, Color::from_rgba(255, 255, 0, 255)),
    ]
  }

  #[test]
  fn limits_each_row_to_the_configured_number_of_columns() {
    let atlas = atlas_with(images()).with_cols(2).create();

    assert_eq!(atlas.dimensions::<u32>(), (6, 4));
    assert_eq!(atlas.get_pixel(0, 0), Some((255, 0, 0, 255)));
    assert_eq!(atlas.get_pixel(3, 0), Some((0, 255, 0, 255)));
    assert_eq!(atlas.get_pixel(0, 2), Some((0, 0, 255, 255)));
    assert_eq!(atlas.get_pixel(3, 2), Some((255, 255, 0, 255)));
  }

  #[test]
  fn rows_setting_calculates_columns_from_the_image_count() {
    let atlas = atlas_with(images()).with_rows(2).create();

    assert_eq!(atlas.dimensions::<u32>(), (6, 4));
    assert_eq!(atlas.get_pixel(0, 2), Some((0, 0, 255, 255)));
    assert_eq!(atlas.get_pixel(3, 2), Some((255, 255, 0, 255)));
  }

  #[test]
  fn trim_removes_unused_rows_and_columns_from_rows_and_cols_layouts() {
    let rows = atlas_with(images()).with_rows(5).create();
    let trimmed_rows = atlas_with(images()).with_rows(5).with_trim(true).create();
    let cols = atlas_with(images().into_iter().take(3).collect()).with_cols(4).create();
    let trimmed_cols = atlas_with(images().into_iter().take(3).collect()).with_cols(4).with_trim(true).create();

    assert_eq!(rows.dimensions::<u32>(), (3, 10));
    assert_eq!(trimmed_rows.dimensions::<u32>(), (3, 8));
    assert_eq!(cols.dimensions::<u32>(), (12, 2));
    assert_eq!(trimmed_cols.dimensions::<u32>(), (9, 2));
  }

  #[test]
  fn fixed_dimensions_omit_images_beyond_the_grid_capacity() {
    let atlas = atlas_with(images()).with_fixed_dimensions(2, 1).create();

    assert_eq!(atlas.dimensions::<u32>(), (4, 2));
    assert_eq!(atlas.get_pixel(0, 0), Some((255, 0, 0, 255)));
    assert_eq!(atlas.get_pixel(2, 1), Some((0, 255, 0, 255)));
  }

  #[test]
  fn fixed_dimensions_reserve_the_entire_grid_by_default() {
    let atlas = atlas_with(images()).with_fixed_dimensions(3, 3).create();

    assert_eq!(atlas.dimensions::<u32>(), (9, 6));
    assert_eq!(atlas.get_pixel(0, 0), Some((255, 0, 0, 255)));
    assert_eq!(atlas.get_pixel(3, 0), Some((0, 255, 0, 255)));
    assert_eq!(atlas.get_pixel(6, 0), Some((0, 0, 255, 255)));
    assert_eq!(atlas.get_pixel(0, 2), Some((255, 255, 0, 255)));
  }

  #[test]
  fn trim_removes_unused_rows_and_columns_from_fixed_dimensions() {
    let atlas = atlas_with(images().into_iter().take(2).collect()).with_fixed_dimensions(3, 3).with_trim(true).create();

    assert_eq!(atlas.dimensions::<u32>(), (4, 2));
    assert_eq!(atlas.get_pixel(0, 0), Some((255, 0, 0, 255)));
    assert_eq!(atlas.get_pixel(2, 0), Some((0, 255, 0, 255)));
  }

  #[test]
  fn zero_columns_places_all_images_in_one_row() {
    let atlas = atlas_with(images()).with_cols(0).create();

    assert_eq!(atlas.dimensions::<u32>(), (12, 2));
    assert_eq!(atlas.get_pixel(0, 0), Some((255, 0, 0, 255)));
    assert_eq!(atlas.get_pixel(3, 1), Some((0, 255, 0, 255)));
    assert_eq!(atlas.get_pixel(6, 0), Some((0, 0, 255, 255)));
    assert_eq!(atlas.get_pixel(9, 0), Some((255, 255, 0, 255)));
  }

  #[test]
  fn fill_appends_rendered_images_to_an_atlas() {
    let first = Image::new_from_color(2, 1, Color::from_rgba(255, 0, 0, 255));
    let second = Image::new_from_color(1, 2, Color::from_rgba(0, 255, 0, 255));
    let mut atlas = atlas();

    fill(&first, &first).apply(&mut atlas);
    fill(&second, &second).apply(&mut atlas);

    let image = atlas.create();
    assert_eq!(image.dimensions::<u32>(), (4, 2));
    assert_eq!(image.get_pixel(0, 0), Some((255, 0, 0, 255)));
    assert_eq!(image.get_pixel(2, 1), Some((0, 255, 0, 255)));
  }
}
