//! The internal canvas implementation.

use core::Channels;
use core::Image;
use core::Rotate;
use core::WriterOptions;
// no direct `core::blend` name use; imports here are filtered as needed
use core::blend::blend_images_at_with_opacity;
use std::cell::Cell;
use std::sync::Arc;
use std::sync::Mutex;

use crate::Anchor;
use crate::Canvas;
use crate::LayerEffects;
use crate::canvas::AddCanvasOptions;
use crate::canvas::Origin;

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
  /// The blend mode to use when compositing this canvas into its parent.
  /// This mirrors LayerInner::blend_mode, but for a canvas group.
  pub blend_mode: fn(core::blend::RGBA, core::blend::RGBA) -> core::blend::RGBA,
  /// When true, this canvas passes children through to the parent, rather than treating
  /// the canvas as a single flattened composite.
  pub pass_through: bool,
  /// Canvas opacity when composited into a parent.
  pub opacity: Cell<f32>,
  /// The origin point (anchor position within the canvas bounds).
  origin: Origin,
  /// The effects applied to the entire canvas.
  effects: LayerEffects,
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
      blend_mode: core::blend::normal,
      pass_through: false,
      opacity: Cell::new(1.0),
      origin: Origin::default(),
      effects: LayerEffects::new(),
    }
  }

  /// Creates a new project with the given name and a blank canvas with the given dimensions.
  pub fn new_blank(name: &str, width: u32, height: u32) -> CanvasInner {
    let mut canvas = CanvasInner::new(name);
    canvas.set_canvas_size(width, height);
    canvas
  }

  /// Creates a new project with the given name and a canvas from the image at the given path.
  pub fn new_from_path(name: &str, path: &str, _options: impl Into<Option<NewLayerOptions>>) -> CanvasInner {
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
  pub fn add_canvas_rc(&mut self, canvas_rc: Arc<Mutex<Canvas>>, options: impl Into<Option<AddCanvasOptions>>) {
    let options = options.into();
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

    // Composite child canvases and local layers into the canvas (handles pass-through logic)
    self.composite_into(&mut canvas, 0, 0);

    // Note: local layers are already blended by composite_into in the pass-through path
    // Apply canvas-level effects (if any) to the composite
    let mut final_image = canvas;
    if !self.effects.drop_shadow.is_none() || !self.effects.stroke.is_none() {
      // We need to compute padding/offset and update origin/position as necessary.
      // offset currently unused; keep underscore to suppress unused variable warning while keeping layout
      let (img, _offset, _content_dims) = self.effects.apply_with_offset(Arc::new(final_image)).into_tuple();
      // offset is padding that indicates where the original content is placed inside img
      final_image = (*img).clone();
      // If the canvas has a parent, we may need to set anchor offset on this canvas; for now we just store result
    }

    self.result = Box::new(final_image);
    // Mark canvas as recomposed
    self.needs_recompose.set(false);
  }

  /// Composite this canvas' layers and children into the destination image, honoring pass-through.
  /// `offset_x` and `offset_y` are positions applied to the layers (accumulated parent offsets).
  pub fn composite_into(&self, dest: &mut Image, offset_x: i32, offset_y: i32) {
    // Composite child canvases first (so child layers can be behind local layers)
    for child_canvas_rc in self.canvases.iter() {
      let child_canvas = child_canvas_rc.lock().unwrap();
      let (child_width, child_height) = child_canvas.dimensions::<u32>();
      if child_width == 0 || child_height == 0 {
        continue;
      }

      let (child_x, child_y) = child_canvas.position();
      let dest_x = offset_x + child_x;
      let dest_y = offset_y + child_y;

      if child_canvas.pass_through() && child_canvas.rotation().is_none() {
        // Composite child's layers directly into `dest` with accumulated offsets
        let child_inner_rc = child_canvas.inner_rc();
        let child_inner = child_inner_rc.lock().unwrap();
        child_inner.composite_into(dest, dest_x, dest_y);
      } else {
        // Composite child's flattened result
        let mut child_result = child_canvas.get_result_image();
        if let Some(rotation_degrees) = child_canvas.rotation() {
          child_result.rotate(rotation_degrees, None);
        }
        let child_blend = child_canvas.blend_mode();
        let child_opacity = child_canvas.opacity();
        blend_images_at_with_opacity(dest, &child_result, 0, 0, dest_x, dest_y, child_blend, child_opacity);
      }
    }

    // Composite local layers into destination
    let canvas_dims = (self.width.get() as i32, self.height.get() as i32);
    for layer in self.layers.iter() {
      let mut layer_ref = layer.lock().unwrap();
      layer_ref.apply_pending_effects();
      layer_ref.apply_anchor_with_canvas_dimensions(canvas_dims.0, canvas_dims.1);
      if layer_ref.is_visible() {
        let opacity = layer_ref.opacity().clamp(0.0, 1.0);
        let blend = layer_ref.blend_mode();
        let (x, y) = layer_ref.position();
        let image = layer_ref.image();
        blend_images_at_with_opacity(dest, &image, 0, 0, offset_x + x, offset_y + y, blend, opacity);
      }
    }
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
  pub fn set_parent(&self, parent: impl Into<Option<Arc<Mutex<CanvasInner>>>>) {
    *self.parent.lock().unwrap() = parent.into();
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
  pub fn set_rotation(&mut self, degrees: impl Into<Option<f32>>) {
    self.rotation.set(degrees.into());
  }

  /// Gets the rotation in degrees for the canvas within its parent.
  pub fn rotation(&self) -> Option<f32> {
    self.rotation.get()
  }

  /// Sets the blend mode used when compositing this canvas into a parent.
  pub fn set_blend_mode(&mut self, blend: fn(core::blend::RGBA, core::blend::RGBA) -> core::blend::RGBA) {
    self.blend_mode = blend;
    self.needs_recompose.set(true);
  }

  /// Gets the blend mode used for compositing this canvas.
  pub fn blend_mode(&self) -> fn(core::blend::RGBA, core::blend::RGBA) -> core::blend::RGBA {
    self.blend_mode
  }

  /// Sets whether this canvas is pass-through.
  pub fn set_pass_through(&mut self, pass: bool) {
    self.pass_through = pass;
    self.needs_recompose.set(true);
  }

  /// Gets whether this canvas is a pass-through group.
  pub fn pass_through(&self) -> bool {
    self.pass_through
  }

  /// Sets the canvas opacity (0.0-1.0) when composited into the parent.
  pub fn set_opacity(&mut self, opacity: f32) {
    self.opacity.set(opacity.clamp(0.0, 1.0));
    self.needs_recompose.set(true);
  }

  /// Gets the canvas opacity.
  pub fn opacity(&self) -> f32 {
    self.opacity.get()
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
  pub fn save(&mut self, path: &str, options: impl Into<Option<WriterOptions>>) {
    let start = std::time::Instant::now();
    if self.needs_recompose.get() {
      self.update_canvas();
    }
    println!("Canvas recomposed in {:?}", start.elapsed());
    self.result.save(path, options.into());
  }

  pub fn set_effects(&mut self, effects: crate::LayerEffects) {
    self.effects = effects;
    self.needs_recompose.set(true);
  }

  /// Converts the entire canvas into a single Image by flattening all layers and child canvases.
  pub fn as_image(&mut self) -> Image {
    if self.needs_recompose.get() {
      self.update_canvas();
    }
    self.result.as_ref().clone()
  }
}
