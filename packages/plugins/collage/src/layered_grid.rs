use std::sync::Arc;

use abra::{
  Color, Image,
  canvas::{AddCanvasOptions, Canvas, LayerEffects, LayerSize, NewLayerOptions},
};
use rand::{Rng, prelude::SliceRandom};
use rayon::prelude::*;

use crate::{CollagePlugin, CollageStyle};

impl CollagePlugin {
  pub(crate) fn layered_grid_collage(&mut self) -> Canvas {
    // Get grid dimensions (columns, rows) and total number of cells.
    let (columns, rows, cell_count) = if let CollageStyle::LayeredGrid(c, r) = self.style {
      (c, r, c * r)
    } else {
      (1, 1, 1)
    };
    // The width of each cell in the grid.
    let cell_width = self.size.0 / columns;
    // The height of each cell in the grid.
    let cell_height = self.size.1 / rows;
    // The root canvas for the collage. Create with explicit size so it doesn't
    // automatically resize to the size of the first child canvas.
    let root_canvas = Canvas::new_blank("Collage", self.size.0, self.size.1);

    self.set_background(&root_canvas);

    let mut item_vec: Vec<u32> = (0..cell_count).collect();
    item_vec.shuffle(&mut self.rng);

    let mut selected_data = vec![];
    for _ in 0..cell_count {
      let image = self.select_random_image();
      let rotation = self
        .options
        .as_ref()
        .map(|opts| opts.rotation)
        .map(|rot| self.select_range(rot));
      let scale = self
        .options
        .as_ref()
        .map(|opts| opts.scale)
        .map(|scale_range| self.rng.random_range(scale_range.0..=scale_range.1))
        .unwrap_or(1.0);
      selected_data.push((image, rotation, scale));
    }

    let processed_canvases: Vec<_> = selected_data
      .into_par_iter()
      .enumerate()
      .map(|(idx, (image, rotation, scale))| {
        let i = item_vec[idx];
        let position = (((i % columns) * cell_width) as i32, ((i / columns) * cell_height) as i32);

        let (scale_width, scale_height) = ((cell_width as f32 * scale) as u32, (cell_height as f32 * scale) as u32);

        // Create canvas and apply transformations in parallel
        let transform_image = Arc::new(Image::new_from_color(scale_width, scale_height, Color::transparent()));
        let canvas = Canvas::new("Cell")
          .add_layer_from_image("empty", transform_image, None)
          .add_layer_from_image(
            "image",
            image,
            NewLayerOptions::new()
              .with_anchor(abra::canvas::Anchor::Center)
              .with_size(LayerSize::Cover(None)),
          );

        let mut canvas_options = AddCanvasOptions::new().with_position(position.0, position.1);

        if let Some(rot) = rotation {
          canvas_options = canvas_options.with_rotation(rot);
        }

        (canvas, canvas_options)
      })
      .collect();

    let collage_effects = self
      .options
      .as_ref()
      .and_then(|opts| opts.effects.clone())
      .unwrap_or(LayerEffects::new());
    // Apply collage-level effects to each image layer.
    for (canvas, _) in &processed_canvases {
      canvas.set_effects(collage_effects.clone());
      // if let Some(image_layer) = canvas.get_layer_by_name("empty") {
      //   image_layer.set_effects(collage_effects.clone());
      // }
    }

    // Add all processed canvases to root canvas sequentially (Canvas::add_canvas requires mutable access)
    for (canvas, canvas_options) in processed_canvases {
      root_canvas.add_canvas(canvas, Some(canvas_options.clone()));
    }

    root_canvas
  }
}
