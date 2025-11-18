//! The Canvas public API struct.

use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use crate::canvas::AddCanvasOptions;
use crate::canvas::Origin;
use crate::image::Image;
use crate::utils::fs::WriterOptions;

use super::canvas_inner::CanvasInner;
use super::canvas_transform::CanvasTransform;
use super::layer::Layer;
use super::layer_inner::LayerInner;
use super::layer_options_applier;
use super::options_new_layer::NewLayerOptions;

impl Debug for Canvas {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let canvas = self.inner_canvas.lock().unwrap();
    f.debug_struct("Canvas")
      .field("id", &canvas.id)
      .field("name", &canvas.name)
      .field("width", &canvas.width.get())
      .field("height", &canvas.height.get())
      .field("layer_count", &canvas.layers.len())
      .field("layers", &canvas.layers)
      .finish()
  }
}

/// A canvas is a group of layers that can be manipulated together.
/// They can be moved, resized, cropped, and saved as a single image.
/// Multiple canvases can be children of other canvases to create complex compositions.
/// Calling save on the canvas will merge all the layers and save the final image.
/// ```ignore
/// let canvas = Canvas::new("My Project");
/// let layers = canvas.layers_mut();
/// layers.add_layer_from_path("Layer1", "path/to/image.png", None);
/// canvas.save("path/to/output.png", None);
/// ```
pub struct Canvas {
  /// Reference to the inner canvas.
  inner_canvas: Arc<Mutex<CanvasInner>>,
}

impl Canvas {
  /// Creates a new project with the given name and an empty canvas of a size of 0x0.
  pub fn new(name: &str) -> Canvas {
    Canvas {
      inner_canvas: Arc::new(Mutex::new(CanvasInner::new(name))),
    }
  }

  /// Creates a new canvas with the given name and a blank canvas of the given size.
  pub fn new_blank(name: &str, width: u32, height: u32) -> Canvas {
    Canvas {
      inner_canvas: Arc::new(Mutex::new(CanvasInner::new_blank(name, width, height))),
    }
  }

  /// Creates a new project with the given name and a canvas from a path.
  pub fn new_from_path(name: &str, path: &str, options: impl Into<Option<NewLayerOptions>>) -> Canvas {
    Canvas {
      inner_canvas: Arc::new(Mutex::new(CanvasInner::new_from_path(name, path, options))),
    }
  }
  /// Saves the canvas to a file.
  pub fn save(&self, path: &str, options: impl Into<Option<WriterOptions>>) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.save(path, options);
  }

  /// Converts the entire canvas into a single Image by flattening all layers and child canvases.
  pub fn as_image(&self) -> Image {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.as_image()
  }

  /// Flattens all layers into a single layer.
  /// All layers will be merged into one layer and removed.
  pub fn flatten(self) -> Self {
    {
      let mut canvas = self.inner_canvas.lock().unwrap();
      canvas.flatten();
    }
    self
  }

  /// Updates the canvas by re-compositing all layers and child canvases.
  ///
  /// Internal-only: composition is triggered automatically by `save` and `as_image`.
  /// Returns a new `Canvas` wrapper referencing the same inner canvas.
  pub(crate) fn update_canvas(&self) -> Canvas {
    {
      let mut canvas = self.inner_canvas.lock().unwrap();
      canvas.update_canvas();
    }

    Canvas {
      inner_canvas: self.inner_canvas.clone(),
    }
  }

  /// Gets the dimensions of the canvas
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u32>,
    <T as TryFrom<u32>>::Error: std::fmt::Debug,
  {
    let canvas = self.inner_canvas.lock().unwrap();
    let width = T::try_from(canvas.width.get()).unwrap();
    let height = T::try_from(canvas.height.get()).unwrap();
    (width, height)
  }

  /// Gets the position of the canvas within its parent
  pub fn position(&self) -> (i32, i32) {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.position()
  }

  /// Sets the position of the canvas within its parent
  pub fn set_position(&self, x: i32, y: i32) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.set_global_position(x, y);
  }

  /// Sets the rotation in degrees for the canvas within its parent
  pub fn set_rotation(&mut self, degrees: impl Into<Option<f32>>) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.set_rotation(degrees);
  }

  /// Gets the rotation in degrees for the canvas within its parent
  pub fn rotation(&self) -> Option<f32> {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.rotation()
  }

  /// Sets the origin point (anchor position within the canvas bounds).
  pub fn set_origin(&self, origin: Origin) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.set_origin(origin);
  }

  /// Gets the origin point (anchor position within the canvas bounds).
  pub fn origin(&self) -> Origin {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.origin()
  }

  /// Adds a new layer from a file path using a fluent API.
  /// This returns `Self` to allow method chaining without needing `layers_mut()`.
  ///
  /// # Example
  /// ```ignore
  /// let project = Canvas::new("My Project")
  ///     .add_layer_from_path("Background", "bg.png", None)
  ///     .add_layer_from_path("Overlay", "overlay.png", Some(NewLayerOptions {
  ///         anchor: Some(Anchor::TopRight),
  ///     }));
  /// project.save("output.png", None);
  /// ```
  pub fn add_layer_from_path(self, name: &str, path: &str, options: impl Into<Option<NewLayerOptions>>) -> Self {
    let image = std::sync::Arc::new(Image::new_from_path(path));
    self.add_layer_from_image(name, image, options)
  }

  /// Adds a new canvas as a child canvas.
  pub fn add_canvas(&self, canvas: Canvas, options: impl Into<Option<AddCanvasOptions>>) {
    let canvas_rc = Arc::new(Mutex::new(canvas));

    // Set the parent reference on the child canvas
    self
      .inner_canvas
      .lock()
      .unwrap()
      .set_parent(Some(self.inner_canvas.clone()));

    let mut inner_canvas = self.inner_canvas.lock().unwrap();
    inner_canvas.add_canvas_rc(canvas_rc, options);
  }
  /// Sets the position of this canvas to the given anchor point within its parent canvas.
  pub fn anchor_to_canvas(&self, anchor: crate::Anchor) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.anchor_to_canvas(anchor);
  }

  /// Applies the stored anchor to position the canvas using parent dimensions
  pub(crate) fn apply_anchor_with_parent_dimensions(&self, parent_width: i32, parent_height: i32) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.apply_anchor_with_parent_dimensions(parent_width, parent_height);
  }

  /// Adds a new layer from an image using a fluent API.
  /// This returns `Self` to allow method chaining.
  ///
  /// # Example
  /// ```ignore
  /// let img = Image::new_from_path("assets/image.png");
  /// let project = Canvas::new("My Project")
  ///     .add_layer_from_image("White Layer", img, None);
  /// ```
  pub fn add_layer_from_image<I: Into<Arc<Image>>>(
    self, name: &str, image: I, options: impl Into<Option<NewLayerOptions>>,
  ) -> Self {
    let image_arc = image.into();
    let canvas_rc = self.inner_canvas.clone();
    let options = options.into();
    let mut layer = LayerInner::new(name, image_arc);
    layer.set_canvas(canvas_rc);

    let layer_rc = Arc::new(Mutex::new(layer));

    // Determine if this is the first layer before modifying canvas
    let is_first_layer = {
      let canvas = self.inner_canvas.lock().unwrap();
      canvas.width.get() == 0 && canvas.height.get() == 0
    };

    // Add to canvas
    {
      let mut canvas = self.inner_canvas.lock().unwrap();
      let (width, height) = layer_rc.lock().unwrap().dimensions::<u32>();
      canvas.layers.push(layer_rc.clone());

      // Set canvas size from first layer (before applying size options)
      if is_first_layer {
        canvas.set_canvas_size(width, height);
      }
    }

    // Apply options
    {
      let mut layer_mut = layer_rc.lock().unwrap();
      let (canvas_width, canvas_height) = self.dimensions();
      layer_options_applier::apply_layer_options(&mut layer_mut, options.as_ref(), canvas_width, canvas_height);
    }

    // If this was the first layer and size options were applied, update canvas size to match resized layer
    if is_first_layer {
      let (new_width, new_height) = layer_rc.lock().unwrap().dimensions::<u32>();
      let mut canvas = self.inner_canvas.lock().unwrap();
      if new_width > 0 && new_height > 0 {
        canvas.set_canvas_size(new_width, new_height);
      }
    }

    self
  }

  /// Gets a layer by its index.
  /// Returns None if the index is out of bounds.
  pub fn get_layer_by_index(&self, index: usize) -> Option<Layer> {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.layers.get(index).cloned().map(Layer::from_inner)
  }

  /// Gets a layer by its name.
  /// Returns the first layer with the matching name, or None if not found.
  pub fn get_layer_by_name(&self, name: &str) -> Option<Layer> {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas
      .layers
      .iter()
      .find(|layer_rc| {
        let layer = layer_rc.lock().unwrap();
        layer.name() == name
      })
      .cloned()
      .map(Layer::from_inner)
  }

  /// Gets all layers in the canvas.
  pub fn layers(&self) -> Vec<Layer> {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.layers.iter().cloned().map(Layer::from_inner).collect()
  }

  /// Gets the number of layers in the canvas.
  pub fn layer_count(&self) -> usize {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.layers.len()
  }

  /// Gets a clone of the result image (internal use only).
  /// This is used when compositing child canvases.
  pub(crate) fn get_result_image(&self) -> Image {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.get_result_image()
  }

  /// Returns a handler for applying transform operations to the canvas
  pub fn transform(&self) -> CanvasTransform {
    CanvasTransform::new(self.inner_canvas.clone())
  }
}
