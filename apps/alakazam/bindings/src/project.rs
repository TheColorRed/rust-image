use crate::{common::*, generate_image::transparent_pattern};
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

use abra::{
  canvas::prelude::{AdjustmentLayerType, Canvas},
  prelude::{Color, Image, Resize},
};

use abra::abra_core::blend::blend_mode_name;

/// Represents a project. A project has one root canvas.
/// The canvas then contains other layers and canvases.
#[napi]
pub struct Project {
  id: String,
  active_layer: Option<Vec<String>>,
  canvas: Canvas,
  width: u32,
  height: u32,
}

#[napi]
impl Project {
  #[napi(constructor)]
  /// Create a new blank project with the given name and dimensions.
  /// @param name The name of the project.
  /// @param width The width of the canvas.
  /// @param height The height of the canvas.
  /// @returns A new Project instance.
  pub fn new_blank(name: String, width: u32, height: u32) -> Self {
    let canvas = Canvas::new_blank(name, width, height);
    let id = canvas.id();
    Project {
      id,
      active_layer: None,
      canvas,
      width,
      height,
    }
  }

  #[napi(constructor)]
  /// Create a new project by loading an image from a file path.
  /// @param name The name of the project.
  /// @param filePath The file path to load the image from.
  /// @returns A new Project instance.
  pub fn new_from_file(name: String, file_path: String) -> Self {
    let canvas = Canvas::new_from_path(name, file_path, None);
    let (width, height) = canvas.dimensions();
    let id = canvas.id();
    let active_layer = canvas.layers().last().map(|layer| vec![layer.id()]);
    Project {
      id,
      active_layer,
      canvas,
      width,
      height,
    }
  }

  #[napi]
  /// Gets the project's active layers.
  /// @returns The active layers, or an empty array if none are set.
  pub fn active_layers(&self) -> Vec<LayerMetadata> {
    self
      .active_layer
      .as_ref()
      .map(|layer_ids| {
        layer_ids
          .iter()
          .filter_map(|id| {
            self
              .canvas
              .get_layer_by_id(id)
              .map(|layer| self.get_layer_metadata(&layer))
          })
          .collect()
      })
      .unwrap_or_default()
  }

  #[napi]
  /// Sets multiple active layers by their IDs.
  /// @param layerIds An array of layer IDs to set as active.
  pub fn set_active_layers(&mut self, layer_ids: Vec<String>) -> &Self {
    self.active_layer = Some(layer_ids);
    self
  }

  #[napi]
  /// Sets a single active layer by its ID.
  /// @param layerId The ID of the layer to set as active.
  pub fn set_active_layer(&mut self, layer_id: String) -> &Self {
    self.set_active_layers(vec![layer_id]);
    self
  }

  #[napi]
  /// Composites the project's canvas and returns the resulting image data.
  /// @returns The composited image data.
  pub fn composite(&self) -> ImageData {
    let start = std::time::Instant::now();
    let image = self.canvas.as_image();
    println!("Composite took {:?}", start.elapsed());
    let (width, height) = image.dimensions();
    ImageData {
      data: Buffer::from(image.rgba().to_vec()),
      width,
      height,
    }
  }

  #[napi(getter)]
  /// Gets the name of the project.
  /// @returns The name of the project.
  pub fn name(&self) -> String {
    self.canvas.name().to_string()
  }

  #[napi(getter)]
  /// Gets the unique ID of the project.
  /// @returns The unique ID of the project.
  pub fn id(&self) -> String {
    self.id.clone()
  }

  #[napi(getter)]
  /// Gets the width of the canvas.
  /// @returns The width of the canvas.
  pub fn width(&self) -> u32 {
    self.width
  }

  #[napi(getter)]
  /// Gets the height of the canvas.
  /// @returns The height of the canvas.
  pub fn height(&self) -> u32 {
    self.height
  }

  #[napi(getter)]
  /// Gets the layers of the canvas.
  /// @returns An array of layers in the canvas.
  pub fn layer_metadata(&self) -> Vec<LayerMetadata> {
    self
      .canvas
      .layers()
      .iter()
      .rev()
      .map(|layer| self.get_layer_metadata(&layer))
      .collect()
  }

  #[napi]
  /// Adds a new layer to the canvas from a file path.
  /// @param name The name of the layer.
  /// @param filePath The file path to load the image from.
  pub fn add_layer_from_path(&mut self, name: String, file_path: String) -> LayerMetadata {
    // `Canvas::add_layer_from_path` consumes `self`, so we can't move `self.canvas`
    // out of the mutable reference directly. Use `std::mem::replace` to swap
    // the Canvas with a temporary blank canvas, then assign the result back.
    let canvas = std::mem::take(&mut self.canvas);
    self.canvas = canvas.add_layer_from_path(name, file_path, None);
    let layers = self.canvas.layers();
    let layer = layers.last().unwrap();
    layer.mark_dirty();
    self.get_layer_metadata(&layer)
  }

  #[napi]
  /// Adds a new empty layer to the canvas.
  /// @param name The name of the layer.
  pub fn add_empty_layer(&mut self, name: String) -> LayerMetadata {
    let (width, height) = self.canvas.dimensions();
    let canvas = std::mem::take(&mut self.canvas);
    let image = Image::new_from_color(width, height, Color::transparent());
    self.canvas = canvas.add_layer_from_image(name, image, None);
    let layers = self.canvas.layers();
    let layer = layers.last().unwrap();
    layer.mark_dirty();
    self.get_layer_metadata(&layer)
  }

  #[napi]
  /// Adds a new adjustment layer to the canvas.
  /// @param name The name of the layer.
  /// @param adjustmentType The type of adjustment layer to add (e.g., "brightness-contrast").
  pub fn add_adjustment_layer(&mut self, name: String, adjustment_type: String) -> LayerMetadata {
    let canvas = std::mem::take(&mut self.canvas);
    let adjustment_type = adjustment_type.into();
    self.canvas = canvas.add_adjustment_layer(name, adjustment_type);
    let layers = self.canvas.layers();
    let layer = layers.last().unwrap();
    layer.mark_dirty();
    self.get_layer_metadata(&layer)
  }

  #[napi]
  /// Gets a layer by its index.
  /// @param index The index of the layer.
  /// @returns The layer at the given index, or null if not found.
  pub fn get_layer_by_index(&self, index: u32) -> Option<Layer> {
    let layer = self.canvas.get_layer_by_index(index as usize);
    layer.map(|layer| Layer {
      layer,
      project_id: self.id.clone(),
    })
  }

  #[napi]
  /// Gets a layer by its UUID.
  /// @param id The UUID of the layer.
  /// @returns The layer with the given UUID, or null if not found.
  pub fn get_layer_by_id(&self, id: String) -> Option<Layer> {
    let layer = self.canvas.get_layer_by_id(&id);
    layer.map(|layer| Layer {
      layer,
      project_id: self.id.clone(),
    })
  }

  #[napi]
  /// Gets a layer by its name.
  /// @param name The name of the layer.
  /// @returns The layer with the given name, or null if not found.
  pub fn get_layer_by_name(&self, name: String) -> Option<Layer> {
    let layer = self.canvas.get_layer_by_name(&name);
    layer.map(|layer| Layer {
      layer,
      project_id: self.id.clone(),
    })
  }

  #[napi]
  /// Moves a layer to a new index in the layer stack.
  /// @param layer The layer to move.
  /// @param newIndex The new index to move the layer to.
  pub fn move_layer_to(&mut self, layer: &Layer, new_index: f64) -> &Self {
    let new_index = if new_index < 0f64 {
      0 as usize
    } else if new_index as usize >= self.canvas.layers().len() {
      (self.canvas.layers().len() - 1) as usize
    } else {
      new_index as usize
    };
    layer.get_underlying_layer().set_index(new_index as usize);
    self
  }

  #[napi]
  /// Moves a layer up one position in the layer stack.
  /// @param layer The layer to move.
  pub fn move_layer_up(&mut self, layer: &Layer) -> &Self {
    layer.get_underlying_layer().move_up();
    self
  }

  #[napi]
  /// Moves a layer down one position in the layer stack.
  /// @param layer The layer to move.
  pub fn move_layer_down(&mut self, layer: &Layer) -> &Self {
    layer.get_underlying_layer().move_down();
    self
  }

  #[napi]
  /// Moves a layer to the top of the layer stack.
  /// @param layer The layer to move.
  pub fn move_layer_to_top(&mut self, layer: &Layer) -> &Self {
    layer.get_underlying_layer().move_to_top();
    self
  }

  #[napi]
  /// Reorders the layers in the layer stack according to the given array of layer IDs.
  /// @param newOrderIds An array of layer IDs representing the new order.
  pub fn reorder_layers(&mut self, new_order_ids: Vec<String>) -> &Self {
    self.canvas.reorder_layers_by_id(new_order_ids);
    self
  }

  #[napi]
  /// Delate a layer from the layer stack.
  /// @param layer The layer to delete.
  pub fn delete_layer(&mut self, layer: &Layer) -> &Self {
    let layer_id = layer.get_underlying_layer().id();
    self.canvas.delete_layer_by_id(&layer_id);
    self
  }

  #[napi]
  /// Moves a layer to the bottom of the layer stack.
  /// @param layer The layer to move.
  pub fn move_layer_to_bottom(&mut self, layer: &Layer) -> &Self {
    layer.get_underlying_layer().move_to_bottom();
    self
  }

  #[napi]
  /// Get the image data of a specific layer.as it would appear when composited without the other layers.
  /// @param layer The layer to composite.
  /// @param maxSize Optional maximum size (in pixels) for the longest dimension of the resulting image. If the composited image exceeds this size, it will be scaled down proportionally.
  /// @returns The image data of the composited layer.
  pub fn composite_layer(&self, layer: &Layer, max_size: Option<u32>) -> ImageData {
    let layer = layer.get_underlying_layer();
    // let (width, height) = layer.dimensions();

    // get current visibility of all layers
    let current_layer_visibility: Vec<(String, bool)> =
      self.canvas.layers().iter().map(|l| (l.id(), l.is_visible())).collect();

    // hide all layers except the target layer
    for l in self.canvas.layers() {
      if l.id() != layer.id() {
        l.set_visible(false);
      } else {
        l.set_visible(true);
      }
    }

    let (width, height) = (self.width, self.height);
    let mut canvas = Canvas::new_blank("Background", width, height).add_layer_from_image(
      "Pattern",
      transparent_pattern(width as u32, height as u32, 2),
      None,
    );

    // get the composited image
    let img = self.canvas.as_image();
    canvas = canvas.add_layer_from_image("Composite", img, None);

    // restore original visibility of all layers
    for (layer_id, visible) in current_layer_visibility {
      if let Some(l) = self.canvas.get_layer_by_id(&layer_id) {
        l.set_visible(visible);
      }
    }

    let (new_width, new_height) = if let Some(max_size) = max_size {
      let scale_factor = (max_size as f32 / width as f32).min(max_size as f32 / height as f32);
      if scale_factor < 1.0 {
        let new_width = (width as f32 * scale_factor) as u32;
        let new_height = (height as f32 * scale_factor) as u32;
        (new_width, new_height)
      } else {
        (width, height)
      }
    } else {
      (width, height)
    };

    if new_width != width || new_height != height {
      canvas.transform().resize(new_width, new_height, None);
    }
    let (width, height) = canvas.dimensions::<u32>();

    ImageData {
      data: Buffer::from(canvas.as_image().rgba().to_vec()),
      width: width,
      height: height,
    }
  }

  #[napi]
  /// Gets the metadata of the project.
  /// @returns The project metadata.
  pub fn metadata(&self) -> ProjectMetadata {
    ProjectMetadata {
      id: self.id.clone(),
      name: self.canvas.name().to_string(),
      width: self.width,
      height: self.height,
      active_layers: self.active_layers(),
      layers: self.layer_metadata(),
    }
  }

  #[napi]
  pub fn save(&self, file_path: String) {
    self.canvas.save(file_path, None);
  }

  /// Helper function to get layer metadata
  fn get_layer_metadata(&self, layer: &abra::canvas::prelude::Layer) -> LayerMetadata {
    let (width, height) = layer.dimensions::<u32>();
    let (x_offset, y_offset) = layer.position();
    LayerMetadata {
      id: layer.id(),
      project_id: self.id.clone(),
      name: layer.name().to_string(),
      blend_mode: blend_mode_name(layer.blend_mode()).0.to_string(),
      opacity: layer.opacity() as f64,
      visible: layer.is_visible(),
      order: layer.current_index().unwrap_or(0) as u32,
      adjustment_type: layer.adjustment_type().map(|t| t.to_string()),
      width,
      height,
      x: x_offset,
      y: y_offset,
      angle: 0f64,
    }
  }
}
