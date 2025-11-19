//! The internal layer implementation.

use core::Image;
use core::blend;
use core::blend::RGBA;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::Mutex;

use crate::Anchor;
use crate::Origin;
use crate::canvas::canvas_inner::CanvasInner;
use crate::effects::LayerEffects;

/// The internal layer implementation - provides the mutable reference API.
pub(crate) struct LayerInner {
  /// The name of the layer.
  name: String,
  /// The image data of the layer.
  image: Arc<Image>,
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
  canvas: Arc<Mutex<CanvasInner>>,
  /// The anchor point for positioning relative to the canvas.
  anchor: Option<Anchor>,
  /// The origin point within the layer that the anchor refers to.
  origin: Origin,
  /// The dimensions to use for anchoring calculations, separate from image dimensions.
  /// Used when effects like drop shadow expand the image beyond content bounds.
  anchor_dimensions: Option<(u32, u32)>,
  /// The positional offset applied when anchoring so effects like drop shadow don't shift placement.
  anchor_offset: (i32, i32),
  /// The effects that will be applied to this layer during rendering.
  effects: LayerEffects,
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

#[allow(dead_code)]
impl LayerInner {
  /// Creates a new layer with the given name, image, and canvas
  pub fn new(name: &str, image: Arc<Image>) -> LayerInner {
    let id = uuid::Uuid::new_v4().to_string();
    let tmp_canvas = Arc::new(Mutex::new(CanvasInner::new("Temporary")));

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
      origin: Origin::default(),
      anchor_dimensions: None,
      anchor_offset: (0, 0),
      effects: LayerEffects::new(),
    }
  }

  /// Sets the canvas reference for the layer.
  pub fn set_canvas(&mut self, canvas: Arc<Mutex<CanvasInner>>) {
    self.canvas = canvas.clone();
  }

  /// Gets the UUID of the layer.
  pub fn id(&self) -> &str {
    &self.id
  }

  /// Sets the blend mode of the layer.
  pub fn set_blend_mode(&mut self, blend_mode: fn(RGBA, RGBA) -> RGBA) {
    self.blend_mode = blend_mode;
    self.canvas.lock().unwrap().needs_recompose.set(true);
  }

  /// Sets the opacity of the layer.
  pub fn set_opacity(&mut self, opacity: f32) {
    self.opacity = opacity;
    self.canvas.lock().unwrap().needs_recompose.set(true);
  }

  /// Sets the visibility of the layer.
  pub fn set_visible(&mut self, visible: bool) {
    self.visible = visible;
    self.canvas.lock().unwrap().needs_recompose.set(true);
  }

  /// Sets the position of the layer.
  pub fn set_global_position(&mut self, x: i32, y: i32) {
    self.x = x;
    self.y = y;
    self.canvas.lock().unwrap().needs_recompose.set(true);
  }

  /// Sets the position of the layer relative to another layer
  pub fn set_relative_position(&mut self, x: i32, y: i32, layer: &LayerInner) {
    self.x = layer.x + x;
    self.y = layer.y + y;
    self.canvas.lock().unwrap().needs_recompose.set(true);
  }

  /// Sets the effects configuration for this layer and marks canvas dirty.
  pub fn set_effects(&mut self, effects: LayerEffects) {
    self.effects = effects;
    self.canvas.lock().unwrap().needs_recompose.set(true);
  }

  /// Sets the position of the layer to the given anchor point
  /// The anchor is stored and will be applied during render time (update_canvas)
  pub fn anchor_to_canvas(&mut self, anchor: Anchor) {
    self.anchor = Some(anchor);
  }

  /// Checks if the layer has an anchor set.
  pub fn has_anchor(&self) -> bool {
    self.anchor.is_some()
  }

  /// Sets the origin point within the layer for anchor positioning.
  pub fn set_origin(&mut self, origin: Origin) {
    self.origin = origin;
  }

  /// Gets the current origin point for anchor positioning.
  pub fn origin(&self) -> Origin {
    self.origin
  }

  /// Sets the positional offset used when applying anchor placement.
  pub fn set_anchor_offset(&mut self, p_x: i32, p_y: i32) {
    self.anchor_offset = (p_x, p_y);
  }

  /// Clears the positional offset used during anchor placement.
  pub fn clear_anchor_offset(&mut self) {
    self.anchor_offset = (0, 0);
  }

  /// Gets the anchor dimensions if set, otherwise returns image dimensions.
  pub fn anchor_dimensions(&self) -> (u32, u32) {
    self.anchor_dimensions.unwrap_or_else(|| self.image.dimensions::<u32>())
  }

  /// Sets the anchor dimensions to use for anchoring calculations.
  pub fn set_anchor_dimensions(&mut self, width: u32, height: u32) {
    self.anchor_dimensions = Some((width, height));
  }

  /// Clears the anchor dimensions, reverting to using image dimensions for anchoring.
  pub fn clear_anchor_dimensions(&mut self) {
    self.anchor_dimensions = None;
  }

  /// Applies the stored anchor to position the layer, given canvas dimensions
  /// This version directly updates x and y to avoid nested borrows of the canvas
  pub fn apply_anchor_with_canvas_dimensions(&mut self, canvas_width: i32, canvas_height: i32) {
    if let Some(anchor) = self.anchor {
      let (self_width, self_height) = self.anchor_dimensions();
      let (x, y) = anchor.calculate_position(canvas_width, canvas_height, self_width as i32, self_height as i32);
      // Position the layer directly at the calculated anchor position
      // The anchor calculation already handles proper centering/positioning
      self.x = x + self.anchor_offset.0;
      self.y = y + self.anchor_offset.1;
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

  /// Gets a mutable reference to the image using copy-on-write semantics.
  /// If the Arc has multiple owners, this will clone the image.
  pub fn image_mut(&mut self) -> &mut Image {
    Arc::make_mut(&mut self.image)
  }

  pub(crate) fn apply_pending_effects(&mut self) {
    let image_arc = self.image.clone();
    let effected_image = self.effects.apply(image_arc);
    self.image = effected_image;
  }

  /// Sets the index of the layer within the canvas's layer stack
  pub fn set_index(&mut self, index: usize) {
    // To avoid borrow conflicts, we need to find the current layer's index by ID
    let self_id = self.id.clone();

    let current_index = {
      let canvas = self.canvas.lock().unwrap();
      let idx = canvas.layers.iter().enumerate().position(|(_, layer_rc)| {
        // Compare by ID
        let layer = layer_rc.lock().unwrap();
        layer.id == self_id
      });
      idx
    };

    if let Some(current_idx) = current_index {
      let mut canvas = self.canvas.lock().unwrap();
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
    let self_id = self.id.clone();

    let current_index = {
      let canvas = self.canvas.lock().unwrap();
      canvas.layers.iter().enumerate().position(|(_, layer_rc)| {
        let layer = layer_rc.lock().unwrap();
        layer.id == self_id
      })
    };

    if let Some(current_idx) = current_index {
      let len = self.canvas.lock().unwrap().layers.len();
      if current_idx < len - 1 {
        self.set_index(current_idx + 1);
      }
    }
  }

  /// Moves the layer down one position in the stack (decreases its index by 1)
  /// Does nothing if the layer is already at the bottom
  pub fn move_down(&mut self) {
    let self_id = self.id.clone();

    let current_index = {
      let canvas = self.canvas.lock().unwrap();
      canvas.layers.iter().enumerate().position(|(_, layer_rc)| {
        let layer = layer_rc.lock().unwrap();
        layer.id == self_id
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
    let self_id = self.id.clone();

    let current_index = {
      let canvas = self.canvas.lock().unwrap();
      canvas.layers.iter().enumerate().position(|(_, layer_rc)| {
        let layer = layer_rc.lock().unwrap();
        layer.id == self_id
      })
    };

    if let Some(current_idx) = current_index {
      let len = self.canvas.lock().unwrap().layers.len();
      if current_idx < len - 1 {
        self.set_index(len - 1);
      }
    }
  }

  /// Moves the layer to the bottom of the stack
  pub fn move_to_bottom(&mut self) {
    let self_id = self.id.clone();

    let current_index = {
      let canvas = self.canvas.lock().unwrap();
      canvas.layers.iter().enumerate().position(|(_, layer_rc)| {
        let layer = layer_rc.lock().unwrap();
        layer.id == self_id
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
  /// This returns a Layer (the public wrapper), not the raw Rc<Mutex<LayerInner>>.
  pub fn duplicate(&self) -> super::Layer {
    let canvas = self.canvas.lock().unwrap();
    let layer = canvas
      .layers
      .iter()
      .find(|layer| layer.lock().unwrap().id() == self.id());
    let layer_ref = layer.unwrap().lock().unwrap();

    let mut new_layer = layer_ref.clone();
    new_layer.set_name(format!("{} clone", new_layer.name()).as_str());

    drop(layer_ref);
    drop(canvas);

    let mut canvas = self.canvas.lock().unwrap();

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
      origin: self.origin,
      anchor_dimensions: self.anchor_dimensions,
      anchor_offset: self.anchor_offset,
      effects: self.effects.clone(),
    }
  }
}
