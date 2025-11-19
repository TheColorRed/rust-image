use std::sync::Arc;

use abra::{
  Color, Image, Rotate,
  image::{AddCanvasOptions, Anchor, Canvas, LayerSize, NewLayerOptions},
};

use crate::{CollagePlugin, CollageStyle};

impl CollagePlugin {
  pub(crate) fn grid_collage(&mut self) -> Canvas {
    // Get the total number of cells in the grid.
    let mut cell_count = 0;
    if let CollageStyle::Grid(columns, rows) = self.style {
      cell_count = columns * rows;
    }
    // The width of each cell in the grid.
    let cell_width = self.size.0 / (cell_count as f32).sqrt() as u32;
    // The height of each cell in the grid.
    let cell_height = self.size.1 / (cell_count as f32).sqrt() as u32;
    // Create a canvas sized to the requested collage dimensions to avoid
    // defaulting to the size of the first added child canvas.
    let root_canvas = Canvas::new_blank("Collage", self.size.0, self.size.1);

    self.set_background(&root_canvas);

    for i in 0..cell_count {
      // Get a random image from the provided images.
      let original_image = self.select_random_image();
      let mut image = (*original_image).clone();

      // Rotate the image if the option.rotation is set
      if let Some(rotation) = self.options.as_ref().and_then(|opts| Some(opts.rotation)) {
        image.rotate(self.select_rotation(rotation), None);
      }

      let trans_image = Arc::new(Image::new_from_color(cell_width, cell_height, Color::transparent()));
      let canvas = Canvas::new("Cell")
        .add_layer_from_image("empty", trans_image, None)
        .add_layer_from_image(
          "image",
          Arc::new(image),
          Some(
            NewLayerOptions::new()
              .with_anchor(Anchor::TopCenter)
              .with_size(LayerSize::Cover(None)),
          ),
        );

      root_canvas.add_canvas(
        canvas,
        Some(AddCanvasOptions::new().with_position(
          ((i % (cell_count as f32).sqrt() as u32) * cell_width) as i32,
          ((i / (cell_count as f32).sqrt() as u32) * cell_height) as i32,
        )),
      );
    }
    root_canvas
  }
}
