//! The internal layer implementation.

use crate::{
  canvas::anchor::Anchor,
  canvas::canvas_inner::CanvasInner,
  combine::blend::{self, RGBA},
  image::Image,
};

use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

/// The internal layer implementation - provides the mutable reference API.
pub(crate) struct LayerInner {
  /// The name of the layer.
  name: String,
  /// The image data of the layer.
  image: Image,
  /// Whether the layer is visible.
  visible: bool,
  /// The opacity of the layer.
  opacity: f32,
  /// The blend mode of the layer.
  blend_mode: fn(RGBA, RGBA) -> RGBA,
  /// The x position of the image within the layer.
  x: i32,
  /// The y position of the image within the layer.
  y: i32,
  /// A UUID for the layer.
  id: String,
  /// Reference to the canvas.
  canvas: Rc<RefCell<CanvasInner>>,
  /// The anchor point for positioning relative to the canvas.
  anchor: Option<Anchor>,
}

impl Debug for LayerInner {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("LayerInner")
      .field("id", &self.id())
      .field("name", &self.name)
      .field("dimensions", &self.image.dimensions::<u32>())
      .field("visible", &self.visible)
      .field("opacity", &self.opacity)
      .field("blend_mode", &"function pointer")
      .field("position", &self.position())
      .finish()
  }
}

impl LayerInner {
  /// Creates a new layer with the given name, image, and canvas
  pub fn new(name: &str, image: Image) -> LayerInner {
    let id = uuid::Uuid::new_v4().to_string();
    let tmp_canvas = Rc::new(RefCell::new(CanvasInner::new("Temporary")));

    LayerInner {
      id,
      name: name.to_string(),
      image,
      visible: true,
      opacity: 1.0,
      blend_mode: blend::normal,
      x: 0,
      y: 0,
      canvas: tmp_canvas,
      anchor: None,
    }
  }

  /// Sets the canvas reference for the layer.
  pub fn set_canvas(&mut self, canvas: Rc<RefCell<CanvasInner>>) {
    self.canvas = canvas.clone();
  }

  /// Gets the UUID of the layer.
  pub fn id(&self) -> &str {
    &self.id
  }

  /// Sets the blend mode of the layer.
  pub fn set_blend_mode(&mut self, blend_mode: fn(RGBA, RGBA) -> RGBA) {
    self.blend_mode = blend_mode;
    self.canvas.borrow_mut().needs_recompose.set(true);
  }

  /// Sets the opacity of the layer.
  pub fn set_opacity(&mut self, opacity: f32) {
    self.opacity = opacity;
    self.canvas.borrow_mut().needs_recompose.set(true);
  }

  /// Sets the visibility of the layer.
  pub fn set_visible(&mut self, visible: bool) {
    self.visible = visible;
    self.canvas.borrow_mut().needs_recompose.set(true);
  }

  /// Sets the position of the layer.
  pub fn set_global_position(&mut self, x: i32, y: i32) {
    self.x = x;
    self.y = y;
    self.canvas.borrow_mut().needs_recompose.set(true);
  }

  /// Sets the position of the layer relative to another layer
  pub fn set_relative_position(&mut self, x: i32, y: i32, layer: &LayerInner) {
    self.x = layer.x + x;
    self.y = layer.y + y;
    self.canvas.borrow_mut().needs_recompose.set(true);
  }

  /// Sets the position of the layer to the given anchor point
  /// The anchor is stored and will be applied during render time (update_canvas)
  pub fn anchor_to_canvas(&mut self, anchor: Anchor) {
    self.anchor = Some(anchor);
  }

  /// Applies the stored anchor to position the layer, given canvas dimensions
  /// This version directly updates x and y to avoid nested borrows of the canvas
  pub fn apply_anchor_with_canvas_dimensions(&mut self, canvas_width: i32, canvas_height: i32) {
    if let Some(anchor) = self.anchor {
      let (self_width, self_height) = self.image.dimensions::<i32>();
      let (x, y) = anchor.calculate_position(canvas_width, canvas_height, self_width, self_height);
      // Directly update position fields to avoid nested borrow
      self.x = x;
      self.y = y;
    }
  }

  /// Gets the dimensions of the layer
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u32>,
    <T as TryFrom<u32>>::Error: std::fmt::Debug,
  {
    self.image.dimensions::<T>()
  }

  /// Gets the position of the image within the layer
  pub fn position(&self) -> (i32, i32) {
    (self.x, self.y)
  }

  /// Gets the name of the layer
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Sets the name of the layer
  pub fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }

  /// Gets the opacity of the layer
  pub fn opacity(&self) -> f32 {
    self.opacity
  }

  /// Gets the blend mode of the layer
  pub fn blend_mode(&self) -> fn(RGBA, RGBA) -> RGBA {
    self.blend_mode
  }

  /// Gets whether the layer is visible
  pub fn is_visible(&self) -> bool {
    self.visible
  }

  /// Gets a reference to the image
  pub fn image(&self) -> &Image {
    &self.image
  }

  /// Gets a mutable reference to the image
  pub fn image_mut(&mut self) -> &mut Image {
    &mut self.image
  }

  /// Sets the index of the layer within the canvas's layer stack
  pub fn set_index(&mut self, index: usize) {
    // To avoid borrow conflicts, we need to find the current layer's index
    // We'll use pointer comparison: find which Rc<RefCell<LayerInner>> contains this &mut self
    // by comparing the pointer address
    let self_ptr = self as *const LayerInner;

    let current_index = {
      let canvas = self.canvas.borrow();
      let idx = canvas.layers.iter().enumerate().position(|(_, layer_rc)| {
        // Compare pointers without borrowing
        let layer_ptr = layer_rc.as_ptr();
        self_ptr == layer_ptr
      });
      idx
    };

    if let Some(current_idx) = current_index {
      let mut canvas = self.canvas.borrow_mut();
      // Directly manipulate layers vec
      if current_idx != index && index <= canvas.layers.len() {
        let layer = canvas.layers.remove(current_idx);
        canvas.layers.insert(index, layer);
      }
      // Mark canvas as needing recomposition since layer order changed
      canvas.needs_recompose.set(true);
    }
  }

  /// Moves the layer up one position in the stack (increases its index by 1)
  /// Does nothing if the layer is already at the top
  pub fn move_up(&mut self) {
    let self_ptr = self as *const LayerInner;

    let current_index = {
      let canvas = self.canvas.borrow();
      canvas.layers.iter().enumerate().position(|(_, layer_rc)| {
        let layer_ptr = layer_rc.as_ptr();
        self_ptr == layer_ptr
      })
    };

    if let Some(current_idx) = current_index {
      let len = self.canvas.borrow().layers.len();
      if current_idx < len - 1 {
        self.set_index(current_idx + 1);
      }
    }
  }

  /// Moves the layer down one position in the stack (decreases its index by 1)
  /// Does nothing if the layer is already at the bottom
  pub fn move_down(&mut self) {
    let self_ptr = self as *const LayerInner;

    let current_index = {
      let canvas = self.canvas.borrow();
      canvas.layers.iter().enumerate().position(|(_, layer_rc)| {
        let layer_ptr = layer_rc.as_ptr();
        self_ptr == layer_ptr
      })
    };

    if let Some(current_idx) = current_index {
      if current_idx > 0 {
        self.set_index(current_idx - 1);
      }
    }
  }

  /// Moves the layer to the top of the stack
  pub fn move_to_top(&mut self) {
    let self_ptr = self as *const LayerInner;

    let current_index = {
      let canvas = self.canvas.borrow();
      canvas.layers.iter().enumerate().position(|(_, layer_rc)| {
        let layer_ptr = layer_rc.as_ptr();
        self_ptr == layer_ptr
      })
    };

    if let Some(current_idx) = current_index {
      let len = self.canvas.borrow().layers.len();
      if current_idx < len - 1 {
        self.set_index(len - 1);
      }
    }
  }

  /// Moves the layer to the bottom of the stack
  pub fn move_to_bottom(&mut self) {
    let self_ptr = self as *const LayerInner;

    let current_index = {
      let canvas = self.canvas.borrow();
      canvas.layers.iter().enumerate().position(|(_, layer_rc)| {
        let layer_ptr = layer_rc.as_ptr();
        self_ptr == layer_ptr
      })
    };

    if let Some(current_idx) = current_index {
      if current_idx > 0 {
        self.set_index(0);
      }
    }
  }

  /// Sets the position of the layer without triggering a recompose.
  /// This is used internally when resizing/cropping the canvas.
  pub(crate) fn set_position_internal(&mut self, x: i32, y: i32) {
    self.x = x;
    self.y = y;
  }

  /// Duplicates the layer within the same canvas.
  /// This returns a Layer (the public wrapper), not the raw Rc<RefCell<LayerInner>>.
  pub fn duplicate(&self) -> super::Layer {
    let canvas = self.canvas.borrow_mut();
    let layer = canvas.layers.iter().find(|layer| layer.borrow().id() == self.id());
    let layer_ref = layer.unwrap().borrow();

    let mut new_layer = layer_ref.clone();
    new_layer.set_name(format!("{} clone", new_layer.name()).as_str());

    drop(layer_ref);
    drop(canvas);

    let mut canvas = self.canvas.borrow_mut();

    let layer_rc = canvas.add_layer(new_layer);
    super::Layer::from_inner(layer_rc)
  }
}

impl Clone for LayerInner {
  fn clone(&self) -> Self {
    LayerInner {
      id: uuid::Uuid::new_v4().to_string(),
      name: self.name.clone(),
      image: self.image.clone(),
      blend_mode: self.blend_mode,
      opacity: self.opacity,
      visible: self.visible,
      x: self.x,
      y: self.y,
      canvas: self.canvas.clone(),
      anchor: self.anchor,
    }
  }
}
