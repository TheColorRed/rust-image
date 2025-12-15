//! The Canvas public API struct.

use abra_core::image::image_ext::CoreImageFsExt;
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use crate::canvas::AddCanvasOptions;
use crate::canvas::Origin;

use abra_core::Image;
use abra_core::WriterOptions;

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

impl Default for Canvas {
  fn default() -> Self {
    Canvas::new("Empty Canvas")
  }
}

impl Canvas {
  /// Creates a new project with the given name and an empty canvas of a size of 0x0.
  pub fn new(p_name: impl Into<String>) -> Canvas {
    Canvas {
      inner_canvas: Arc::new(Mutex::new(CanvasInner::new(p_name))),
    }
  }

  /// Creates a new canvas with the given name and a blank canvas of the given size.
  pub fn new_blank(p_name: impl Into<String>, p_width: u32, p_height: u32) -> Canvas {
    Canvas {
      inner_canvas: Arc::new(Mutex::new(CanvasInner::new_blank(p_name, p_width, p_height))),
    }
  }

  /// Creates a new project with the given name and a canvas from a path.
  pub fn new_from_path(
    p_name: impl Into<String>, p_path: impl Into<String>, options: impl Into<Option<NewLayerOptions>>,
  ) -> Canvas {
    // Create the inner canvas first, then ensure all layers inside the inner
    // have their `canvas` reference set to this `Arc<Mutex<CanvasInner>>`.
    let inner = Arc::new(Mutex::new(CanvasInner::new_from_path(p_name, p_path, options)));
    {
      // Set the canvas reference for each layer to the created Arc so
      // layer-level operations mark the correct parent canvas as dirty.
      let inner_clone = inner.clone();
      let mut guard = inner.lock().unwrap();
      for layer_rc in guard.layers.iter_mut() {
        layer_rc.lock().unwrap().set_canvas(inner_clone.clone());
      }
    }
    Canvas { inner_canvas: inner }
  }
  /// Gets the unique ID of the canvas.
  pub fn id(&self) -> String {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.id.clone()
  }

  /// Gets the name of the canvas.
  pub fn name(&self) -> String {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.name.clone()
  }

  /// Saves the canvas to a file.
  pub fn save(&self, p_path: impl Into<String>, p_options: impl Into<Option<WriterOptions>>) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.save(p_path, p_options);
  }

  /// Converts the entire canvas into a single Image by flattening all layers and child canvases
  /// without modifying the original canvas.
  /// Returns a new Image instance containing the flattened canvas.
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
  pub fn add_layer_from_path(
    self, p_name: impl Into<String>, p_path: impl Into<String>, p_options: impl Into<Option<NewLayerOptions>>,
  ) -> Self {
    let image = Arc::new(Image::new_from_path(p_path));
    self.add_layer_from_image(p_name, image, p_options)
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
    self, p_name: impl Into<String>, image: I, p_options: impl Into<Option<NewLayerOptions>>,
  ) -> Self {
    let image_arc = image.into();
    let canvas_rc = self.inner_canvas.clone();
    let options = p_options.into();
    let mut layer = LayerInner::new(p_name.into(), image_arc);
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

  /// Adds a new adjustment layer to the canvas.
  /// This returns `Self` to allow method chaining.
  pub fn add_adjustment_layer(self, p_name: impl Into<String>, adjustment_type: crate::AdjustmentLayerType) -> Self {
    let canvas_rc = self.inner_canvas.clone();
    let mut canvas = canvas_rc.lock().unwrap();
    canvas.add_adjustment_layer(p_name, adjustment_type);
    self
  }

  /// Deletes a layer by its ID from the canvas.
  /// If the layer is not found, no action is taken.
  pub fn delete_layer_by_id(&self, layer_id: &str) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.delete_layer_by_id(layer_id);
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

  /// Sets the effects to apply to the entire canvas.
  /// - `p_effects`: the LayerEffects to apply.
  pub fn set_effects(&self, p_effects: crate::LayerEffects) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.set_effects(p_effects);
  }

  /// Gets a layer by its index.
  /// Returns None if the index is out of bounds.
  pub fn get_layer_by_index(&self, index: usize) -> Option<Layer> {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.layers.get(index).cloned().map(Layer::from_inner)
  }

  /// Gets a layer by its UUID.
  /// Returns None if the index is out of bounds.
  pub fn get_layer_by_id(&self, id: &str) -> Option<Layer> {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas
      .layers
      .iter()
      .find(|layer_rc| {
        let layer = layer_rc.lock().unwrap();
        layer.id() == id
      })
      .cloned()
      .map(Layer::from_inner)
  }

  /// Gets a layer by its name.
  /// Returns the first layer with the matching name, or None if not found.
  pub fn get_layer_by_name(&self, name: impl Into<String>) -> Option<Layer> {
    let name = name.into();
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

  /// Reorders the layers in the layer stack according to the given array of layer IDs.
  /// If any IDs are not found or if there are duplicate IDs, no changes are made.
  /// # Parameters
  /// - `new_order_ids`: A vector of layer IDs in the desired order (from bottom to top).
  pub fn reorder_layers_by_id(&self, new_order_ids: Vec<String>) {
    // Make sure all IDs are unique
    // If there are duplicates, exit early
    let unique_ids: HashSet<_> = new_order_ids.iter().collect();
    if unique_ids.len() != new_order_ids.len() {
      return;
    }

    // Reorder the layers
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.layers = new_order_ids
      .iter()
      .rev()
      .filter_map(|id| {
        canvas
          .layers
          .iter()
          .find(|layer_rc| layer_rc.lock().unwrap().id() == *id)
          .cloned()
      })
      .collect();

    canvas.mark_dirty();
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

  /// Internal: returns cloned Arc<Mutex<CanvasInner>> so other modules can access internal inner.
  pub(crate) fn inner_rc(&self) -> Arc<Mutex<CanvasInner>> {
    self.inner_canvas.clone()
  }

  /// Sets the blend mode used when compositing this canvas into a parent.
  pub fn set_blend_mode(
    &self, blend_mode: fn(abra_core::blend::RGBA, abra_core::blend::RGBA) -> abra_core::blend::RGBA,
  ) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.set_blend_mode(blend_mode);
  }

  /// Gets the blend mode used when compositing this canvas into a parent.
  pub fn blend_mode(&self) -> fn(abra_core::blend::RGBA, abra_core::blend::RGBA) -> abra_core::blend::RGBA {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.blend_mode()
  }

  /// Sets whether this canvas is pass-through.
  pub fn set_pass_through(&self, pass: bool) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.set_pass_through(pass);
  }

  /// Gets whether this canvas is a pass-through group.
  pub fn pass_through(&self) -> bool {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.pass_through()
  }

  /// Sets the canvas opacity (0.0 - 1.0).
  pub fn set_opacity(&self, opacity: f32) {
    let mut canvas = self.inner_canvas.lock().unwrap();
    canvas.set_opacity(opacity);
  }

  /// Gets the canvas opacity (0.0 - 1.0).
  pub fn opacity(&self) -> f32 {
    let canvas = self.inner_canvas.lock().unwrap();
    canvas.opacity()
  }

  /// Returns a handler for applying transform operations to the canvas
  pub fn transform(&self) -> CanvasTransform {
    CanvasTransform::new(self.inner_canvas.clone())
  }
}
