//! The Canvas public API struct.

use abra_core::BlendMode;
use abra_core::IntoNumber;
use abra_core::Transform;
use abra_core::image::image_ext::ImageExt;
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use crate::canvas::AddCanvasOptions;
use crate::canvas::Origin;

use abra_core::Image;
use abra_core::Resolution;
use abra_core::WriterOptions;
use typography::Text;

use super::canvas_inner::CanvasInner;
use super::canvas_transform::CanvasTransform;
use super::layer::Layer;
use super::layer_inner::LayerInner;
use super::layer_options_applier;
use super::options_new_layer::NewLayerOptions;

/// Content accepted by [`Canvas::add_layer_from_image`].
pub enum LayerContent {
  Image(Arc<Image>),
  Text(Text),
}

/// Converts supported layer inputs into their source content.
pub trait IntoLayerContent {
  fn into_layer_content(self) -> LayerContent;
}

impl IntoLayerContent for Image {
  fn into_layer_content(self) -> LayerContent {
    LayerContent::Image(Arc::new(self))
  }
}

impl IntoLayerContent for Arc<Image> {
  fn into_layer_content(self) -> LayerContent {
    LayerContent::Image(self)
  }
}

impl IntoLayerContent for Text {
  fn into_layer_content(self) -> LayerContent {
    LayerContent::Text(self)
  }
}

/// Units accepted by [`Canvas::new_from_unit`].
///
/// Each variant contains its complete width, height, and resolution
/// specification. Physical dimensions are converted to pixels at construction;
/// changing the resulting canvas resolution later does not resize its pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanvasUnit {
  /// `(width_px, height_px, resolution)` in pixels.
  Pixels(u32, u32, Resolution),
  /// `(width_inches, height_inches, resolution)`.
  Inches(u32, u32, Resolution),
  /// `(width_centimeters, height_centimeters, resolution)`.
  Centimeters(u32, u32, Resolution),
  /// `(width_millimeters, height_millimeters, resolution)`.
  Millimeters(u32, u32, Resolution),
  /// `(width_points, height_points, resolution)`, where 72 points equal one inch.
  Points(u32, u32, Resolution),
  /// `(width_picas, height_picas, resolution)`, where 6 picas equal one inch.
  Picas(u32, u32, Resolution),
  /// `(width_percent, height_percent, reference_width_px, reference_height_px, resolution)`.
  ///
  /// The percentages are resolved against the supplied reference pixel area.
  Percent(u32, u32, u32, u32, Resolution),
  /// `(columns, rows, column_width_px, row_height_px, resolution)`.
  ///
  /// Column dimensions are application-specific, so the cell size is explicit
  /// rather than assumed by the library.
  Columns(u32, u32, u32, u32, Resolution),
}

impl CanvasUnit {
  /// Resolves this unit specification to target pixel dimensions and resolution.
  fn pixel_dimensions(self) -> (u32, u32, Resolution) {
    let (width, height, resolution) = match self {
      Self::Pixels(width, height, resolution) => (f64::from(width), f64::from(height), resolution),
      Self::Inches(width, height, resolution) => {
        (f64::from(width) * f64::from(resolution.x_dpi), f64::from(height) * f64::from(resolution.y_dpi), resolution)
      }
      Self::Centimeters(width, height, resolution) => (
        f64::from(width) / 2.54 * f64::from(resolution.x_dpi),
        f64::from(height) / 2.54 * f64::from(resolution.y_dpi),
        resolution,
      ),
      Self::Millimeters(width, height, resolution) => (
        f64::from(width) / 25.4 * f64::from(resolution.x_dpi),
        f64::from(height) / 25.4 * f64::from(resolution.y_dpi),
        resolution,
      ),
      Self::Points(width, height, resolution) => (
        f64::from(width) / 72.0 * f64::from(resolution.x_dpi),
        f64::from(height) / 72.0 * f64::from(resolution.y_dpi),
        resolution,
      ),
      Self::Picas(width, height, resolution) => (
        f64::from(width) / 6.0 * f64::from(resolution.x_dpi),
        f64::from(height) / 6.0 * f64::from(resolution.y_dpi),
        resolution,
      ),
      Self::Percent(width, height, reference_width, reference_height, resolution) => (
        f64::from(reference_width) * f64::from(width) / 100.0,
        f64::from(reference_height) * f64::from(height) / 100.0,
        resolution,
      ),
      Self::Columns(columns, rows, column_width_px, row_height_px, resolution) => {
        (f64::from(columns * column_width_px), f64::from(rows * row_height_px), resolution)
      }
    };
    (width.round().max(1.0) as u32, height.round().max(1.0) as u32, resolution)
  }
}

impl<'a> Debug for Canvas<'a> {
  fn fmt(&self, p_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let canvas = self.inner.lock().unwrap();
    p_f
      .debug_struct("Canvas")
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
pub struct Canvas<'a> {
  /// Reference to the inner canvas.
  inner: Arc<Mutex<CanvasInner<'a>>>,
}

impl<'a> Default for Canvas<'a> {
  fn default() -> Self {
    Canvas::new("Empty Canvas")
  }
}

impl<'a> Canvas<'a> {
  /// Creates a new project with the given name and an empty canvas of a size of 0x0.
  pub fn new(p_name: impl Into<String>) -> Self {
    Canvas {
      inner: Arc::new(Mutex::new(CanvasInner::new(p_name))),
    }
  }

  /// Creates a new canvas with the given name and a blank canvas of the given size.
  pub fn new_blank(p_name: impl Into<String>, p_width: impl IntoNumber, p_height: impl IntoNumber) -> Self {
    Canvas {
      inner: Arc::new(Mutex::new(CanvasInner::new_blank(p_name, p_width, p_height))),
    }
  }

  /// Creates a canvas from a complete physical, relative, or grid specification.
  ///
  /// Use [`CanvasUnit::Pixels`] when dimensions are already known in pixels.
  /// Physical units convert through their embedded [`Resolution`]; percentage
  /// and column units use their embedded reference values.
  pub fn new_from_unit(p_name: impl Into<String>, p_unit: CanvasUnit) -> Self {
    let (width, height, resolution) = p_unit.pixel_dimensions();
    let canvas = Self::new_blank(p_name, width, height);
    canvas.set_resolution(resolution);
    canvas
  }

  /// Applies target [`CanvasUnit`] resolution metadata, then resamples all
  /// canvas layers through [`CanvasTransform::resize`].
  pub fn resample(&self, p_unit: CanvasUnit) {
    let (width, height, resolution) = p_unit.pixel_dimensions();
    self.set_resolution(resolution);
    self.transform().resize(super::CanvasResizeTarget::Exact(abra_core::Size::new(width, height)), None);
  }

  /// Creates a new project with the given name and a canvas from a path.
  pub fn new_from_path(
    p_name: impl Into<String>, p_path: impl Into<String>, p_options: impl Into<Option<NewLayerOptions>>,
  ) -> Self {
    // Create the inner canvas first, then ensure all layers inside the inner
    // have their `canvas` reference set to this `Arc<Mutex<CanvasInner>>`.
    let inner = Arc::new(Mutex::new(CanvasInner::new_from_path(p_name, p_path, p_options)));
    {
      // Set the canvas reference for each layer to the created Arc so
      // layer-level operations mark the correct parent canvas as dirty.
      let inner_clone = inner.clone();
      let mut guard = inner.lock().unwrap();
      for layer_rc in guard.layers.iter_mut() {
        layer_rc.lock().unwrap().set_canvas(inner_clone.clone());
      }
    }
    Canvas { inner }
  }

  /// Gets the unique ID of the canvas.
  pub fn id(&self) -> String {
    let canvas = self.inner.lock().unwrap();
    canvas.id.clone()
  }

  /// Gets the name of the canvas.
  pub fn name(&self) -> String {
    let canvas = self.inner.lock().unwrap();
    canvas.name.clone()
  }

  /// Saves the canvas to a file.
  pub fn save(&self, p_path: impl Into<String>, p_options: impl Into<Option<WriterOptions>>) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.save(p_path, p_options);
  }

  /// Converts the entire canvas into a single Image by flattening all layers and child canvases
  /// without modifying the original canvas.
  /// Returns a new Image instance containing the flattened canvas.
  pub fn as_image(&self) -> Image {
    let mut canvas = self.inner.lock().unwrap();
    canvas.as_image()
  }

  /// Whether the canvas, or any layer or child canvas in it, has changed since it was last composed (by `as_image`,
  /// `save` or similar).
  pub fn is_dirty(&self) -> bool {
    self.inner.lock().unwrap().is_dirty()
  }

  /// Flattens all layers into a single layer.
  /// All layers will be merged into one layer and removed.
  pub fn flatten(&self) {
    {
      let mut canvas = self.inner.lock().unwrap();
      canvas.flatten();
    }
  }

  /// Updates the canvas by re-compositing all layers and child canvases.
  ///
  /// Internal-only: composition is triggered automatically by `save` and `as_image`.
  /// Returns a new `Canvas` wrapper referencing the same inner canvas.
  #[allow(dead_code)]
  pub(crate) fn update_canvas(&self) -> Self {
    {
      let mut canvas = self.inner.lock().unwrap();
      canvas.update_canvas();
    }

    Canvas {
      inner: self.inner.clone(),
    }
  }

  /// Gets the dimensions of the canvas
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u32>,
    <T as TryFrom<u32>>::Error: std::fmt::Debug,
  {
    let canvas = self.inner.lock().unwrap();
    let width = T::try_from(canvas.width.get()).unwrap();
    let height = T::try_from(canvas.height.get()).unwrap();
    (width, height)
  }

  /// Returns the canvas document resolution.
  pub fn resolution(&self) -> Resolution {
    self.inner.lock().unwrap().resolution
  }

  /// Sets the canvas document resolution without resizing its pixels.
  pub fn set_resolution(&self, p_resolution: Resolution) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.set_resolution(p_resolution);
  }

  /// Gets the position of the canvas within its parent
  pub fn position(&self) -> (i32, i32) {
    let canvas = self.inner.lock().unwrap();
    canvas.position()
  }

  /// Sets the position of the canvas within its parent
  pub fn set_position(&self, p_x: i32, p_y: i32) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.set_global_position(p_x, p_y);
  }

  /// Sets the rotation in degrees for the canvas within its parent
  pub fn set_rotation(&self, p_degrees: impl Into<Option<f32>>) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.set_rotation(p_degrees);
  }

  /// Gets the rotation in degrees for the canvas within its parent
  pub fn rotation(&self) -> Option<f32> {
    let canvas = self.inner.lock().unwrap();
    canvas.rotation()
  }

  /// Sets the origin point (anchor position within the canvas bounds).
  pub fn set_origin(&self, p_origin: Origin) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.set_origin(p_origin);
  }

  /// Gets the origin point (anchor position within the canvas bounds).
  pub fn origin(&self) -> Origin {
    let canvas = self.inner.lock().unwrap();
    canvas.origin()
  }

  /// Adds a new layer from a file path.
  /// Returns the created layer so it can be configured immediately.
  ///
  /// # Example
  /// ```ignore
  /// let project = Canvas::new("My Project");
  /// project.add_layer_from_path("Background", "bg.png", None);
  /// project.add_layer_from_path("Overlay", "overlay.png", Some(NewLayerOptions {
  ///     anchor: Some(Anchor::TopRight),
  /// }));
  /// project.save("output.png", None);
  /// ```
  pub fn add_layer_from_path(
    &self, p_name: impl Into<String>, p_path: impl Into<String>, p_options: impl Into<Option<NewLayerOptions>>,
  ) -> Layer<'a> {
    let image = Arc::new(Image::read(p_path.into()).expect("Failed to load image"));
    self.add_layer_from_image(p_name, image, p_options)
  }

  /// Adds a new layer from an image.
  /// Returns the created layer so it can be configured immediately.
  ///
  /// # Example
  /// ```ignore
  /// let img = Image::read("assets/image.png")?;
  /// let project = Canvas::new("My Project");
  /// project.add_layer_from_image("White Layer", img, None);
  /// ```
  pub fn add_layer_from_image<I: IntoLayerContent>(
    &self, p_name: impl Into<String>, p_image: I, p_options: impl Into<Option<NewLayerOptions>>,
  ) -> Layer<'a> {
    let resolution = self.resolution();
    let canvas_rc = self.inner.clone();
    let options = p_options.into();
    let mut layer = match p_image.into_layer_content() {
      LayerContent::Image(p_image) => LayerInner::new(p_name, p_image),
      LayerContent::Text(text) => LayerInner::new_text(p_name, text, resolution),
    };
    layer.set_canvas(canvas_rc);

    // Determine if this is the first layer before modifying canvas
    let is_first_layer = {
      let canvas = self.inner.lock().unwrap();
      canvas.width.get() == 0 && canvas.height.get() == 0
    };

    if is_first_layer {
      layer.prepare_for_composition(resolution);
    }

    let layer_rc = Arc::new(Mutex::new(layer));

    // Add to canvas
    {
      let mut canvas = self.inner.lock().unwrap();
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
      let mut canvas = self.inner.lock().unwrap();
      if new_width > 0 && new_height > 0 {
        canvas.set_canvas_size(new_width, new_height);
      }
    }

    Layer::from_inner(layer_rc)
  }

  /// Adds a new adjustment layer to the canvas.
  /// Returns the created layer so it can be configured immediately.
  pub fn add_adjustment_layer(
    &self, p_name: impl Into<String>, p_adjustment_type: crate::AdjustmentLayerType,
  ) -> Layer<'a> {
    let canvas_rc = self.inner.clone();
    let mut canvas = canvas_rc.lock().unwrap();
    let layer_rc = canvas.add_adjustment_layer(p_name, p_adjustment_type);
    layer_rc.lock().unwrap().set_canvas(canvas_rc.clone());
    Layer::from_inner(layer_rc)
  }

  /// Deletes a layer by its ID from the canvas.
  /// If the layer is not found, no action is taken.
  pub fn delete_layer_by_id(&self, p_layer_id: &str) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.delete_layer_by_id(p_layer_id);
  }

  /// Adds a new canvas as a child canvas.
  /// Adds a child canvas and returns a handle to it.
  ///
  /// The child inherits this canvas's resolution before the handle is returned,
  /// allowing text added through the returned handle to use inherited DPI.
  pub fn add_canvas(&self, p_canvas: Canvas<'a>, p_options: impl Into<Option<AddCanvasOptions>>) -> Canvas<'a> {
    let child_inner = p_canvas.inner;
    let mut inner_canvas = self.inner.lock().unwrap();
    inner_canvas.add_canvas_rc(child_inner.clone(), p_options);
    Canvas { inner: child_inner }
  }

  /// Sets the position of this canvas to the given anchor point within its parent canvas.
  pub fn anchor_to_canvas(&self, p_anchor: crate::Anchor) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.anchor_to_canvas(p_anchor);
  }

  /// Applies the stored anchor to position the canvas using parent dimensions
  #[allow(dead_code)]
  pub(crate) fn apply_anchor_with_parent_dimensions(&self, p_parent_width: i32, p_parent_height: i32) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.apply_anchor_with_parent_dimensions(p_parent_width, p_parent_height);
  }

  /// Sets the effects to apply to the entire canvas.
  /// - `p_effects`: the LayerEffects to apply.
  pub fn set_effects(&self, p_effects: crate::LayerEffects<'a>) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.set_effects(p_effects);
  }

  /// Gets a layer by its index.
  /// Returns None if the index is out of bounds.
  pub fn get_layer_by_index(&self, p_index: usize) -> Option<Layer<'a>> {
    let canvas = self.inner.lock().unwrap();
    canvas.layers.get(p_index).cloned().map(Layer::from_inner)
  }

  /// Gets a layer by its UUID.
  /// Returns None if the index is out of bounds.
  pub fn get_layer_by_id(&self, p_id: &str) -> Option<Layer<'a>> {
    let canvas = self.inner.lock().unwrap();
    canvas
      .layers
      .iter()
      .find(|layer_rc| {
        let layer = layer_rc.lock().unwrap();
        layer.id() == p_id
      })
      .cloned()
      .map(Layer::from_inner)
  }

  /// Gets a layer by its name.
  /// Returns the first layer with the matching name, or None if not found.
  pub fn get_layer_by_name(&self, p_name: impl Into<String>) -> Option<Layer<'a>> {
    let p_name = p_name.into();
    let canvas = self.inner.lock().unwrap();
    canvas
      .layers
      .iter()
      .find(|layer_rc| {
        let layer = layer_rc.lock().unwrap();
        layer.name() == p_name
      })
      .cloned()
      .map(Layer::from_inner)
  }

  /// Gets all layers in the canvas.
  pub fn layers(&self) -> Vec<Layer<'a>> {
    let canvas = self.inner.lock().unwrap();
    canvas.layers.iter().cloned().map(Layer::from_inner).collect()
  }

  /// Reorders the layers in the layer stack according to the given array of layer IDs.
  /// If any IDs are not found or if there are duplicate IDs, no changes are made.
  /// # Parameters
  /// - `p_new_order_ids`: A vector of layer IDs in the desired order (from bottom to top).
  pub fn reorder_layers_by_id(&self, p_new_order_ids: Vec<String>) {
    // Make sure all IDs are unique
    // If there are duplicates, exit early
    let unique_ids: HashSet<_> = p_new_order_ids.iter().collect();
    if unique_ids.len() != p_new_order_ids.len() {
      return;
    }

    // Reorder the layers
    let mut canvas = self.inner.lock().unwrap();
    canvas.layers = p_new_order_ids
      .iter()
      .rev()
      .filter_map(|id| canvas.layers.iter().find(|layer_rc| layer_rc.lock().unwrap().id() == *id).cloned())
      .collect();

    canvas.mark_dirty();
  }

  /// Gets the number of layers in the canvas.
  pub fn layer_count(&self) -> usize {
    let canvas = self.inner.lock().unwrap();
    canvas.layers.len()
  }

  /// Gets a clone of the result image (internal use only).
  /// This is used when compositing child canvases.
  #[allow(dead_code)]
  pub(crate) fn get_result_image(&self) -> Image {
    let canvas = self.inner.lock().unwrap();
    canvas.get_result_image()
  }

  /// Sets the blend mode used when compositing this canvas into a parent.
  pub fn set_blend_mode(&self, p_blend_mode: BlendMode) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.set_blend_mode(p_blend_mode);
  }

  /// Gets the blend mode used when compositing this canvas into a parent.
  pub fn blend_mode(&self) -> BlendMode {
    let canvas = self.inner.lock().unwrap();
    canvas.blend_mode()
  }

  /// Sets whether this canvas is pass-through.
  pub fn set_pass_through(&self, p_pass: bool) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.set_pass_through(p_pass);
  }

  /// Gets whether this canvas is a pass-through group.
  pub fn pass_through(&self) -> bool {
    let canvas = self.inner.lock().unwrap();
    canvas.pass_through()
  }

  /// Sets the canvas opacity (0.0 - 1.0).
  pub fn set_opacity(&self, p_opacity: f32) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.set_opacity(p_opacity);
  }

  /// Gets the canvas opacity (0.0 - 1.0).
  pub fn opacity(&self) -> f32 {
    let canvas = self.inner.lock().unwrap();
    canvas.opacity()
  }

  /// Returns a handler for applying transform operations to the canvas
  pub fn transform(&self) -> CanvasTransform<'a> {
    CanvasTransform::new(self.inner.clone())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn resample_scales_child_canvases_and_layers() {
    let root = Canvas::new_blank("root", 10, 10);
    let child = root.add_canvas(Canvas::new_blank("child", 5, 5), None);
    let layer = child.add_layer_from_image("source", Image::new(5, 5), None);

    root.resample(CanvasUnit::Pixels(20, 20, Resolution::SCREEN));

    assert_eq!(root.dimensions::<u32>(), (20, 20));
    assert_eq!(child.dimensions::<u32>(), (10, 10));
    root.as_image();
    assert_eq!(layer.dimensions(), (10, 10));
  }
}
