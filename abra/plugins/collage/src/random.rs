use crate::{CollagePlugin, CollageStyle};

use abra::canvas::prelude::*;
use abra::prelude::*;

use rand::Rng;
use rayon::prelude::*;

use std::sync::Arc;

impl CollagePlugin {
  pub(crate) fn random_collage(&mut self) -> Canvas {
    // Get the total number of images to include in the collage.
    // The ColorStyle::Random will always be true here.
    let total_images = match &self.style {
      CollageStyle::Random(amount) => (*amount).max(1),
      _ => self.images.len() as u32,
    };

    let (root_canvas_width, root_canvas_height) = self.size;
    let root_canvas = Canvas::new_blank("Random Collage", root_canvas_width, root_canvas_height);

    self.set_background(&root_canvas);

    (0..total_images)
      .into_iter()
      .map(|_| {
        let image = self.select_random_image();
        let options = self.options.as_mut().unwrap().clone();
        let rotation = self.select_range(options.rotation);
        let scale = self.select_range(options.scale);
        let (width, height) = image.dimensions::<u32>();
        let width_range = root_canvas_width.saturating_sub((width as f32 * scale) as u32);
        let height_range = root_canvas_height.saturating_sub((height as f32 * scale) as u32);
        let position =
          PointF::new(self.rng.random_range(0..=width_range as i32), self.rng.random_range(0..=height_range as i32));
        (image, rotation, scale, position)
      })
      .collect::<Vec<(Arc<Image>, f32, f32, PointF)>>()
      .into_par_iter()
      .for_each(|(image, rotation, scale, position)| {
        let (width, height) = image.dimensions::<u32>();
        let (scale_width, scale_height) = ((width as f32 * scale) as u32, (height as f32 * scale) as u32);
        let transform_image = Arc::new(Image::new_from_color(scale_width, scale_height, Color::transparent()));
        let canvas = Canvas::new("Random Image")
          .add_layer_from_image("empty", transform_image, None)
          .add_layer_from_image("image", image, Some(NewLayerOptions::new().with_size(LayerSize::Cover(None))));

        if let Some(image_layer) = canvas.get_layer_by_name("image") {
          let effects = self
            .options
            .as_ref()
            .and_then(|opts| opts.effects.clone())
            .unwrap_or(LayerEffects::new());
          image_layer.set_effects(effects.clone());
        }

        let canvas_options = AddCanvasOptions::new()
          .with_position(position.x as i32, position.y as i32)
          .with_rotation(rotation);

        root_canvas.add_canvas(canvas, Some(canvas_options.clone()));
      });

    root_canvas
  }
}
