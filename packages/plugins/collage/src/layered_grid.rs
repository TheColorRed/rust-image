use abra::{Canvas, Image, LayerSize, NewLayerOptions, canvas::AddCanvasOptions, color::Color, transform::Crop};
use rand::prelude::SliceRandom;
use rayon::prelude::*;

use crate::{CollagePlugin, CollageStyle};

impl CollagePlugin {
  pub(crate) fn layered_grid_collage(&mut self) -> abra::Canvas {
    // Get the total number of cells in the grid.
    let mut cell_count = 0;
    if let CollageStyle::LayeredGrid(columns, rows) = self.style {
      cell_count = columns * rows;
    };
    // The width of each cell in the grid.
    let cell_width = self.size.0 / (cell_count as f32).sqrt() as u32;
    // The height of each cell in the grid.
    let cell_height = self.size.1 / (cell_count as f32).sqrt() as u32;
    // The root canvas for the collage.
    let root_canvas = Canvas::new("Collage");

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
        .map(|rot| self.select_rotation(rot));
      selected_data.push((image, rotation));
    }

    let processed_canvases: Vec<_> = selected_data
      .into_par_iter()
      .enumerate()
      .map(|(idx, (image, rotation))| {
        let i = item_vec[idx];
        let position = (
          ((i % (cell_count as f32).sqrt() as u32) * cell_width) as i32,
          ((i / (cell_count as f32).sqrt() as u32) * cell_height) as i32,
        );

        // Create canvas and apply transformations in parallel
        let transform_image = std::sync::Arc::new(Image::new_from_color(cell_width, cell_height, Color::transparent()));
        let canvas = Canvas::new("Cell")
          .add_layer_from_image("empty", transform_image, None)
          .add_layer_from_image("image", image, Some(NewLayerOptions::new().with_size(LayerSize::Cover(None))));

        let mut canvas_options = AddCanvasOptions::new().with_position(position.0, position.1);

        if let Some(empty_layer) = canvas.get_layer_by_name("empty") {
          if let Some(image_layer) = canvas.get_layer_by_name("image") {
            let (width, height) = empty_layer.dimensions::<u32>();
            image_layer.transform().crop(0, 0, width, height);
          }
        }

        if let Some(rot) = rotation {
          canvas_options = canvas_options.with_rotation(rot);
        }

        (canvas, canvas_options)
      })
      .collect();

    // Add all processed canvases to root canvas sequentially (Canvas::add_canvas requires mutable access)
    for (canvas, canvas_options) in processed_canvases {
      // Queue effects on the child canvas's layers using the builder
      if let Some(opts) = &self.options {
        if let Some(effects) = &opts.effects {
          for layer in canvas.layers() {
            let builder = layer.effects();
            let builder = if let Some(drop_shadow_effect) = &effects.drop_shadow {
              builder.with_drop_shadow(drop_shadow_effect.clone())
            } else {
              builder
            };
            if let Some(stroke_effect) = &effects.stroke {
              builder.with_stroke(stroke_effect.clone());
            }
          }
        }
      }

      root_canvas.add_canvas(canvas, Some(canvas_options));
    }

    root_canvas
  }
}
