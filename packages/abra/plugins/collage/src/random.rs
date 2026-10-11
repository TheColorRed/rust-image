use crate::{CollagePlugin, CollageStyle};

use abra::canvas::prelude::*;
use abra::prelude::*;

use rand::prelude::RngExt;
use rayon::prelude::*;

use std::sync::Arc;

impl<'a> CollagePlugin<'a> {
  pub(crate) fn random_collage(&mut self) -> Canvas<'a> {
    // Get the total number of images to include in the collage.
    // The ColorStyle::Random will always be true here.
    let total_images = match &self.style {
      CollageStyle::Random(amount) => (*amount).max(1),
      _ => self.images.len() as u32,
    };

    let (root_canvas_width, root_canvas_height) = self.size;
    let root_canvas = Canvas::new_blank("Random Collage", root_canvas_width, root_canvas_height);

    self.set_background(&root_canvas);

    // Precompute a clone of options and derived collage_effects so we don't borrow `self` inside the parallel region.
    let options = self.options.clone().unwrap_or(crate::CollageOptions::new());
    let collage_effects = options.effects.clone().unwrap_or(LayerEffects::new());

    // Create item list sequentially (uses RNG and plugin state)
    let items: Vec<(Arc<Image>, f32, f32, PointF)> = (0..total_images)
      .into_iter()
      .map(|_| {
        let image = self.select_random_image();
        let rotation = self.select_range(options.rotation);
        let scale = self.select_range(options.scale);
        let (width, height) = image.dimensions::<u32>();
        let width_range = root_canvas_width.saturating_sub((width as f32 * scale) as u32);
        let height_range = root_canvas_height.saturating_sub((height as f32 * scale) as u32);
        let position =
          PointF::new(self.rng.random_range(0..=width_range as i32), self.rng.random_range(0..=height_range as i32));
        (image, rotation, scale, position)
      })
      .collect();

    // Process each item in parallel to build canvases
    let processed_canvases: Vec<(Canvas, AddCanvasOptions)> = items
      .into_par_iter()
      .map(|(image, rotation, scale, position)| {
        let (width, height) = image.dimensions::<u32>();
        let (scale_width, scale_height) = ((width as f32 * scale) as u32, (height as f32 * scale) as u32);
        let transform_image = Arc::new(Image::new_from_color(scale_width, scale_height, Color::transparent()));
        let canvas = Canvas::new("Random Image");
        canvas.add_layer_from_image("empty", transform_image, None);
        canvas.add_layer_from_image("image", image, Some(NewLayerOptions::new().with_size(LayerSize::Cover(None))));

        // Apply effects to the image layer (use cloned collage_effects).
        // Scope the borrow so it doesn't overlap the move/return of `canvas` below.
        // Apply collage-level effects to the canvas so the image layer inherits them.
        canvas.set_effects(collage_effects.clone());

        let canvas_options =
          AddCanvasOptions::new().with_position(position.x as i32, position.y as i32).with_rotation(rotation);

        (canvas, canvas_options)
      })
      .collect();

    // Add canvases sequentially into the root canvas
    for (canvas, canvas_options) in processed_canvases {
      root_canvas.add_canvas(canvas, Some(canvas_options.clone()));
    }

    root_canvas
  }
}
