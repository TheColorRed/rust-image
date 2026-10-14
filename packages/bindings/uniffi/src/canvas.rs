//! `Canvas` and `Layer` from the `canvas` crate.
//!
//! A canvas is a group of layers and child canvases that can be manipulated together, positioned with anchors,
//! transformed, and exported to an image.
//!
//! What is not exported, and why:
//! - `with_image_mut` takes a Rust closure which cannot cross FFI; callers use `layer.image()` / `layer.set_image()`
//!   or the transform and effect methods.
//! - Non-solid fills in `DropShadow` and `Stroke` use `with_color(color)` for cross-language convenience.

use std::sync::Arc;

use abra::abra_core::{FlipAxis, Resolution, ResizeTarget, Size, Transform, TransformAlgorithm, WriterOptions};
use abra::canvas::prelude::{
  self as library, AdjustmentLayerType, Anchor, CanvasResizeTarget, CanvasUnit, LayerMove, LayerSize, Origin,
  OutlinePosition,
};

use crate::{BlendMode, Color, Image};

// The anchor point for positioning within a parent container.
#[uniffi::remote(Enum)]
pub enum Anchor {
  TopLeft,
  TopCenter,
  TopRight,
  CenterLeft,
  Center,
  CenterRight,
  BottomLeft,
  BottomCenter,
  BottomRight,
}

// Origin point within a layer defining which point aligns with the anchor.
#[uniffi::remote(Enum)]
pub enum Origin {
  TopLeft,
  TopCenter,
  TopRight,
  CenterLeft,
  Center,
  CenterRight,
  BottomLeft,
  BottomCenter,
  BottomRight,
  Custom(f32, f32),
}

// How an image is resized when added as a layer.
#[uniffi::remote(Enum)]
pub enum LayerSize {
  Maintain,
  Contain(Option<TransformAlgorithm>),
  Cover(Option<TransformAlgorithm>),
  Specific(u32, u32, Option<TransformAlgorithm>),
  Percentage(f32, Option<TransformAlgorithm>),
}

// Target specifications for resizing a canvas.
#[uniffi::remote(Enum)]
pub enum CanvasResizeTarget {
  Exact(Size),
  StretchWidth(u32),
  StretchHeight(u32),
  Scale(f32),
  RelativeWidth(i32),
  RelativeHeight(i32),
}

// Target specifications for resizing a layer or image.
#[uniffi::remote(Enum)]
pub enum ResizeTarget {
  Exact(Size),
  FitWidth(u32),
  FitHeight(u32),
  Fit(Size),
  RelativeWidth(i32),
  RelativeHeight(i32),
  Scale(f32),
}

// Relative or absolute layer stack movement.
#[uniffi::remote(Enum)]
pub enum LayerMove {
  Up,
  Down,
  Top,
  Bottom,
}

// Types of adjustment layers.
#[uniffi::remote(Enum)]
pub enum AdjustmentLayerType {
  BrightnessContrast,
  Exposure,
  HueSaturation,
  ColorBalance,
  BlackWhite,
  PhotoFilter,
  ChannelMixer,
  Invert,
  Posterize,
  GradientMap,
  SolidColor,
  Gradient,
  Pattern,
}

// Physical, relative, or grid unit specifications for creating a canvas.
#[uniffi::remote(Enum)]
pub enum CanvasUnit {
  Pixels(u32, u32, Resolution),
  Inches(u32, u32, Resolution),
  Centimeters(u32, u32, Resolution),
  Millimeters(u32, u32, Resolution),
  Points(u32, u32, Resolution),
  Picas(u32, u32, Resolution),
  Percent(u32, u32, u32, u32, Resolution),
  Columns(u32, u32, u32, u32, Resolution),
}

// Position of outline stroke relative to shape edge.
#[uniffi::remote(Enum)]
pub enum OutlinePosition {
  Inside,
  Outside,
  Center,
}

/// Options for configuring a new layer added to a canvas.
#[derive(Clone, Debug, Default, PartialEq, uniffi::Record)]
pub struct NewLayerOptions {
  pub anchor: Option<Anchor>,
  pub opacity: Option<f32>,
  pub blend_mode: Option<BlendMode>,
  pub size: Option<LayerSize>,
}

impl NewLayerOptions {
  pub fn to_core(self) -> library::NewLayerOptions {
    library::NewLayerOptions {
      anchor: self.anchor,
      opacity: self.opacity,
      blend_mode: self.blend_mode.map(|m| m.to_core()),
      size: self.size,
    }
  }

  #[allow(dead_code)]
  pub fn from_core(options: library::NewLayerOptions) -> Self {
    Self {
      anchor: options.anchor,
      opacity: options.opacity,
      blend_mode: options.blend_mode.map(BlendMode::from),
      size: options.size,
    }
  }
}

/// Creates a `NewLayerOptions` record with default settings.
#[uniffi::export]
pub fn new_layer_options() -> NewLayerOptions {
  NewLayerOptions::default()
}

/// Options for adding a child canvas to a parent canvas.
#[derive(Clone, Debug, Default, PartialEq, uniffi::Record)]
pub struct AddCanvasOptions {
  pub anchor: Option<Anchor>,
  pub position: Option<Vec<i32>>,
  pub rotation: Option<f32>,
}

impl AddCanvasOptions {
  pub fn to_core(self) -> library::AddCanvasOptions {
    library::AddCanvasOptions {
      anchor: self.anchor,
      position: self.position.map(|p| {
        let [x, y]: [i32; 2] = p.try_into().expect("a position takes 2 values: [x, y]");
        (x, y)
      }),
      rotation: self.rotation,
    }
  }

  #[allow(dead_code)]
  pub fn from_core(options: library::AddCanvasOptions) -> Self {
    Self {
      anchor: options.anchor,
      position: options.position.map(|(x, y)| vec![x, y]),
      rotation: options.rotation,
    }
  }
}

/// Creates an `AddCanvasOptions` record with default settings.
#[uniffi::export]
pub fn add_canvas_options() -> AddCanvasOptions {
  AddCanvasOptions::default()
}

/// Configuration for drop shadow effect on a layer or canvas.
#[derive(Clone, Debug, uniffi::Object)]
pub struct DropShadow(library::DropShadow<'static>);

#[uniffi::export]
impl DropShadow {
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self(library::DropShadow::new())
  }

  /// Sets the drop shadow color.
  pub fn with_color(&self, color: Arc<Color>) -> Arc<DropShadow> {
    let mut shadow = self.0.clone();
    shadow = shadow.with_fill(abra::abra_core::Fill::Solid(color.0));
    Arc::new(DropShadow(shadow))
  }

  /// Sets the drop shadow opacity (0.0 to 1.0).
  pub fn with_opacity(&self, opacity: f32) -> Arc<DropShadow> {
    Arc::new(DropShadow(self.0.clone().with_opacity(opacity)))
  }

  /// Sets the angle of the shadow in degrees.
  pub fn with_angle(&self, angle: f32) -> Arc<DropShadow> {
    Arc::new(DropShadow(self.0.clone().with_angle(angle)))
  }

  /// Sets the distance of the shadow from the object.
  pub fn with_distance(&self, distance: f32) -> Arc<DropShadow> {
    Arc::new(DropShadow(self.0.clone().with_distance(distance)))
  }

  /// Sets the spread of the shadow between 0.0 and 1.0.
  pub fn with_spread(&self, spread: f32) -> Arc<DropShadow> {
    Arc::new(DropShadow(self.0.clone().with_spread(spread)))
  }

  /// Sets the blur radius of the shadow.
  pub fn with_size(&self, size: f32) -> Arc<DropShadow> {
    Arc::new(DropShadow(self.0.clone().with_size(size)))
  }

  /// Sets the blend mode used to combine the shadow with the background.
  pub fn with_blend_mode(&self, blend_mode: BlendMode) -> Arc<DropShadow> {
    let mut shadow = self.0.clone();
    shadow.blend_mode = blend_mode.to_core();
    Arc::new(DropShadow(shadow))
  }
}

/// Configuration for stroke outline effect on a layer or canvas.
#[derive(Clone, Debug, uniffi::Object)]
pub struct Stroke(library::Stroke<'static>);

#[uniffi::export]
impl Stroke {
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self(library::Stroke::new())
  }

  /// Sets the thickness of the outline in pixels.
  pub fn with_size(&self, size: u32) -> Arc<Stroke> {
    Arc::new(Stroke(self.0.clone().with_size(size)))
  }

  /// Sets the color of the outline.
  pub fn with_color(&self, color: Arc<Color>) -> Arc<Stroke> {
    Arc::new(Stroke(self.0.clone().with_fill(abra::abra_core::Fill::Solid(color.0))))
  }

  /// Sets the opacity of the outline (0.0 to 1.0).
  pub fn with_opacity(&self, opacity: f32) -> Arc<Stroke> {
    Arc::new(Stroke(self.0.clone().with_opacity(opacity)))
  }

  /// Sets the position of the outline relative to the shape: Inside, Outside, or Center.
  pub fn with_position(&self, position: OutlinePosition) -> Arc<Stroke> {
    Arc::new(Stroke(self.0.clone().with_position(position)))
  }
}

/// Composite effects collection for a layer or canvas.
#[derive(Clone, Debug, uniffi::Object)]
pub struct LayerEffects(library::LayerEffects<'static>);

#[uniffi::export]
impl LayerEffects {
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self(library::LayerEffects::new())
  }

  /// Sets the drop shadow effect.
  pub fn with_drop_shadow(&self, drop_shadow: Arc<DropShadow>) -> Arc<LayerEffects> {
    let mut effects = self.0.clone();
    effects.drop_shadow = Some(drop_shadow.0.clone());
    Arc::new(LayerEffects(effects))
  }

  /// Sets the stroke outline effect.
  pub fn with_stroke(&self, stroke: Arc<Stroke>) -> Arc<LayerEffects> {
    let mut effects = self.0.clone();
    effects.stroke = Some(stroke.0.clone());
    Arc::new(LayerEffects(effects))
  }
}

/// A canvas is a composition of image layers and child canvases.
#[derive(uniffi::Object)]
pub struct Canvas(library::Canvas<'static>);

#[uniffi::export]
impl Canvas {
  /// Creates an empty canvas with a given name (0x0 initial dimensions).
  #[uniffi::constructor]
  pub fn new(name: String) -> Self {
    Self(library::Canvas::new(name))
  }

  /// Creates a blank canvas with the given name and dimensions.
  #[uniffi::constructor]
  pub fn new_blank(name: String, width: u32, height: u32) -> Self {
    Self(library::Canvas::new_blank(name, width, height))
  }

  /// Creates a canvas initialized from an image file path.
  #[uniffi::constructor]
  pub fn new_from_path(name: String, path: String, options: Option<NewLayerOptions>) -> Self {
    Self(library::Canvas::new_from_path(name, path, options.map(|o| o.to_core())))
  }

  /// Creates a canvas from a complete physical, relative, or grid specification.
  #[uniffi::constructor]
  pub fn new_from_unit(name: String, unit: CanvasUnit) -> Self {
    Self(library::Canvas::new_from_unit(name, unit))
  }

  /// The unique ID of the canvas.
  pub fn id(&self) -> String {
    self.0.id()
  }

  /// The name of the canvas.
  pub fn name(&self) -> String {
    self.0.name()
  }

  /// The width in pixels.
  pub fn width(&self) -> u32 {
    self.0.dimensions::<u32>().0
  }

  /// The height in pixels.
  pub fn height(&self) -> u32 {
    self.0.dimensions::<u32>().1
  }

  /// The dimensions as a `Size`.
  pub fn size(&self) -> Size {
    let (width, height) = self.0.dimensions::<u32>();
    Size {
      width: width as f32,
      height: height as f32,
    }
  }

  /// The dimensions as `[width, height]`.
  pub fn dimensions(&self) -> Vec<u32> {
    let (width, height) = self.0.dimensions::<u32>();
    vec![width, height]
  }

  /// Document resolution (DPI) metadata.
  pub fn resolution(&self) -> Resolution {
    self.0.resolution()
  }

  /// Sets document resolution metadata without resizing pixels.
  pub fn set_resolution(&self, resolution: Resolution) {
    self.0.set_resolution(resolution);
  }

  /// Applies target `CanvasUnit` resolution metadata, then resamples all layers.
  pub fn resample(&self, unit: CanvasUnit) {
    self.0.resample(unit);
  }

  /// The position within a parent canvas as `[x, y]`.
  pub fn position(&self) -> Vec<i32> {
    let (x, y) = self.0.position();
    vec![x, y]
  }

  /// Sets the position within a parent canvas.
  pub fn set_position(&self, x: i32, y: i32) {
    self.0.set_position(x, y);
  }

  /// The rotation in degrees within a parent canvas.
  pub fn rotation(&self) -> Option<f32> {
    self.0.rotation()
  }

  /// Sets the rotation in degrees within a parent canvas.
  pub fn set_rotation(&self, degrees: Option<f32>) {
    self.0.set_rotation(degrees);
  }

  /// The origin point within the canvas bounds.
  pub fn origin(&self) -> Origin {
    self.0.origin()
  }

  /// Sets the origin point within the canvas bounds.
  pub fn set_origin(&self, origin: Origin) {
    self.0.set_origin(origin);
  }

  /// The blend mode used when composited into a parent canvas.
  pub fn blend_mode(&self) -> BlendMode {
    BlendMode::from(self.0.blend_mode())
  }

  /// Sets the blend mode used when composited into a parent canvas.
  pub fn set_blend_mode(&self, blend_mode: BlendMode) {
    self.0.set_blend_mode(blend_mode.to_core());
  }

  /// Whether child elements pass through to the parent canvas.
  pub fn pass_through(&self) -> bool {
    self.0.pass_through()
  }

  /// Sets whether child elements pass through to the parent canvas.
  pub fn set_pass_through(&self, pass: bool) {
    self.0.set_pass_through(pass);
  }

  /// The opacity of the canvas (0.0 to 1.0).
  pub fn opacity(&self) -> f32 {
    self.0.opacity()
  }

  /// Sets the opacity of the canvas (0.0 to 1.0).
  pub fn set_opacity(&self, opacity: f32) {
    self.0.set_opacity(opacity);
  }

  /// Limits where the canvas shows; the mask must be the size of the canvas. `None` removes it.
  pub fn set_mask(&self, mask: Option<Arc<crate::Mask>>) {
    self.0.set_mask(mask.map(|m| m.core()));
  }

  /// Whether the canvas has uncomposed changes.
  pub fn is_dirty(&self) -> bool {
    self.0.is_dirty()
  }

  /// Sets the position of this canvas to the given anchor point within its parent.
  pub fn anchor_to_canvas(&self, anchor: Anchor) {
    self.0.anchor_to_canvas(anchor);
  }

  /// Adds a new layer from an image file path.
  pub fn add_layer_from_path(&self, name: String, path: String, options: Option<NewLayerOptions>) -> Arc<Layer> {
    let layer = self.0.add_layer_from_path(name, path, options.map(|o| o.to_core()));
    Arc::new(Layer(layer))
  }

  /// Adds a new layer from an existing Image.
  pub fn add_layer_from_image(&self, name: String, image: Arc<Image>, options: Option<NewLayerOptions>) -> Arc<Layer> {
    let layer = self.0.add_layer_from_image(name, image.core(), options.map(|o| o.to_core()));
    Arc::new(Layer(layer))
  }

  /// Adds an adjustment layer.
  pub fn add_adjustment_layer(&self, name: String, adjustment_type: AdjustmentLayerType) -> Arc<Layer> {
    let layer = self.0.add_adjustment_layer(name, adjustment_type);
    Arc::new(Layer(layer))
  }

  /// Adds a child canvas with optional placement options.
  pub fn add_canvas(&self, canvas: Arc<Canvas>, options: Option<AddCanvasOptions>) -> Arc<Canvas> {
    let child = self.0.add_canvas(canvas.0.clone(), options.map(|o| o.to_core()));
    Arc::new(Canvas(child))
  }

  /// Deletes a layer by its UUID.
  pub fn delete_layer_by_id(&self, layer_id: String) {
    self.0.delete_layer_by_id(&layer_id);
  }

  /// Gets a layer by 0-based index in the stack.
  pub fn get_layer_by_index(&self, index: u32) -> Option<Arc<Layer>> {
    self.0.get_layer_by_index(index as usize).map(|l| Arc::new(Layer(l)))
  }

  /// Gets a layer by its UUID.
  pub fn get_layer_by_id(&self, id: String) -> Option<Arc<Layer>> {
    self.0.get_layer_by_id(&id).map(|l| Arc::new(Layer(l)))
  }

  /// Gets the first layer matching the given name.
  pub fn get_layer_by_name(&self, name: String) -> Option<Arc<Layer>> {
    self.0.get_layer_by_name(name).map(|l| Arc::new(Layer(l)))
  }

  /// Gets all layers currently in the canvas.
  pub fn layers(&self) -> Vec<Arc<Layer>> {
    self.0.layers().into_iter().map(|l| Arc::new(Layer(l))).collect()
  }

  /// Total number of layers in the canvas.
  pub fn layer_count(&self) -> u32 {
    self.0.layer_count() as u32
  }

  /// Reorders the layers in the stack matching the given order of IDs (bottom to top).
  pub fn reorder_layers_by_id(&self, new_order_ids: Vec<String>) {
    self.0.reorder_layers_by_id(new_order_ids);
  }

  /// Flattens all layers into a single layer.
  pub fn flatten(&self) {
    self.0.flatten();
  }

  /// Saves the composed canvas to a file path.
  pub fn save(&self, path: String, options: Option<WriterOptions>) {
    self.0.save(path, options);
  }

  /// Converts the entire canvas into a single flattened Image.
  pub fn as_image(&self) -> Arc<Image> {
    Arc::new(Image::from_core(self.0.as_image()))
  }

  /// Creates a handle clone sharing the underlying canvas.
  #[uniffi::method(name = "clone")]
  pub fn duplicate(&self) -> Arc<Canvas> {
    Arc::new(Canvas(self.0.clone()))
  }

  /// Sets effects to apply to the entire canvas.
  pub fn set_effects(&self, effects: Arc<LayerEffects>) {
    self.0.set_effects(effects.0.clone());
  }

  /// Resizes the canvas and its layer tree.
  pub fn resize(&self, target: CanvasResizeTarget, algorithm: Option<TransformAlgorithm>) {
    self.0.transform().resize(target, algorithm);
  }

  /// Crops the canvas and all layers.
  pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) {
    self.0.transform().crop(x, y, width, height);
  }

  /// Rotates all layers by the given degrees.
  pub fn rotate(&self, degrees: f64, algorithm: Option<TransformAlgorithm>) {
    self.0.transform().rotate(degrees, algorithm);
  }

  /// Flips all layers along the specified axis.
  pub fn flip(&self, axis: FlipAxis) {
    self.0.transform().flip(axis);
  }

  /// Returns a fluent transform handler for chaining transform operations.
  pub fn transform(&self) -> Arc<CanvasTransform> {
    Arc::new(CanvasTransform(Arc::new(Canvas(self.0.clone()))))
  }
}

/// A fluent transform proxy for canvas transformations.
#[derive(uniffi::Object)]
pub struct CanvasTransform(Arc<Canvas>);

#[uniffi::export]
impl CanvasTransform {
  /// Resizes the canvas and returns the transform proxy for chaining.
  pub fn resize(&self, target: CanvasResizeTarget, algorithm: Option<TransformAlgorithm>) -> Arc<CanvasTransform> {
    self.0.resize(target, algorithm);
    Arc::new(CanvasTransform(self.0.clone()))
  }

  /// Crops the canvas and returns the transform proxy for chaining.
  pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Arc<CanvasTransform> {
    self.0.crop(x, y, width, height);
    Arc::new(CanvasTransform(self.0.clone()))
  }

  /// Rotates the canvas and returns the transform proxy for chaining.
  pub fn rotate(&self, degrees: f64, algorithm: Option<TransformAlgorithm>) -> Arc<CanvasTransform> {
    self.0.rotate(degrees, algorithm);
    Arc::new(CanvasTransform(self.0.clone()))
  }

  /// Flips the canvas and returns the transform proxy for chaining.
  pub fn flip(&self, axis: FlipAxis) -> Arc<CanvasTransform> {
    self.0.flip(axis);
    Arc::new(CanvasTransform(self.0.clone()))
  }
}

/// A layer within a canvas.
#[derive(uniffi::Object)]
pub struct Layer(library::Layer<'static>);

#[uniffi::export]
impl Layer {
  /// Creates a new standalone layer with a name and image.
  #[uniffi::constructor]
  pub fn new(name: String, image: Arc<Image>) -> Self {
    Self(library::Layer::new(name, Arc::new(image.core())))
  }

  /// The unique UUID of the layer.
  pub fn id(&self) -> String {
    self.0.id()
  }

  /// The name of the layer.
  pub fn name(&self) -> String {
    self.0.name()
  }

  /// Sets the name of the layer.
  pub fn set_name(&self, name: String) {
    self.0.set_name(name);
  }

  /// The width in pixels.
  pub fn width(&self) -> u32 {
    self.0.dimensions().0
  }

  /// The height in pixels.
  pub fn height(&self) -> u32 {
    self.0.dimensions().1
  }

  /// The dimensions as a `Size`.
  pub fn size(&self) -> Size {
    let (width, height) = self.0.dimensions();
    Size {
      width: width as f32,
      height: height as f32,
    }
  }

  /// The dimensions as `[width, height]`.
  pub fn dimensions(&self) -> Vec<u32> {
    let (width, height) = self.0.dimensions();
    vec![width, height]
  }

  /// The opacity of the layer (0.0 to 1.0).
  pub fn opacity(&self) -> f32 {
    self.0.opacity()
  }

  /// Sets the opacity of the layer (0.0 to 1.0).
  pub fn set_opacity(&self, opacity: f32) {
    self.0.set_opacity(opacity);
  }

  /// Limits where the layer shows; the mask must be the size of the layer. `None` removes it.
  pub fn set_mask(&self, mask: Option<Arc<crate::Mask>>) {
    self.0.set_mask(mask.map(|m| m.core()));
  }

  /// Replaces the live effects of the layer: effects made by an effect's `live_effect`, re-run when the canvas composes.
  /// On an adjustment layer they change the picture beneath it. An empty list removes them.
  pub fn set_live_effects(&self, effects: Vec<Arc<crate::LiveEffect>>) {
    self.0.set_live_effects(effects.iter().map(|effect| effect.core()).collect());
  }

  /// The blend mode of the layer.
  pub fn blend_mode(&self) -> BlendMode {
    BlendMode::from(self.0.blend_mode())
  }

  /// Sets the blend mode of the layer.
  pub fn set_blend_mode(&self, blend_mode: BlendMode) {
    self.0.set_blend_mode(blend_mode.to_core());
  }

  /// Whether the layer is visible.
  pub fn is_visible(&self) -> bool {
    self.0.is_visible()
  }

  /// Sets whether the layer is visible.
  pub fn set_visible(&self, visible: bool) {
    self.0.set_visible(visible);
  }

  /// The position of the layer as `[x, y]`.
  pub fn position(&self) -> Vec<i32> {
    let (x, y) = self.0.position();
    vec![x, y]
  }

  /// Sets the position of the layer within the canvas.
  pub fn set_position(&self, x: i32, y: i32) {
    self.0.set_global_position(x, y);
  }

  /// Sets the position of the layer within the canvas.
  pub fn set_global_position(&self, x: i32, y: i32) {
    self.0.set_global_position(x, y);
  }

  /// Sets the position of the layer relative to another layer.
  pub fn set_relative_position(&self, x: i32, y: i32, layer: Arc<Layer>) {
    self.0.set_relative_position(x, y, &layer.0);
  }

  /// The anchor point of the layer.
  pub fn anchor(&self) -> Option<Anchor> {
    self.0.anchor()
  }

  /// Sets the anchor point of the layer.
  pub fn set_anchor(&self, anchor: Option<Anchor>) {
    self.0.set_anchor(anchor);
  }

  /// Anchors the layer to a specific position on the canvas.
  pub fn anchor_to_canvas(&self, anchor: Anchor) {
    self.0.anchor_to_canvas(anchor);
  }

  /// The origin point within the layer for anchor positioning.
  pub fn origin(&self) -> Origin {
    self.0.origin()
  }

  /// Sets the origin point within the layer for anchor positioning.
  pub fn set_origin(&self, origin: Origin) {
    self.0.set_origin(origin);
  }

  /// Current 0-based index of this layer in the layer stack.
  pub fn current_index(&self) -> Option<u32> {
    self.0.current_index().map(|i| i as u32)
  }

  /// Adjustment layer type, or None if not an adjustment layer.
  pub fn adjustment_type(&self) -> Option<AdjustmentLayerType> {
    self.0.adjustment_type()
  }

  /// Moves the layer in the stack according to `LayerMove` (Up, Down, Top, Bottom).
  pub fn move_to(&self, action: LayerMove) {
    self.0.move_to(action);
  }

  /// Moves the layer to the specified index in the stack.
  pub fn set_index(&self, new_index: u32) {
    self.0.set_index(new_index as usize);
  }

  /// Duplicates the layer in the canvas and returns the new layer handle.
  #[uniffi::method(name = "clone")]
  pub fn duplicate(&self) -> Arc<Layer> {
    Arc::new(Layer(self.0.duplicate()))
  }

  /// Marks the parent canvas as needing recomposition.
  pub fn mark_dirty(&self) {
    self.0.mark_dirty();
  }

  /// Returns a copy of the layer's image.
  pub fn image(&self) -> Arc<Image> {
    Arc::new(Image::from_core(self.0.clone_image()))
  }

  /// Returns a flattened copy of the layer's image.
  pub fn as_image(&self) -> Arc<Image> {
    Arc::new(Image::from_core(self.0.as_image()))
  }

  /// Replaces the image of the layer.
  pub fn set_image(&self, image: Arc<Image>) {
    self.0.set_image(Arc::new(image.core()));
  }

  /// Saves the layer image to a file.
  pub fn save(&self, file: String, options: Option<WriterOptions>) {
    self.0.save(file, options);
  }

  /// Sets effects to apply to the layer.
  pub fn set_effects(&self, effects: Arc<LayerEffects>) {
    self.0.set_effects(effects.0.clone());
  }

  /// Resizes the layer image.
  pub fn resize(&self, target: ResizeTarget, algorithm: Option<TransformAlgorithm>) {
    self.0.transform().resize(target, algorithm);
  }

  /// Crops the layer image.
  pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) {
    self.0.transform().crop(x, y, width, height);
  }

  /// Rotates the layer image by the specified degrees.
  pub fn rotate(&self, degrees: f64, algorithm: Option<TransformAlgorithm>) {
    self.0.transform().rotate(degrees, algorithm);
  }

  /// Flips the layer image along the specified axis.
  pub fn flip(&self, axis: FlipAxis) {
    self.0.transform().flip(axis);
  }

  /// Returns a fluent transform handler for chaining transform operations.
  pub fn transform(&self) -> Arc<LayerTransform> {
    Arc::new(LayerTransform(Arc::new(Layer(self.0.clone()))))
  }
}

/// A fluent transform proxy for layer transformations.
#[derive(uniffi::Object)]
pub struct LayerTransform(Arc<Layer>);

#[uniffi::export]
impl LayerTransform {
  /// Resizes the layer and returns the transform proxy for chaining.
  pub fn resize(&self, target: ResizeTarget, algorithm: Option<TransformAlgorithm>) -> Arc<LayerTransform> {
    self.0.resize(target, algorithm);
    Arc::new(LayerTransform(self.0.clone()))
  }

  /// Crops the layer and returns the transform proxy for chaining.
  pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Arc<LayerTransform> {
    self.0.crop(x, y, width, height);
    Arc::new(LayerTransform(self.0.clone()))
  }

  /// Rotates the layer and returns the transform proxy for chaining.
  pub fn rotate(&self, degrees: f64, algorithm: Option<TransformAlgorithm>) -> Arc<LayerTransform> {
    self.0.rotate(degrees, algorithm);
    Arc::new(LayerTransform(self.0.clone()))
  }

  /// Flips the layer and returns the transform proxy for chaining.
  pub fn flip(&self, axis: FlipAxis) -> Arc<LayerTransform> {
    self.0.flip(axis);
    Arc::new(LayerTransform(self.0.clone()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_image(width: u32, height: u32, color: Arc<Color>) -> Arc<Image> {
    Arc::new(Image::new_from_color(width, height, color))
  }

  #[test]
  fn canvas_creation_and_properties() {
    let empty = Canvas::new("Empty".into());
    assert_eq!(empty.name(), "Empty");
    assert_eq!((empty.width(), empty.height()), (0, 0));
    assert_eq!(empty.layer_count(), 0);

    let blank = Canvas::new_blank("Poster".into(), 1920, 1080);
    assert_eq!(blank.name(), "Poster");
    assert_eq!((blank.width(), blank.height()), (1920, 1080));
    assert_eq!(blank.dimensions(), vec![1920, 1080]);
    assert_eq!(blank.size(), Size { width: 1920.0, height: 1080.0 });
    assert_eq!(blank.opacity(), 1.0);
    blank.set_opacity(0.75);
    assert!((blank.opacity() - 0.75).abs() < 1e-4);

    blank.set_blend_mode(BlendMode::Multiply);
    assert_eq!(blank.blend_mode(), BlendMode::Multiply);

    blank.set_position(10, 20);
    assert_eq!(blank.position(), vec![10, 20]);

    blank.set_rotation(Some(45.0));
    assert_eq!(blank.rotation(), Some(45.0));

    blank.set_origin(Origin::TopRight);
    assert_eq!(blank.origin(), Origin::TopRight);

    let duplicate = blank.duplicate();
    assert_eq!(duplicate.name(), "Poster");
  }

  #[test]
  fn canvas_from_unit() {
    let dpi = Resolution { x_dpi: 100.0, y_dpi: 100.0 };
    let canvas = Canvas::new_from_unit("Print".into(), CanvasUnit::Inches(2, 3, dpi));
    assert_eq!((canvas.width(), canvas.height()), (200, 300));
    assert_eq!(canvas.resolution(), dpi);
  }

  #[test]
  fn layers_management_and_ordering() {
    let canvas = Canvas::new_blank("Comp".into(), 400, 400);
    let red = Arc::new(Color::from_rgba(255, 0, 0, 255));
    let blue = Arc::new(Color::from_rgba(0, 0, 255, 255));

    let img1 = test_image(100, 100, red);
    let img2 = test_image(200, 200, blue);

    let l1 = canvas.add_layer_from_image(
      "Layer1".into(),
      img1,
      Some(NewLayerOptions {
        anchor: Some(Anchor::TopLeft),
        opacity: Some(0.8),
        blend_mode: Some(BlendMode::Screen),
        size: Some(LayerSize::Maintain),
      }),
    );
    assert_eq!(l1.name(), "Layer1");
    assert_eq!(l1.width(), 100);
    assert_eq!(l1.height(), 100);
    assert!((l1.opacity() - 0.8).abs() < 1e-4);
    assert_eq!(l1.blend_mode(), BlendMode::Screen);
    assert_eq!(canvas.layer_count(), 1);

    let l2 = canvas.add_layer_from_image("Layer2".into(), img2, None);
    assert_eq!(canvas.layer_count(), 2);

    let found_by_name = canvas.get_layer_by_name("Layer1".into()).unwrap();
    assert_eq!(found_by_name.id(), l1.id());

    let found_by_idx = canvas.get_layer_by_index(0).unwrap();
    assert_eq!(found_by_idx.name(), "Layer1");

    let all_layers = canvas.layers();
    assert_eq!(all_layers.len(), 2);

    // Layer stack reordering (order parameter is bottom to top, canvas.layers index 0 is top)
    let id1 = l1.id();
    let id2 = l2.id();
    canvas.reorder_layers_by_id(vec![id2.clone(), id1.clone()]);
    assert_eq!(canvas.get_layer_by_index(0).unwrap().id(), id1);
    assert_eq!(canvas.get_layer_by_index(1).unwrap().id(), id2);

    // Delete layer
    canvas.delete_layer_by_id(id2);
    assert_eq!(canvas.layer_count(), 1);
  }

  #[test]
  fn layer_transforms_and_image_access() {
    let canvas = Canvas::new_blank("Canvas".into(), 200, 200);
    let color = Arc::new(Color::from_rgba(50, 100, 150, 255));
    let img = test_image(50, 50, color.clone());
    let layer = canvas.add_layer_from_image("Box".into(), img, None);

    // Initial layer dimensions
    assert_eq!((layer.width(), layer.height()), (50, 50));

    // Transforms are queued on the layer and materialized during composition
    layer.crop(5, 5, 30, 30);
    layer.transform().resize(ResizeTarget::Exact(Size { width: 40.0, height: 40.0 }), None);

    // Composing the canvas applies the queued operations
    let composed = canvas.as_image();
    assert_eq!((composed.width(), composed.height()), (200, 200));

    // Image read and write
    let img_copy = layer.image();
    assert_eq!((img_copy.width(), img_copy.height()), (40, 40));

    let new_img = test_image(20, 20, color);
    layer.set_image(new_img);
    assert_eq!((layer.width(), layer.height()), (20, 20));
  }

  #[test]
  fn canvas_transforms_and_composition() {
    let canvas = Canvas::new_blank("Main".into(), 100, 100);
    let color = Arc::new(Color::from_rgba(10, 20, 30, 255));
    canvas.add_layer_from_image("L1".into(), test_image(50, 50, color), None);

    // Flattening and as_image
    let result = canvas.as_image();
    assert_eq!((result.width(), result.height()), (100, 100));

    // Direct canvas transform
    canvas.resize(CanvasResizeTarget::Exact(Size { width: 120.0, height: 80.0 }), None);
    assert_eq!((canvas.width(), canvas.height()), (120, 80));

    // Fluent canvas transform
    canvas.transform().crop(0, 0, 60, 40);
    assert_eq!((canvas.width(), canvas.height()), (60, 40));

    // Flatten layers
    canvas.flatten();
    assert_eq!(canvas.layer_count(), 1);
  }

  #[test]
  fn child_canvas_and_options() {
    let parent = Canvas::new_blank("Parent".into(), 300, 300);
    let child = Arc::new(Canvas::new_blank("Child".into(), 100, 100));

    let child_handle = parent.add_canvas(
      child,
      Some(AddCanvasOptions {
        anchor: Some(Anchor::Center),
        position: Some(vec![10, 10]),
        rotation: Some(15.0),
      }),
    );
    assert_eq!(child_handle.name(), "Child");
  }

  #[test]
  fn an_adjustment_layer_with_a_live_effect_and_mask_changes_only_the_masked_picture_beneath() {
    let canvas = Canvas::new_blank("Adjust".into(), 2, 1);
    canvas.add_layer_from_image("base".into(), test_image(2, 1, Arc::new(Color::from_rgba(100, 100, 100, 255))), None);
    let adjustment = canvas.add_adjustment_layer("lighten".into(), AdjustmentLayerType::BrightnessContrast);
    adjustment.set_live_effects(vec![crate::brightness(40).live_effect()]);
    adjustment.set_mask(Some(Arc::new(crate::Mask::from_values(2, 1, vec![255, 0]))));
    assert_eq!(canvas.as_image().rgba(), vec![140, 140, 140, 255, 100, 100, 100, 255]);

    adjustment.set_live_effects(Vec::new());
    assert_eq!(canvas.as_image().rgba(), vec![100, 100, 100, 255, 100, 100, 100, 255]);
  }

  #[test]
  fn an_adjusted_masked_skin_canvas_between_the_base_and_the_makeup_changes_only_the_skin() {
    let gray = || test_image(3, 1, Arc::new(Color::from_rgba(100, 100, 100, 255)));
    let root = Canvas::new_blank("root".into(), 3, 1);

    let base = Arc::new(Canvas::new_blank("base".into(), 3, 1));
    base.add_layer_from_image("photo".into(), gray(), None);
    root.add_canvas(base, None);

    // The skin canvas holds its own copy of the photo, brightened by an adjustment layer, and shows only where the mask is white.
    let skin = Arc::new(Canvas::new_blank("skin".into(), 3, 1));
    skin.add_layer_from_image("photo".into(), gray(), None);
    let tone = skin.add_adjustment_layer("tone".into(), AdjustmentLayerType::BrightnessContrast);
    tone.set_live_effects(vec![crate::brightness(40).live_effect()]);
    skin.set_mask(Some(Arc::new(crate::Mask::from_values(3, 1, vec![255, 0, 255]))));
    root.add_canvas(skin, None);

    // The makeup covers the last pixel, which is skin, and is on top of it.
    let makeup = Arc::new(Canvas::new_blank("makeup".into(), 3, 1));
    let red = makeup.add_layer_from_image("lips".into(), test_image(1, 1, Arc::new(Color::from_rgba(255, 0, 0, 255))), None);
    red.set_global_position(2, 0);
    root.add_canvas(makeup, None);

    assert_eq!(root.as_image().rgba(), vec![140, 140, 140, 255, 100, 100, 100, 255, 255, 0, 0, 255]);

    // Taking the tone off puts the skin back to the photo, and the makeup stays.
    tone.set_live_effects(Vec::new());
    assert_eq!(root.as_image().rgba(), vec![100, 100, 100, 255, 100, 100, 100, 255, 255, 0, 0, 255]);
  }

  #[test]
  fn tan_and_tone_adjustment_layers_in_a_masked_canvas_match_the_skin_tools() {
    let skin = || test_image(4, 1, Arc::new(Color::from_rgba(200, 140, 100, 255)));
    let values = vec![255, 255, 0, 0];
    let tan = Arc::new(Color::from_rgb(210, 150, 100));

    let canvas = Canvas::new_blank("skin".into(), 4, 1);
    canvas.add_layer_from_image("photo".into(), skin(), None);
    let tint = canvas.add_adjustment_layer("tan".into(), AdjustmentLayerType::BrightnessContrast);
    tint.set_live_effects(vec![crate::color_multiply(tan.clone()).live_effect()]);
    let tone = canvas.add_adjustment_layer("tone".into(), AdjustmentLayerType::BrightnessContrast);
    tone.set_live_effects(vec![crate::exposure(0.0).with_gamma(1.4_f64.powf(0.8)).live_effect()]);

    let root = Canvas::new_blank("root".into(), 4, 1);
    let base = Arc::new(Canvas::new_blank("base".into(), 4, 1));
    base.add_layer_from_image("photo".into(), skin(), None);
    root.add_canvas(base, None);
    canvas.set_mask(Some(Arc::new(crate::Mask::from_values(4, 1, values.clone()))));
    root.add_canvas(Arc::new(canvas), None);
    let layered = root.as_image().rgba();

    let expected = Arc::new(crate::Image::from_core(skin().core()));
    let mask = || Arc::new(crate::Mask::from_values(4, 1, values.clone()));
    crate::skin_tan(tan).with_mask(mask()).apply(expected.clone());
    crate::skin_tone(-80.0).with_mask(mask()).apply(expected.clone());

    assert_ne!(layered[0..4], skin().rgba()[0..4], "the skin changed");
    assert_eq!(layered[8..12], skin().rgba()[8..12], "outside the mask is untouched");
    assert_eq!(layered, expected.rgba());
  }

  #[test]
  fn layer_effects_configuration() {
    let canvas = Canvas::new_blank("EffectsComp".into(), 200, 200);
    let red = Arc::new(Color::from_rgba(255, 0, 0, 255));
    let white = Arc::new(Color::from_rgba(255, 255, 255, 255));
    let layer = canvas.add_layer_from_image("Object".into(), test_image(50, 50, red), None);

    let shadow = DropShadow::new()
      .with_color(white.clone())
      .with_distance(8.0)
      .with_size(4.0)
      .with_opacity(0.5);

    let stroke = Stroke::new()
      .with_size(2)
      .with_color(white)
      .with_position(OutlinePosition::Outside);

    let effects = LayerEffects::new()
      .with_drop_shadow(shadow)
      .with_stroke(stroke);

    layer.set_effects(effects.clone());
    canvas.set_effects(effects);
  }
}

