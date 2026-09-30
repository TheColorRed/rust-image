//! The internal canvas implementation.

use abra_core::BlendMode;
use abra_core::Image;
use abra_core::IntoNumber;
use abra_core::Resolution;
use abra_core::WriterOptions;
use abra_core::blend::blend as blend_images;
use abra_core::image::image_ext::*;
use abra_core::writer;
use abra_core::{ResizeTarget, Size, Transform, TransformAlgorithm};
use std::cell::Cell;
use std::sync::Arc;
use std::sync::Mutex;

use crate::Anchor;
use crate::LayerEffects;
use crate::canvas::AddCanvasOptions;
use crate::canvas::Origin;

use super::layer_inner::{LayerInner, LayerOperation};
use super::options_new_layer::NewLayerOptions;

/// The internal canvas implementation - provides the mutable reference API.
pub(crate) struct CanvasInner<'a> {
  /// The unique identifier of the canvas.
  pub id: String,
  /// The name of the canvas.
  pub name: String,
  /// Child canvases in the canvas (store inner directly to avoid lifetime / Mutex invariance issues).
  pub(crate) canvases: Vec<Arc<Mutex<CanvasInner<'a>>>>,
  /// The layers in the canvas.
  pub layers: Vec<Arc<Mutex<LayerInner<'a>>>>,
  /// The width of the canvas.
  pub width: Cell<u32>,
  /// The height of the canvas.
  pub height: Cell<u32>,
  /// The physical pixel density of the document.
  pub resolution: Resolution,
  /// Whether this canvas explicitly overrides an inherited parent resolution.
  resolution_is_explicit: bool,
  /// The x position of the canvas within its parent.
  x: Cell<i32>,
  /// The y position of the canvas within its parent.
  y: Cell<i32>,
  /// This is the final image that is created by blending all the layers.
  pub result: Image,
  /// Whether the canvas has been manually resized and needs to skip update_canvas on save.
  needs_recompose: Cell<bool>,
  /// The anchor point for positioning relative to the parent canvas.
  anchor: Option<Anchor>,
  /// The rotation in degrees for positioning within the parent canvas.
  rotation: Cell<Option<f32>>,
  /// The blend mode to use when compositing this canvas into its parent.
  /// This mirrors LayerInner::blend_mode, but for a canvas group.
  pub blend_mode: BlendMode,
  /// When true, this canvas passes children through to the parent, rather than treating
  /// the canvas as a single flattened composite.
  pub pass_through: bool,
  /// Canvas opacity when composited into a parent.
  pub opacity: Cell<f32>,
  /// The origin point (anchor position within the canvas bounds).
  origin: Origin,
  /// The effects applied to the entire canvas.
  effects: LayerEffects<'a>,
}

impl<'a> CanvasInner<'a> {
  /// Creates a new canvas with the given name and an empty canvas of a size of 0x0.
  pub fn new(p_name: impl Into<String>) -> Self {
    let p_name = p_name.into();
    CanvasInner {
      id: uuid::Uuid::new_v4().to_string(),
      name: p_name.to_string(),
      result: Image::new(0, 0),
      canvases: vec![],
      layers: vec![],
      width: Cell::new(0),
      height: Cell::new(0),
      resolution: Resolution::default(),
      resolution_is_explicit: false,
      x: Cell::new(0),
      y: Cell::new(0),
      needs_recompose: Cell::new(true),
      anchor: None,
      rotation: Cell::new(None),
      blend_mode: abra_core::BlendMode::Normal,
      pass_through: false,
      opacity: Cell::new(1.0),
      origin: Origin::default(),
      effects: LayerEffects::new(),
    }
  }

  /// Creates a new project with the given name and a blank canvas with the given dimensions.
  pub fn new_blank(p_name: impl Into<String>, p_width: impl IntoNumber, p_height: impl IntoNumber) -> Self {
    let mut canvas = CanvasInner::new(&p_name.into());
    canvas.set_canvas_size(p_width, p_height);
    canvas
  }

  /// Creates a new project with the given name and a canvas from the image at the given path.
  /// The image is loaded from the path and used as the initial canvas content.
  /// The canvas size is set to the image dimensions and the image is added as the first layer.
  /// The layer is named "Background" by default.
  pub fn new_from_path(
    p_name: impl Into<String>, p_path: impl Into<String>, _options: impl Into<Option<NewLayerOptions>>,
  ) -> Self {
    let name = p_name.into();
    let image = Image::read(p_path.into()).expect("Failed to load image");
    let (width, height) = image.dimensions::<u32>();

    let mut canvas_inner = CanvasInner::new(&name);
    let layer = canvas_inner.add_layer(LayerInner::new(&name, Arc::new(image)));
    layer.lock().unwrap().set_name("Background");
    canvas_inner.set_canvas_size(width, height);
    canvas_inner
  }

  /// Marks the canvas as needing recomposition.
  pub fn mark_dirty(&self) {
    self.needs_recompose.set(true);
  }

  /// Queues proportional scaling for direct layers, then recurses into child canvases.
  pub(crate) fn rescale_tree(
    &mut self, p_scale_x: Option<f32>, p_scale_y: Option<f32>, p_algorithm: Option<TransformAlgorithm>,
  ) {
    let resolution = self.resolution;
    for layer_rc in &self.layers {
      let mut layer = layer_rc.lock().unwrap();
      layer.prepare_for_composition(resolution);
      let (old_width, old_height) = layer.dimensions::<u32>();
      let (old_x, old_y) = layer.position();
      let new_width = p_scale_x.map(|scale| (old_width as f32 * scale).round().max(1.0) as u32).unwrap_or(old_width);
      let new_height = p_scale_y.map(|scale| (old_height as f32 * scale).round().max(1.0) as u32).unwrap_or(old_height);
      let new_x = p_scale_x.map(|scale| (old_x as f32 * scale).round() as i32).unwrap_or(old_x);
      let new_y = p_scale_y.map(|scale| (old_y as f32 * scale).round() as i32).unwrap_or(old_y);

      layer.queue_operation(LayerOperation::Resize(ResizeTarget::Exact(Size::new(new_width, new_height)), p_algorithm));
      layer.set_position_internal(new_x, new_y);
    }

    for child_rc in &self.canvases {
      let mut child = child_rc.lock().unwrap();
      let (old_x, old_y) = child.position();
      let old_width = child.width.get();
      let old_height = child.height.get();
      let new_width = p_scale_x.map(|scale| (old_width as f32 * scale).round().max(1.0) as u32);
      let new_height = p_scale_y.map(|scale| (old_height as f32 * scale).round().max(1.0) as u32);
      let new_x = p_scale_x.map(|scale| (old_x as f32 * scale).round() as i32).unwrap_or(old_x);
      let new_y = p_scale_y.map(|scale| (old_y as f32 * scale).round() as i32).unwrap_or(old_y);

      child.rescale_tree(p_scale_x, p_scale_y, p_algorithm);
      if let Some(new_width) = new_width {
        child.width.set(new_width);
      }
      if let Some(new_height) = new_height {
        child.height.set(new_height);
      }
      child.set_global_position(new_x, new_y);
      child.mark_dirty();
    }
  }

  /// Sets this document's explicit resolution and propagates inheritance to children.
  pub fn set_resolution(&mut self, p_resolution: Resolution) {
    self.resolution_is_explicit = true;
    self.apply_resolution(p_resolution);
  }

  /// Inherits a parent resolution unless this canvas explicitly overrides it.
  fn inherit_resolution(&mut self, p_resolution: Resolution) {
    if !self.resolution_is_explicit {
      self.apply_resolution(p_resolution);
    }
  }

  fn apply_resolution(&mut self, p_resolution: Resolution) {
    self.resolution = p_resolution;
    self.result.set_resolution(p_resolution);
    for layer in &self.layers {
      layer.lock().unwrap().set_resolution(p_resolution);
    }
    for child in &self.canvases {
      child.lock().unwrap().inherit_resolution(p_resolution);
    }
    self.mark_dirty();
  }

  /// Adds a new layer to the canvas.
  pub fn add_layer(&mut self, mut p_layer: LayerInner<'a>) -> Arc<Mutex<LayerInner<'a>>> {
    p_layer.set_resolution(self.resolution);
    let layer_rc = Arc::new(Mutex::new(p_layer));
    self.layers.push(layer_rc.clone());
    self.mark_dirty();
    layer_rc.clone()
  }

  /// Adds a new adjustment layer to the canvas.
  pub fn add_adjustment_layer(
    &mut self, p_name: impl Into<String>, p_layer_type: crate::AdjustmentLayerType,
  ) -> Arc<Mutex<LayerInner<'a>>> {
    let name = p_name.into();
    let adjustment_layer = LayerInner::new_adjustment_layer(name, p_layer_type);
    let layer_rc = Arc::new(Mutex::new(adjustment_layer));
    self.layers.push(layer_rc.clone());
    self.mark_dirty();
    layer_rc.clone()
  }

  /// Deletes a layer by its ID.
  pub fn delete_layer_by_id(&mut self, p_layer_id: &str) -> &mut Self {
    self.layers.retain(|layer_rc| layer_rc.lock().unwrap().id() != p_layer_id);
    self.mark_dirty();
    self
  }

  /// Adds an already-wrapped child canvas with the given options.
  pub fn add_canvas_rc(
    &mut self, p_child_inner: Arc<Mutex<CanvasInner<'a>>>, p_options: impl Into<Option<AddCanvasOptions>>,
  ) {
    let p_options = p_options.into();
    p_child_inner.lock().unwrap().inherit_resolution(self.resolution);
    // Set canvas size from first child canvas
    if self.width.get() == 0 && self.height.get() == 0 {
      let child = p_child_inner.lock().unwrap();
      let (width, height) = child.dimensions::<u32>();
      if width > 0 && height > 0 {
        self.set_canvas_size(width, height);
      }
    }

    // Now that parent size is set, calculate and apply anchor positioning
    {
      let parent_width = self.width.get() as i32;
      let parent_height = self.height.get() as i32;

      let child_dims = p_child_inner.lock().unwrap().dimensions::<i32>();
      let child_width = child_dims.0;
      let child_height = child_dims.1;

      let positions = p_options.as_ref().and_then(|o| o.position);
      let (x, y) = if let Some((x, y)) = positions {
        (x, y)
      } else {
        let anchor = p_options.as_ref().and_then(|o| o.anchor).unwrap_or(Anchor::Center);
        let (x, y) = anchor.calculate_position(parent_width, parent_height, child_width, child_height);
        (x, y)
      };

      let mut child = p_child_inner.lock().unwrap();
      child.set_global_position(x, y);
      if let Some(rotation) = p_options.as_ref().and_then(|o| o.rotation) {
        child.set_rotation(Some(rotation));
      }
    }

    self.canvases.push(p_child_inner);
    self.mark_dirty();
  }

  /// Updates the canvas image by merging all the layers and child canvases into one image.
  pub fn update_canvas(&mut self) {
    let width = self.width.get();
    let height = self.height.get();

    // Skip if canvas has zero dimensions
    if width == 0 || height == 0 {
      return;
    }

    let mut canvas = Image::new(width, height);

    // First pass: Apply anchors and recursively update child canvases
    for child_inner_rc in self.canvases.iter() {
      let mut child_inner = child_inner_rc.lock().unwrap();
      child_inner.apply_anchor_with_parent_dimensions(width as i32, height as i32);
      drop(child_inner);

      let mut child_inner_mut = child_inner_rc.lock().unwrap();
      child_inner_mut.update_canvas();
    }

    // Materialize text and transforms only after child attachment and resolution inheritance settle.
    for layer in &self.layers {
      layer.lock().unwrap().prepare_for_composition(self.resolution);
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

    final_image.set_resolution(self.resolution);
    self.result = final_image;
    // Mark canvas as recomposed
    self.needs_recompose.set(false);
  }

  /// Composite this canvas' layers and children into the destination image, honoring pass-through.
  /// `p_offset_x` and `p_offset_y` are positions applied to the layers (accumulated parent offsets).
  pub fn composite_into(&self, p_dest: &mut Image, p_offset_x: i32, p_offset_y: i32) {
    // Composite child canvases first (so child layers can be behind local layers)
    for child_inner_rc in self.canvases.iter() {
      let child_inner = child_inner_rc.lock().unwrap();
      let (child_width, child_height) = child_inner.dimensions::<u32>();
      if child_width == 0 || child_height == 0 {
        continue;
      }

      let (child_x, child_y) = child_inner.position();
      let dest_x = p_offset_x + child_x;
      let dest_y = p_offset_y + child_y;

      if child_inner.pass_through() && child_inner.rotation().is_none() {
        // Composite child's layers directly into `dest` with accumulated offsets
        let child_inner_ref = child_inner_rc.lock().unwrap();
        child_inner_ref.composite_into(p_dest, dest_x, dest_y);
      } else {
        // Composite child's flattened result
        let mut child_result = child_inner.get_result_image();
        if let Some(rotation_degrees) = child_inner.rotation() {
          child_result.rotate(rotation_degrees, None);
        }
        let child_blend = child_inner.blend_mode();
        let child_opacity = child_inner.opacity();
        blend_images(&child_result)
          .with_offset((dest_x, dest_y))
          .with_opacity(child_opacity)
          .with_mode(child_blend)
          .apply(p_dest);
      }
    }

    // Composite local layers into destination
    let canvas_dims = (self.width.get() as i32, self.height.get() as i32);
    let dest_has_content = !self.canvases.is_empty();
    let mut first_layer = true;
    for layer in self.layers.iter() {
      let mut layer_ref = layer.lock().unwrap();
      let rendered_image = layer_ref.apply_pending_effects();
      layer_ref.apply_anchor_with_canvas_dimensions(canvas_dims.0, canvas_dims.1);
      if layer_ref.is_visible() {
        let opacity = layer_ref.opacity().clamp(0.0, 1.0);
        let blend =
          if !dest_has_content && first_layer { abra_core::BlendMode::Normal } else { layer_ref.blend_mode() };
        let (x, y) = layer_ref.position();
        blend_images(&rendered_image)
          .with_offset((p_offset_x + x, p_offset_y + y))
          .with_opacity(opacity)
          .with_mode(blend)
          .apply(p_dest);
        first_layer = false;
      }
    }
  }

  /// Gets a clone of the result image.
  pub fn get_result_image(&self) -> Image {
    self.result.clone()
  }

  /// Resizes the canvas image to the given dimensions.
  pub fn set_canvas_size(&mut self, p_width: impl IntoNumber, p_height: impl IntoNumber) {
    let p_width: u32 = p_width.into();
    let p_height: u32 = p_height.into();
    self.result = Image::new(p_width, p_height);
    self.result.set_resolution(self.resolution);
    self.width.set(p_width);
    self.height.set(p_height);
  }

  /// Sets the position of the canvas to the given anchor point within its parent canvas.
  pub fn anchor_to_canvas(&mut self, p_anchor: Anchor) {
    self.anchor = Some(p_anchor);
  }

  /// Applies the stored anchor to position the canvas, given parent canvas dimensions
  pub fn apply_anchor_with_parent_dimensions(&mut self, p_parent_width: i32, p_parent_height: i32) {
    if let Some(anchor) = self.anchor {
      let (self_width, self_height) = self.dimensions::<i32>();
      let (x, y) = anchor.calculate_position(p_parent_width, p_parent_height, self_width, self_height);
      // Position the canvas directly at the calculated anchor position
      // The anchor calculation already handles proper centering/positioning
      self.x.set(x);
      self.y.set(y);
    }
  }

  /// Sets the global position of the canvas within its parent.
  pub fn set_global_position(&mut self, p_x: i32, p_y: i32) {
    self.x.set(p_x);
    self.y.set(p_y);
  }

  /// Gets the position of the canvas.
  pub fn position(&self) -> (i32, i32) {
    (self.x.get(), self.y.get())
  }

  /// Sets the rotation in degrees for the canvas within its parent.
  pub fn set_rotation(&mut self, p_degrees: impl Into<Option<f32>>) {
    self.rotation.set(p_degrees.into());
  }

  /// Gets the rotation in degrees for the canvas within its parent.
  pub fn rotation(&self) -> Option<f32> {
    self.rotation.get()
  }

  /// Sets the blend mode used when compositing this canvas into a parent.
  pub fn set_blend_mode(&mut self, p_blend: BlendMode) {
    self.blend_mode = p_blend;
    self.mark_dirty();
  }

  /// Gets the blend mode used for compositing this canvas.
  pub fn blend_mode(&self) -> BlendMode {
    self.blend_mode
  }

  /// Sets whether this canvas is pass-through.
  pub fn set_pass_through(&mut self, p_pass: bool) {
    self.pass_through = p_pass;
    self.mark_dirty();
  }

  /// Gets whether this canvas is a pass-through group.
  pub fn pass_through(&self) -> bool {
    self.pass_through
  }

  /// Sets the canvas opacity (0.0-1.0) when composited into the parent.
  pub fn set_opacity(&mut self, p_opacity: f32) {
    self.opacity.set(p_opacity.clamp(0.0, 1.0));
    self.mark_dirty();
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
  pub fn set_origin(&mut self, p_origin: Origin) {
    self.origin = p_origin;
  }

  /// Gets the origin point (anchor position within the canvas bounds).
  pub fn origin(&self) -> Origin {
    self.origin.clone()
  }

  /// Flattens all layers in the canvas into a single layer.
  /// All layers will be merged into one layer and removed.
  pub fn flatten(&mut self) {
    self.update_canvas();
    let flattened_image = self.result.clone();
    self.layers.clear();
    let mut flattened_layer = LayerInner::new("Flattened Layer", Arc::new(flattened_image));
    flattened_layer.set_visible(true);
    self.add_layer(flattened_layer);
  }

  /// Saves the project to the given path.
  pub fn save(&mut self, p_path: impl Into<String>, p_options: impl Into<Option<WriterOptions>>) {
    let start = std::time::Instant::now();
    if self.needs_recompose.get() {
      self.update_canvas();
    }
    println!("Canvas recomposed in {:?}", start.elapsed());
    writer(p_path.into()).with_options(p_options).save(&self.result).expect("Failed to save canvas");
  }

  pub fn set_effects(&mut self, p_effects: crate::LayerEffects<'a>) {
    self.effects = p_effects;
    self.mark_dirty();
  }

  /// Converts the entire canvas into a single Image by flattening all layers and child canvases.
  pub fn as_image(&mut self) -> Image {
    if self.needs_recompose.get() {
      self.update_canvas();
    }
    self.result.clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn explicit_child_resolution_overrides_parent_inheritance() {
    let mut root = CanvasInner::new_blank("root", 100, 100);
    let child = Arc::new(Mutex::new(CanvasInner::new_blank("child", 50, 50)));
    child.lock().unwrap().set_resolution(Resolution::SCREEN);

    root.add_canvas_rc(child.clone(), None);
    root.set_resolution(Resolution::ART);

    assert_eq!(child.lock().unwrap().resolution, Resolution::SCREEN);
  }
}
