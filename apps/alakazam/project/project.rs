use crate::{actions::open_file::ImageDataLoaded, project::layer::Layer};
use abra::Image;

pub struct Project {
  pub original_path: String,
  pub layers: Vec<Layer>,
  pub image: Image,
  pub width: u32,
  pub height: u32,
}

impl Project {
  pub fn new_from_image_data(image_data: ImageDataLoaded) -> Project {
    let layer = image_data.image;
    let image = layer.clone();
    let path = image_data.path;
    let (width, height) = layer.dimensions();
    Project {
      original_path: path,
      layers: vec![Layer::new("Layer 1".to_string(), layer)],
      width,
      height,
      image,
    }
  }
  pub fn new_from_image(image: Image) -> Project {
    let layer = image.clone();
    let (width, height) = layer.dimensions();
    Project {
      original_path: "".to_string(),
      layers: vec![Layer::new("Layer 1".to_string(), layer)],
      width,
      height,
      image,
    }
  }
  /// Adds a new layer to the project with the given name
  pub fn add_layer_with_name(&mut self, image: Image, name: String) {
    let layer = Layer::new(name, image);
    self.layers.push(layer);
  }
  /// Adds a new layer to the project using the default name of "Layer {index}"
  pub fn add_layer(&mut self, image: Image) {
    self.add_layer_with_name(image, format!("Layer {}", self.layers.len() + 1));
  }
  /// Remove the layer at the given index and cleans up the memory
  pub fn delete_layer_at(&mut self, index: usize) {
    if index < self.layers.len() {
      self.layers.remove(index);
    }
  }
  pub fn get_layer_at(&self, index: usize) -> Option<&Layer> {
    if index < self.layers.len() {
      return Some(&self.layers[index]);
    }
    None
  }
}
