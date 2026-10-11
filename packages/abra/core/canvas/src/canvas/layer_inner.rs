//! The internal layer implementation.

use abra_core::BlendMode;
use abra_core::image::image_ext::ImageExt;
use abra_core::{FlipAxis, Image, ResizeTarget, Resolution, Transform, TransformAlgorithm};
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::Mutex;

use abra_core::Channels;
use abra_core::image::gpu::{ChainRenderer, GpuSession, Hardware, LiveEffect, ready_gpu_provider};

use crate::Origin;
use crate::canvas::canvas_inner::CanvasInner;
use crate::effects::LayerEffects;
use crate::{Anchor, LayerMove};
use typography::Text;

#[derive(Clone, Copy)]
pub(crate) enum LayerOperation {
  Resize(ResizeTarget, Option<TransformAlgorithm>),
  Crop(u32, u32, u32, u32),
  Rotate(f64, Option<TransformAlgorithm>),
  Flip(FlipAxis),
}

/// Live, re-renderable effects on a layer. The source image is never modified: the effects are evaluated over it
/// whenever the layer is composed, on the GPU when available. The GPU session holds the source uploaded once.
#[derive(Default)]
struct LiveState {
  effects: Vec<Arc<dyn LiveEffect>>,
  session: Option<ChainRenderer<Box<dyn GpuSession>>>,
  rendered: Option<Arc<Image>>,
}

impl Clone for LiveState {
  fn clone(&self) -> Self {
    LiveState {
      effects: self.effects.clone(),
      session: None,
      rendered: None,
    }
  }
}

impl LiveState {
  /// The source pixels changed: the uploaded copy and the rendered result are both stale.
  fn invalidate_source(&mut self) {
    self.session = None;
    self.rendered = None;
  }
}

/// The internal layer implementation - provides the mutable reference API.
pub struct LayerInner<'a> {
  /// The name of the layer.
  name: String,
  /// The image data of the layer.
  image: Arc<Image>,
  /// Source text retained so document-resolution changes can re-rasterize it.
  text_source: Option<Text>,
  /// Whether the retained text source needs to be rasterized before composition.
  text_needs_rasterization: bool,
  /// Pixel operations recorded by the fluent layer transform API.
  operations: Vec<LayerOperation>,
  /// Number of queued operations already materialized into `image`.
  applied_operations: usize,
  /// Whether the layer is visible.
  visible: bool,
  /// The opacity of the layer.
  opacity: f32,
  /// The blend mode of the layer.
  blend_mode: BlendMode,
  /// The x position of the image within the layer.
  x: i32,
  /// The y position of the image within the layer.
  y: i32,
  /// A UUID for the layer.
  id: String,
  /// Reference to the canvas.
  canvas: Arc<Mutex<CanvasInner<'a>>>,
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
  effects: LayerEffects<'a>,
  /// The type of adjustment layer, if this is an adjustment layer.
  adjustment_layer_type: Option<crate::AdjustmentLayerType>,
  /// Effects evaluated over the source image at composition time.
  live: LiveState,
}

impl Debug for LayerInner<'_> {
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    p_f
      .debug_struct("LayerInner")
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

impl Default for LayerInner<'_> {
  fn default() -> Self {
    LayerInner {
      id: uuid::Uuid::new_v4().to_string(),
      name: "Layer".to_string(),
      image: Arc::new(Image::new(1, 1)),
      text_source: None,
      text_needs_rasterization: false,
      operations: Vec::new(),
      applied_operations: 0,
      visible: true,
      opacity: 1.0,
      blend_mode: BlendMode::Normal,
      x: 0,
      y: 0,
      canvas: Arc::new(Mutex::new(CanvasInner::new("Temporary"))),
      anchor: None,
      origin: Origin::default(),
      anchor_dimensions: None,
      anchor_offset: (0, 0),
      effects: LayerEffects::new(),
      adjustment_layer_type: None,
      live: LiveState::default(),
    }
  }
}

impl<'a> LayerInner<'a> {
  /// Creates a new layer with the given name, image, and canvas
  pub fn new(p_name: impl Into<String>, p_image: Arc<Image>) -> LayerInner<'a> {
    LayerInner {
      name: p_name.into(),
      image: p_image,
      ..Default::default()
    }
  }

  /// Creates a layer backed by text that can be rasterized again at a new DPI.
  pub(crate) fn new_text(p_name: impl Into<String>, p_text: Text, p_resolution: Resolution) -> LayerInner<'a> {
    let mut image = Image::new(1, 1);
    image.set_resolution(p_resolution);
    LayerInner {
      name: p_name.into(),
      image: Arc::new(image),
      text_source: Some(p_text),
      text_needs_rasterization: true,
      ..Default::default()
    }
  }

  pub fn new_adjustment_layer(p_name: impl Into<String>, p_layer_type: crate::AdjustmentLayerType) -> LayerInner<'a> {
    let image = Arc::new(Image::new_from_color(1, 1, abra_core::Color::transparent()));
    LayerInner {
      name: p_name.into(),
      image,
      adjustment_layer_type: Some(p_layer_type),
      ..Default::default()
    }
  }

  /// Sets the canvas reference for the layer.
  pub(crate) fn set_canvas(&mut self, p_canvas: Arc<Mutex<CanvasInner<'a>>>) {
    self.canvas = p_canvas.clone();
  }

  /// Gets the UUID of the layer.
  pub fn id(&self) -> &str {
    &self.id
  }

  /// Marks the layer's canvas as needing to recompose.
  pub fn mark_dirty(&mut self) {
    self.canvas.lock().unwrap().mark_dirty();
  }

  /// Gets the anchor point of the layer.
  pub fn anchor(&self) -> Option<Anchor> {
    self.anchor
  }

  /// Sets the blend mode of the layer.
  pub fn set_blend_mode(&mut self, p_blend_mode: BlendMode) {
    self.blend_mode = p_blend_mode;
    self.mark_dirty();
  }

  /// Sets the opacity of the layer.
  pub fn set_opacity(&mut self, p_opacity: f32) {
    self.opacity = p_opacity;
    self.mark_dirty();
  }

  /// Sets the visibility of the layer.
  pub fn set_visible(&mut self, p_visible: bool) {
    self.visible = p_visible;
    self.mark_dirty();
  }

  /// Sets the position of the layer.
  pub fn set_global_position(&mut self, p_x: i32, p_y: i32) {
    self.x = p_x;
    self.y = p_y;
    self.mark_dirty();
  }

  /// Sets the anchor point for the layer.
  pub fn set_anchor(&mut self, p_anchor: Option<Anchor>) {
    self.anchor = p_anchor;
    self.mark_dirty();
  }

  /// Sets the position of the layer relative to another layer
  pub fn set_relative_position(&mut self, p_x: i32, p_y: i32, p_layer: &LayerInner) {
    self.x = p_layer.x + p_x;
    self.y = p_layer.y + p_y;
    self.mark_dirty();
  }

  /// Sets the effects configuration for this layer and marks canvas dirty.
  pub fn set_effects(&mut self, p_effects: LayerEffects<'a>) {
    self.effects = p_effects;
    self.mark_dirty();
  }

  /// Sets the position of the layer to the given anchor point
  /// The anchor is stored and will be applied during render time (update_canvas)
  pub fn anchor_to_canvas(&mut self, p_anchor: Anchor) {
    self.anchor = Some(p_anchor);
  }

  /// Checks if the layer has an anchor set.
  pub fn has_anchor(&self) -> bool {
    self.anchor.is_some()
  }

  /// Sets the origin point within the layer for anchor positioning.
  pub fn set_origin(&mut self, p_origin: Origin) {
    self.origin = p_origin;
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
  pub fn set_anchor_dimensions(&mut self, p_width: u32, p_height: u32) {
    self.anchor_dimensions = Some((p_width, p_height));
  }

  /// Clears the anchor dimensions, reverting to using image dimensions for anchoring.
  pub fn clear_anchor_dimensions(&mut self) {
    self.anchor_dimensions = None;
  }

  /// Applies the stored anchor to position the layer, given canvas dimensions
  /// This version directly updates x and y to avoid nested borrows of the canvas
  pub fn apply_anchor_with_canvas_dimensions(&mut self, p_canvas_width: i32, p_canvas_height: i32) {
    if let Some(anchor) = self.anchor {
      let (self_width, self_height) = self.anchor_dimensions();
      let (x, y) = anchor.calculate_position(p_canvas_width, p_canvas_height, self_width as i32, self_height as i32);
      // Position the layer directly at the calculated anchor position
      // The anchor calculation already handles proper centering/positioning
      self.x = x + self.anchor_offset.0;
      self.y = y + self.anchor_offset.1;
    }
  }

  /// Gets the dimensions of the layer
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u64>,
    <T as TryFrom<u64>>::Error: std::fmt::Debug,
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
  pub fn set_name(&mut self, p_name: impl Into<String>) {
    self.name = p_name.into();
  }

  /// Gets the opacity of the layer
  pub fn opacity(&self) -> f32 {
    self.opacity
  }

  /// Gets the blend mode of the layer
  pub fn blend_mode(&self) -> BlendMode {
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
    self.text_source = None;
    self.text_needs_rasterization = false;
    self.operations.clear();
    self.applied_operations = 0;
    self.live.invalidate_source();
    Arc::make_mut(&mut self.image)
  }

  /// Records a transform to be materialized during canvas composition.
  pub(crate) fn queue_operation(&mut self, p_operation: LayerOperation) {
    self.operations.push(p_operation);
  }

  /// Marks retained text for re-rasterization at the supplied document resolution.
  pub fn set_resolution(&mut self, p_resolution: Resolution) {
    if self.text_source.is_some() {
      self.text_needs_rasterization = true;
      self.applied_operations = 0;
    }
    self.live.invalidate_source();
    Arc::make_mut(&mut self.image).set_resolution(p_resolution);
  }

  /// Resolves pending text and pixel operations only when the canvas is ready to compose.
  pub fn prepare_for_composition(&mut self, p_resolution: Resolution) {
    if self.text_needs_rasterization {
      if let Some(text) = self.text_source.clone() {
        let mut image = Image::from(text.with_dpi(p_resolution.y_dpi));
        image.set_resolution(p_resolution);
        self.image = Arc::new(image);
        self.live.invalidate_source();
        self.text_needs_rasterization = false;
        self.applied_operations = 0;
      }
    }

    if self.applied_operations == self.operations.len() {
      return;
    }
    self.live.invalidate_source();
    let image = Arc::make_mut(&mut self.image);
    for operation in self.operations[self.applied_operations..].iter().copied() {
      match operation {
        LayerOperation::Resize(target, algorithm) => image.resize(target, algorithm),
        LayerOperation::Crop(x, y, width, height) => image.crop(x, y, width, height),
        LayerOperation::Rotate(angle, algorithm) => image.rotate(angle, algorithm),
        LayerOperation::Flip(axis) => image.flip(axis),
      }
    }
    self.applied_operations = self.operations.len();
  }

  /// Renders the layer effects without replacing the layer's source image.
  pub fn apply_pending_effects(&mut self) -> Arc<Image> {
    let image_arc = self.render_live();
    let result = self.effects.apply_with_offset(image_arc);
    let (orig_w, orig_h) = result.content_dimensions;
    self.set_anchor_dimensions(orig_w, orig_h);
    let (pad_left, pad_top) = result.offset;
    self.set_anchor_offset(-pad_left, -pad_top);
    result.image
  }

  /// Replaces the live effects evaluated over this layer's image and marks the canvas dirty. Only the effects are
  /// re-run on the next composition; the source is not re-uploaded or modified.
  pub fn set_live_effects(&mut self, p_effects: Vec<Arc<dyn LiveEffect>>) {
    self.live.effects = p_effects;
    self.live.rendered = None;
    self.mark_dirty();
  }

  /// The live effects on this layer.
  pub fn live_effects(&self) -> &[Arc<dyn LiveEffect>] {
    &self.live.effects
  }

  /// The source image with the live effects applied, cached until the effects or the source change.
  fn render_live(&mut self) -> Arc<Image> {
    if self.live.effects.is_empty() {
      return self.image.clone();
    }
    if let Some(rendered) = &self.live.rendered {
      return rendered.clone();
    }
    let rendered = Arc::new(self.render_live_uncached());
    self.live.rendered = Some(rendered.clone());
    rendered
  }

  fn render_live_uncached(&mut self) -> Image {
    if let Some(provider) = ready_gpu_provider(Hardware::Auto) {
      let (width, height) = self.image.dimensions::<u32>();
      if self.live.session.is_none() {
        self.live.session = (provider.new_session)()
          .and_then(|session| ChainRenderer::new(session, width, height, &self.image.to_rgba_vec()))
          .ok();
      }
      if let Some(session) = self.live.session.as_mut() {
        let effects: Vec<&dyn LiveEffect> = self.live.effects.iter().map(|e| e.as_ref()).collect();
        match session.render_blocking(&effects) {
          Ok(pixels) => {
            let mut image = Image::new_from_pixels(width, height, pixels, Channels::RGBA);
            image.set_resolution(self.image.resolution());
            return image;
          }
          Err(_) => self.live.session = None,
        }
      }
    }
    let mut image = (*self.image).clone();
    for effect in &self.live.effects {
      effect.apply_cpu(&mut image);
    }
    image
  }

  /// Sets the index of the layer within the canvas's layer stack
  pub fn set_index(&mut self, p_index: usize) {
    // To avoid borrow conflicts, we need to find the current layer's index by ID
    let current_index = self.current_index();

    if let Some(current_idx) = current_index {
      let mut canvas = self.canvas.lock().unwrap();
      // Directly manipulate layers vec
      if current_idx != p_index && p_index <= canvas.layers.len() {
        let layer = canvas.layers.remove(current_idx);
        canvas.layers.insert(p_index, layer);
      }
      // Mark canvas as needing recomposition since layer order changed
      canvas.mark_dirty();
    }
  }

  /// Gets the current index of this layer in the canvas's layer stack
  pub fn current_index(&self) -> Option<usize> {
    let canvas = self.canvas.lock().unwrap();
    canvas.layers.iter().enumerate().find_map(|(i, layer_rc)| {
      if let Ok(layer) = layer_rc.try_lock() {
        if layer.id == self.id { Some(i) } else { None }
      } else {
        // If can't lock, assume it's self
        Some(i)
      }
    })
  }

  pub fn adjustment_type(&self) -> Option<crate::AdjustmentLayerType> {
    self.adjustment_layer_type.clone()
  }

  /// Moves this layer according to a relative or absolute stack action.
  pub fn move_to(&mut self, p_action: LayerMove) {
    let Some(current_index) = self.current_index() else { return };
    let target_index = match p_action {
      LayerMove::Up => {
        let len = self.canvas.lock().unwrap().layers.len();
        (current_index + 1 < len).then_some(current_index + 1)
      }
      LayerMove::Down => current_index.checked_sub(1),
      LayerMove::Top => {
        let top = self.canvas.lock().unwrap().layers.len().saturating_sub(1);
        (current_index != top).then_some(top)
      }
      LayerMove::Bottom => (current_index != 0).then_some(0),
    };

    if let Some(target_index) = target_index {
      self.set_index(target_index);
    }
  }

  /// Sets the position of the layer without triggering a recompose.
  /// This is used internally when resizing/cropping the canvas.
  pub fn set_position_internal(&mut self, p_x: i32, p_y: i32) {
    self.x = p_x;
    self.y = p_y;
  }

  /// Duplicates the layer within the same canvas.
  /// This returns a Layer (the public wrapper), not the raw Rc<Mutex<LayerInner>>.
  pub fn duplicate(&self) -> super::Layer<'a> {
    let mut new_layer = self.clone();
    new_layer.set_name(format!("{} clone", new_layer.name()).as_str());

    let layer_rc = {
      let mut canvas = self.canvas.lock().unwrap();
      canvas.add_layer(new_layer).clone()
    };

    super::Layer::from_inner(layer_rc)
  }

  pub fn save(&self, p_file: impl Into<String>, p_options: impl Into<Option<abra_core::WriterOptions>>) {
    self.image.write(p_file.into(), p_options).expect("Failed to save layer");
  }

  pub fn as_image(&self) -> Image {
    (*self.image).clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Canvas;
  use crate::effects::DropShadow;
  use abra_core::{Image, Transform};
  use std::sync::Arc;

  #[test]
  fn apply_pending_sets_anchor_offset_for_drop_shadow() {
    let img = Arc::new(Image::new(1, 1));
    let mut layer = LayerInner::new("test", img.clone());
    let shadow = DropShadow::new().with_distance(2.0).with_size(3.0);
    layer.set_effects(LayerEffects::new().with_drop_shadow(shadow.clone()));

    // Compute expected padding following the same logic as apply_drop_shadow_with_offset
    let angle_rad = shadow.angle.to_radians();
    let offset_x = (shadow.distance * angle_rad.cos()).round() as i32;
    let offset_y = (shadow.distance * angle_rad.sin()).round() as i32;
    let blur_padding = shadow.size as i32;
    let pad_left = (-offset_x).max(0) + blur_padding;
    let pad_top = (-offset_y).max(0) + blur_padding;
    layer.apply_pending_effects();
    assert_eq!(layer.anchor_dimensions(), (1, 1));
    assert_eq!(layer.anchor_offset, (-pad_left, -pad_top));
  }

  #[test]
  fn duplicate_returns_a_new_layer() {
    let canvas = Canvas::new("test");
    let layer = canvas.add_layer_from_image("source", Arc::new(Image::new(1, 1)), None);

    let duplicate = layer.duplicate();

    assert_eq!(duplicate.name(), "source clone");
    assert_ne!(duplicate.id(), layer.id());
    assert_eq!(canvas.layer_count(), 2);
  }

  #[test]
  fn repeated_effect_rendering_keeps_the_source_image_unchanged() {
    let mut layer = LayerInner::new("test", Arc::new(Image::new(1, 1)));
    layer.set_effects(LayerEffects::new().with_drop_shadow(DropShadow::new().with_size(3.0)));

    let first_render = layer.apply_pending_effects();
    let second_render = layer.apply_pending_effects();

    assert_eq!(layer.dimensions::<u32>(), (1, 1));
    assert_eq!(first_render.dimensions::<u32>(), second_render.dimensions::<u32>());
  }

  #[test]
  fn layer_transforms_materialize_during_canvas_composition() {
    let canvas = Canvas::new_blank("test", 40, 40);
    let layer = canvas.add_layer_from_image("source", Image::new(10, 10), None);

    layer.transform().resize(ResizeTarget::Exact(abra_core::Size::new(20, 20)), None);
    assert_eq!(layer.dimensions(), (10, 10));

    canvas.as_image();
    assert_eq!(layer.dimensions(), (20, 20));
  }
}

impl<'a> Clone for LayerInner<'a> {
  fn clone(&self) -> Self {
    LayerInner {
      id: uuid::Uuid::new_v4().to_string(),
      name: self.name.clone(),
      image: self.image.clone(),
      text_source: self.text_source.clone(),
      text_needs_rasterization: self.text_needs_rasterization,
      operations: self.operations.clone(),
      applied_operations: self.applied_operations,
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
      adjustment_layer_type: self.adjustment_layer_type.clone(),
      live: self.live.clone(),
    }
  }
}
