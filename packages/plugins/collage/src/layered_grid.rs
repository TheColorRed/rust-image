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

    let processed_data: Vec<_> = selected_data
      .into_par_iter()
      .enumerate()
      .map(|(idx, (image, rotation))| {
        let i = item_vec[idx];
        let position = (
          ((i % (cell_count as f32).sqrt() as u32) * cell_width) as i32,
          ((i / (cell_count as f32).sqrt() as u32) * cell_height) as i32,
        );
        (image, rotation, position)
      })
      .collect();

    for (image, rotation, position) in processed_data {
      let transform_image = Image::new_from_color(cell_width, cell_height, Color::transparent());
      let canvas = Canvas::new("Cell")
        .add_layer_from_image("empty", transform_image, None)
        .add_layer_from_image(
          "image",
          (*image).clone(),
          Some(NewLayerOptions::new().with_size(LayerSize::Cover(None))),
        );

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

      if let Some(layer) = canvas.layers().last() {
        self.apply_effects(&layer);
      }

      root_canvas.add_canvas(canvas, Some(canvas_options));
    }
    root_canvas
  }
}
