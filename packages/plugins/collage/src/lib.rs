use core::ops::Deref;
use core::ops::DerefMut;
use std::sync::Arc;

use abra::Area;
use abra::Image;
use abra::canvas::Canvas;
use abra::canvas::LayerEffects;
use abra::canvas::LayerSize;
use abra::canvas::NewLayerOptions;
use abra::drawing::Brush;
use abra::drawing::fill_area_with_brush;
// Note: `paint_with_brush`, `shader_from_fill` and `stroke_with_brush` were
// previously imported but are not used directly in this file. Remove them to
// avoid unused import warnings. Use `fill_area_with_brush` which is consumed
// in `CollagePlugin::set_background` above.
use abra::plugin::{Plugin, PluginError, PluginResult};
use abra::{Color, Fill, LoadedImages};
use rand::prelude::{IndexedRandom, Rng};
use rand::rngs::ThreadRng;

mod grid;
mod layered_grid;
mod random;

#[derive(Clone)]
pub struct CollageOptions {
  /// Rotation range for images in the collage.
  /// - Positive values rotate clockwise.
  /// - Negative values rotate counter-clockwise.
  rotation: (f32, f32),
  /// Scale range for images in the collage.
  /// - Values less than 1.0 shrink the image.
  /// - Values greater than 1.0 enlarge the image.
  /// - Values less than zero are clamped to 0.0.
  scale: (f32, f32),
  /// Background color for the collage.
  /// - If None, the background will be transparent.
  background: Fill,
  /// The effects to apply to each layer in the collage.
  effects: Option<LayerEffects>,
}

impl CollageOptions {
  /// Creates a new CollageOptions instance with default values.
  pub fn new() -> Self {
    Self {
      rotation: (0.0, 0.0),
      scale: (1.0, 1.0),
      background: Fill::Solid(Color::transparent()),
      effects: None,
    }
  }

  /// Sets the rotation range for images in the collage.
  pub fn with_rotation_range(mut self, min: impl Into<f64>, max: impl Into<f64>) -> Self {
    self.rotation = (min.into() as f32, max.into() as f32);
    self
  }

  /// Sets the scale range for images in the collage.
  pub fn with_scale_range(mut self, min: impl Into<f64>, max: impl Into<f64>) -> Self {
    self.scale = (min.into() as f32, max.into() as f32);
    self
  }

  /// Sets the background color for the collage.
  pub fn with_background(mut self, background: impl Into<Fill>) -> Self {
    self.background = background.into();
    self
  }

  /// Sets the effects to apply to each layer in the collage.
  pub fn with_effects(mut self, effects: LayerEffects) -> Self {
    self.effects = Some(effects);
    self
  }
}

pub enum CollageStyle {
  /// A grid collage with specified number of columns and rows.
  /// All cells will be evenly sized to fit within the overall canvas size.
  /// This may cause some images to be cropped.
  /// - columns: Number of columns in the grid.
  /// - rows: Number of rows in the grid.
  Grid(u32, u32),
  /// A layered grid collage with specified number of columns and rows.
  /// Each cell will be the size of the image placed within it, causing an "overlap" effect.
  /// - columns: Number of columns in the grid.
  /// - rows: Number of rows in the grid.
  LayeredGrid(u32, u32),
  /// A random collage where images are placed at random positions.
  /// - `count`: Number of images to include in the random collage.
  Random(u32),
}

/// A plugin that creates collages from multiple images.
pub struct CollagePlugin {
  /// The size of the collage canvas.
  size: (u32, u32),
  /// The style of the collage.
  style: CollageStyle,
  /// The images to include in the collage.
  images: Vec<Arc<abra::Image>>,
  /// Options for generating the collage.
  options: Option<CollageOptions>,
  /// Indices of images already selected to avoid duplicates.
  selected_images: Vec<usize>,
  /// Random number generator for consistent randomness across selections.
  rng: ThreadRng,
}

impl CollagePlugin {
  /// Creates a new CollagePlugin instance from already loaded images.
  pub fn new<I: Into<LoadedImages>>(size: (u32, u32), images: I) -> Self {
    let loaded = images.into();
    Self {
      size,
      style: CollageStyle::Grid(2, 2),
      images: loaded.all(),
      options: None,
      selected_images: Vec::new(),
      rng: rand::rng(),
    }
  }

  pub fn with_style(mut self, style: CollageStyle) -> Self {
    self.style = style;
    self
  }

  pub fn with_options(mut self, options: CollageOptions) -> Self {
    self.options = Some(options);
    self
  }

  /// Selects a random image from the provided images.
  /// Ensures no duplicates until all images have been used.
  /// If there are more images than cells in the collage, not all images will be used.
  fn select_random_image(&mut self) -> Arc<abra::Image> {
    let available_indices: Vec<usize> = (0..self.images.len())
      .filter(|i| !self.selected_images.contains(i))
      .collect();

    if available_indices.is_empty() {
      // Reset selected images if all have been used
      self.selected_images.clear();
      return self.select_random_image();
    }

    let &selected_index = available_indices.choose(&mut self.rng).unwrap();
    self.selected_images.push(selected_index);
    self.images[selected_index].clone()
  }

  fn select_range(&mut self, range: (f32, f32)) -> f32 {
    let (min, max) = range;
    self.rng.random_range(min..=max)
  }

  fn set_background(&self, root_canvas: &Canvas) {
    if let Some(options) = &self.options {
      let background = match options.background.clone() {
        Fill::Solid(color) => {
          let bg_image = Arc::new(Image::new_from_color(self.size.0, self.size.1, color));
          Canvas::new("Background Color").add_layer_from_image("background color", bg_image, None)
        }
        Fill::Gradient(gradient) => {
          let mut bg_image = Image::new(self.size.0, self.size.1);
          let brush = Brush::new().with_color(gradient.clone());
          let area = Area::new_from_image(&bg_image);
          fill_area_with_brush(&mut bg_image, &area, &brush);

          Canvas::new("Background Color").add_layer_from_image("background color", Arc::new(bg_image), None)
        }
        Fill::Image(image) => {
          let bg_image = Arc::new(Image::new(self.size.0, self.size.1));
          Canvas::new("Background Color")
            .add_layer_from_image("background color", bg_image, None)
            .add_layer_from_image(
              "Image",
              image.clone(),
              Some(NewLayerOptions::new().with_size(LayerSize::Cover(None))),
            )
            .flatten()
        }
      };
      root_canvas.add_canvas(background, None);
    }
  }
}

impl Plugin for CollagePlugin {
  fn name(&self) -> &str {
    "Collage"
  }

  fn description(&self) -> &str {
    "A plugin that creates collages from multiple images."
  }

  fn apply(&mut self) -> Result<PluginResult, PluginError> {
    let start = std::time::Instant::now();
    let mut plugin_result = PluginResult::new();
    match &self.style {
      CollageStyle::Grid(_columns, _rows) => {
        let collage_result = self.grid_collage();
        plugin_result.add_canvas(collage_result);
      }
      CollageStyle::LayeredGrid(_columns, _rows) => {
        let collage_result = self.layered_grid_collage();
        plugin_result.add_canvas(collage_result);
      }
      CollageStyle::Random(_count) => {
        let collage_result = self.random_collage();
        plugin_result.add_canvas(collage_result);
      }
    };

    if plugin_result.is_empty() {
      return Err(PluginError::execution_failed("CollagePlugin produced no canvases"));
    }

    println!("CollagePlugin created in {:?}", start.elapsed());
    Ok(plugin_result)
  }
}
