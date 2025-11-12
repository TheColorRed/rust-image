//! The internal canvas implementation.

use std::cell::Cell;
use std::sync::Arc;
use std::sync::Mutex;

use crate::Anchor;
use crate::Canvas;
use crate::Channels;
use crate::canvas::AddCanvasOptions;
use crate::canvas::Origin;
use crate::combine::blend;
use crate::combine::blend::blend_images_at_with_opacity;
use crate::image::Image;
use crate::transform::Rotate;
use crate::utils::fs::WriterOptions;

use super::layer_inner::LayerInner;
use super::options_new_layer::NewLayerOptions;

/// The internal canvas implementation - provides the mutable reference API.
pub(crate) struct CanvasInner {
  /// The unique identifier of the canvas.
  pub id: String,
  /// The name of the canvas.
  pub name: String,
  /// Child canvases in the canvas.
  canvases: Vec<Arc<Mutex<Canvas>>>,
  /// The layers in the canvas.
  pub layers: Vec<Arc<Mutex<LayerInner>>>,
  /// The width of the canvas.
  pub width: Cell<u32>,
  /// The height of the canvas.
  pub height: Cell<u32>,
  /// The x position of the canvas within its parent.
  x: Cell<i32>,
  /// The y position of the canvas within its parent.
  y: Cell<i32>,
  /// Reference to the parent canvas (if this canvas is a child).
  parent: Mutex<Option<Arc<Mutex<CanvasInner>>>>,
  /// This is the final image that is created by blending all the layers.
  pub result: Box<Image>,
  /// Whether the canvas has been manually resized and needs to skip update_canvas on save.
  pub needs_recompose: Cell<bool>,
  /// The anchor point for positioning relative to the parent canvas.
  anchor: Option<Anchor>,
  /// The rotation in degrees for positioning within the parent canvas.
  rotation: Cell<Option<f32>>,
  /// The origin point (anchor position within the canvas bounds).
  origin: Origin,
}

impl CanvasInner {
  /// Creates a new canvas with the given name and an empty canvas of a size of 0x0.
  pub fn new(name: &str) -> CanvasInner {
    CanvasInner {
      id: uuid::Uuid::new_v4().to_string(),
      name: name.to_string(),
      result: Box::new(Image::new(0, 0)),
      canvases: vec![],
      layers: vec![],
      width: Cell::new(0),
      height: Cell::new(0),
      x: Cell::new(0),
      y: Cell::new(0),
      parent: Mutex::new(None),
      needs_recompose: Cell::new(true),
      anchor: None,
      rotation: Cell::new(None),
      origin: Origin::default(),
    }
  }

  /// Creates a new project with the given name and a blank canvas with the given dimensions.
  pub fn new_blank(name: &str, width: u32, height: u32) -> CanvasInner {
    let mut canvas = CanvasInner::new(name);
    canvas.set_canvas_size(width, height);
    canvas
  }

  /// Creates a new project with the given name and a canvas from the image at the given path.
  pub fn new_from_path(name: &str, path: &str, _options: Option<NewLayerOptions>) -> CanvasInner {
    let image = Image::new_from_path(path);
    let (width, height) = image.dimensions();
    let mut canvas = CanvasInner::new(name);
    canvas.set_canvas_size(width, height);
    canvas
  }

  /// Adds a new layer to the canvas.
  pub fn add_layer(&mut self, layer: LayerInner) -> Arc<Mutex<LayerInner>> {
    let layer_rc = Arc::new(Mutex::new(layer));
    self.layers.push(layer_rc.clone());
    self.needs_recompose.set(true);
    layer_rc.clone()
  }

  /// Adds an already-wrapped child canvas with the given options.
  pub fn add_canvas_rc(&mut self, canvas_rc: Arc<Mutex<Canvas>>, options: Option<AddCanvasOptions>) {
    // Set canvas size from first child canvas
    if self.width.get() == 0 && self.height.get() == 0 {
      let canvas_ref = canvas_rc.lock().unwrap();
      let (width, height) = canvas_ref.dimensions();
      if width > 0 && height > 0 {
        self.set_canvas_size(width, height);
      }
    }

    // Now that parent size is set, calculate and apply anchor positioning
    {
      let parent_width = self.width.get() as i32;
      let parent_height = self.height.get() as i32;

      let child_dims = canvas_rc.lock().unwrap().dimensions::<i32>();
      let child_width = child_dims.0;
      let child_height = child_dims.1;

      let positions = options.as_ref().and_then(|o| o.position);
      let (x, y) = if let Some((x, y)) = positions {
        (x, y)
      } else {
        let anchor = options.as_ref().and_then(|o| o.anchor).unwrap_or(Anchor::Center);
        let (x, y) = anchor.calculate_position(parent_width, parent_height, child_width, child_height);
        (x, y)
      };

      // Set the child canvas position and rotation using public API
      let mut canvas_borrow = canvas_rc.lock().unwrap();
      canvas_borrow.set_position(x, y);
      if let Some(rotation) = options.as_ref().and_then(|o| o.rotation) {
        canvas_borrow.set_rotation(Some(rotation));
      }
    }

    self.canvases.push(canvas_rc);
    self.needs_recompose.set(true);
  }

  /// Updates the canvas image by merging all the layers and child canvases into one image.
  pub fn update_canvas(&mut self) {
    let width = self.width.get();
    let height = self.height.get();

    // Skip if canvas has zero dimensions
    if width == 0 || height == 0 {
      return;
    }

    let empty_pixels = vec![0u8; (width * height * 4) as usize];
    let mut canvas = {
      let channels = Channels::RGBA;
      let mut img = Image::new(width, height);
      match channels {
        Channels::RGBA => img.set_rgba(empty_pixels),
        Channels::RGB => img.set_rgb(empty_pixels),
      }
      img
    };

    // First pass: Apply anchors and recursively update child canvases
    for child_canvas_rc in self.canvases.iter() {
      let child_canvas = child_canvas_rc.lock().unwrap();
      child_canvas.apply_anchor_with_parent_dimensions(width as i32, height as i32);
      drop(child_canvas);

      let child_canvas_mut = child_canvas_rc.lock().unwrap();
      child_canvas_mut.update_canvas();
    }

    // Second pass: Blend child canvases
    for child_canvas_rc in self.canvases.iter() {
      let child_canvas = child_canvas_rc.lock().unwrap();
      let (child_width, child_height) = child_canvas.dimensions::<u32>();
      if child_width > 0 && child_height > 0 {
        let mut child_result = child_canvas.get_result_image();

        // Apply rotation if set
        if let Some(rotation_degrees) = child_canvas.rotation() {
          child_result.rotate(rotation_degrees, None);
        }

        let (child_x, child_y) = child_canvas.position();
        blend_images_at_with_opacity(&mut canvas, &child_result, 0, 0, child_x, child_y, blend::normal, 1.0);
      }
    }

    // Blend layers
    let canvas_dims = (width as i32, height as i32);
    for layer in self.layers.iter() {
      let mut layer_ref = layer.lock().unwrap();
      // Apply pending effects before rendering
      layer_ref.apply_pending_effects();
      // Apply anchor positioning before rendering
      layer_ref.apply_anchor_with_canvas_dimensions(canvas_dims.0, canvas_dims.1);

      if layer_ref.is_visible() {
        let opacity = layer_ref.opacity().clamp(0.0, 1.0);
        let blend = layer_ref.blend_mode();
        let (x, y) = layer_ref.position();
        let image = layer_ref.image();
        blend_images_at_with_opacity(&mut canvas, &image, 0, 0, x, y, blend, opacity);
      }
    }
    self.result = Box::new(canvas);
    self.needs_recompose.set(true);
  }
  /// Gets a clone of the result image.
  pub fn get_result_image(&self) -> Image {
    (*self.result).clone()
  }

  /// Resizes the canvas image to the given dimensions.
  pub fn set_canvas_size(&mut self, width: u32, height: u32) {
    self.result = Box::new(Image::new(width, height));
    self.width.set(width);
    self.height.set(height);
  }

  /// Sets the position of the canvas to the given anchor point within its parent canvas.
  pub fn anchor_to_canvas(&mut self, anchor: Anchor) {
    self.anchor = Some(anchor);
  }

  /// Applies the stored anchor to position the canvas, given parent canvas dimensions
  pub fn apply_anchor_with_parent_dimensions(&mut self, parent_width: i32, parent_height: i32) {
    if let Some(anchor) = self.anchor {
      let (self_width, self_height) = self.dimensions::<i32>();
      let (x, y) = anchor.calculate_position(parent_width, parent_height, self_width, self_height);
      // Position the canvas directly at the calculated anchor position
      // The anchor calculation already handles proper centering/positioning
      self.x.set(x);
      self.y.set(y);
    }
  }

  /// Sets the parent canvas reference.
  pub fn set_parent(&self, parent: Option<Arc<Mutex<CanvasInner>>>) {
    *self.parent.lock().unwrap() = parent;
  }

  /// Sets the global position of the canvas within its parent.
  pub fn set_global_position(&mut self, x: i32, y: i32) {
    self.x.set(x);
    self.y.set(y);
  }

  /// Gets the position of the canvas.
  pub fn position(&self) -> (i32, i32) {
    (self.x.get(), self.y.get())
  }

  /// Sets the rotation in degrees for the canvas within its parent.
  pub fn set_rotation(&mut self, degrees: Option<f32>) {
    self.rotation.set(degrees);
  }

  /// Gets the rotation in degrees for the canvas within its parent.
  pub fn rotation(&self) -> Option<f32> {
    self.rotation.get()
  }

  /// Gets the dimensions of the canvas.
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u32>,
    <T as TryFrom<u32>>::Error: std::fmt::Debug,
  {
    let width = T::try_from(self.width.get()).unwrap();
    let height = T::try_from(self.height.get()).unwrap();
    (width, height)
  }

  /// Sets the origin point (anchor position within the canvas bounds).
  pub fn set_origin(&mut self, origin: Origin) {
    self.origin = origin;
  }

  /// Gets the origin point (anchor position within the canvas bounds).
  pub fn origin(&self) -> Origin {
    self.origin.clone()
  }

  /// Flattens all layers in the canvas into a single layer.
  /// All layers will be merged into one layer and removed.
  pub fn flatten(&mut self) {
    self.update_canvas();
    let flattened_image = (*self.result).clone();
    self.layers.clear();
    let mut flattened_layer = LayerInner::new("Flattened Layer", std::sync::Arc::new(flattened_image));
    flattened_layer.set_visible(true);
    self.add_layer(flattened_layer);
  }

  /// Saves the project to the given path.
  pub fn save(&mut self, path: &str, options: Option<WriterOptions>) {
    if self.needs_recompose.get() {
      self.update_canvas();
    }
    self.result.save(path, options);
  }

  /// Converts the entire canvas into a single Image by flattening all layers and child canvases.
  pub fn as_image(&mut self) -> Image {
    if self.needs_recompose.get() {
      self.update_canvas();
    }
    self.result.as_ref().clone()
  }
}
