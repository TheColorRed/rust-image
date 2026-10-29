//! A canvas of cells, each holding a picture read from a file.

use std::collections::HashMap;
use std::sync::Arc;

use abra_core::Image;
use abra_core::image::image_ext::ImageExt;
use rayon::prelude::*;

use super::canvas::Canvas;
use super::options_add_canvas::AddCanvasOptions;
use super::options_new_layer::{LayerSize, NewLayerOptions};

/// One cell of a canvas made by [`Canvas::new_of_cells`]: the picture file that goes in it, and where the cell is and how big it is, in
/// pixels of the canvas.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasCell {
  /// The picture file.
  pub path: String,
  /// The left edge of the cell, which may be negative or reach past the canvas.
  pub x: i32,
  /// The top edge of the cell.
  pub y: i32,
  pub width: u32,
  pub height: u32,
  /// Keeps the picture's own proportions, with nothing cropped: the cell shrinks to the largest size of the picture that fits in
  /// `width` x `height`, and sits in the middle of that space. When false the picture is scaled to cover the whole cell and the part that
  /// does not fit is cropped.
  pub keep_aspect: bool,
}

impl<'a> Canvas<'a> {
  /// A canvas of `p_width` x `p_height` with a child canvas for each of `p_cells`, in order, each holding its picture scaled to cover the
  /// cell (the part that does not fit is cropped), at the cell's place. The cells are the canvas's [`Canvas::canvases`], so each can be
  /// turned, scaled and moved afterwards.
  ///
  /// The pictures are read in parallel, and a file that more than one cell uses is read once. A file is held only while the cells that use
  /// it are made, so the memory it needs is about one picture for each thread, not all of them. Fails with the first file that cannot be
  /// read, and says which.
  pub fn new_of_cells(
    p_name: impl Into<String>, p_width: u32, p_height: u32, p_cells: &[CanvasCell],
  ) -> Result<Canvas<'a>, String> {
    let mut by_file: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, cell) in p_cells.iter().enumerate() {
      by_file.entry(cell.path.as_str()).or_default().push(index);
    }

    let made: Vec<Result<Vec<(usize, Canvas<'a>)>, String>> = by_file
      .into_par_iter()
      .map(|(path, indexes)| {
        let picture = Arc::new(Image::read(path).map_err(|error| format!("{path}: {error}"))?);
        Ok(
          indexes
            .into_iter()
            .map(|index| {
              let cell = &p_cells[index];
              let (width, height) = if cell.keep_aspect {
                let (picture_width, picture_height) = picture.dimensions::<u32>();
                let fit = (cell.width as f32 / picture_width as f32).min(cell.height as f32 / picture_height as f32);
                (
                  ((picture_width as f32 * fit).round() as u32).clamp(1, cell.width.max(1)),
                  ((picture_height as f32 * fit).round() as u32).clamp(1, cell.height.max(1)),
                )
              } else {
                (cell.width, cell.height)
              };
              let canvas = Canvas::new_blank("Cell", width, height);
              canvas.add_layer_from_image(
                "photo",
                Arc::clone(&picture),
                NewLayerOptions::new().with_size(LayerSize::Cover(None)),
              );
              (index, canvas)
            })
            .collect(),
        )
      })
      .collect();

    let mut canvases = Vec::with_capacity(p_cells.len());
    for group in made {
      canvases.extend(group?);
    }
    // The cells go in in the order they were given, whichever file finished first.
    canvases.sort_by_key(|(index, _)| *index);

    let root = Canvas::new_blank(p_name, p_width, p_height);
    for (index, canvas) in canvases {
      let cell = &p_cells[index];
      // A cell that kept its picture's proportions is smaller than its place, and sits in the middle of it.
      let (width, height) = canvas.dimensions::<u32>();
      let x = cell.x + (cell.width as i32 - width as i32) / 2;
      let y = cell.y + (cell.height as i32 - height as i32) / 2;
      root.add_canvas(canvas, AddCanvasOptions::new().with_position(x, y));
    }
    Ok(root)
  }
}
