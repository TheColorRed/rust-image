//! The Canvas public API struct.

use abra_core::BlendMode;
use abra_core::IntoNumber;
use abra_core::Transform;
use abra_core::image::image_ext::ImageExt;
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use crate::canvas::AddCanvasOptions;
use crate::canvas::CoverView;
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

impl<'a> Clone for Canvas<'a> {
  fn clone(&self) -> Self {
    Canvas {
      inner: self.inner.clone(),
    }
  }
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
      let mut guard = inner.lock().unwrap();
      let dirty = guard.dirty_flag();
      for layer_rc in guard.layers.iter_mut() {
        layer_rc.lock().unwrap().set_canvas(&inner, dirty.clone());
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

  /// Sets the factor the canvas is scaled by within its parent, about its middle. 1 is its own size. The canvas keeps its own pixels,
  /// so changing the factor never loses quality.
  pub fn set_scale(&self, p_factor: f32) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.set_scale(p_factor);
  }

  /// Zooms, turns and moves the canvas inside a frame of `p_frame` pixels (width, height) that it has to cover, as a photo sits in a cell:
  /// the view says how far, and the canvas is set to where that puts it, never smaller than covers the frame at its turn and never moved
  /// so far that the frame shows past an edge. It sets the canvas's own position, rotation and scale, so only those are marked changed
  /// and a view that follows a finger costs no more than setting them.
  pub fn set_view_in_frame(&self, p_frame: (u32, u32), p_view: CoverView) {
    let mut canvas = self.inner.lock().unwrap();
    let (width, height) = canvas.dimensions::<u32>();
    let placement = p_view.placement((width as f32, height as f32), (p_frame.0 as f32, p_frame.1 as f32));
    canvas.set_scale(placement.scale);
    canvas.set_rotation(placement.rotation);
    canvas.set_global_position(placement.position.0, placement.position.1);
  }

  /// Gets the factor the canvas is scaled by within its parent
  pub fn scale(&self) -> f32 {
    let canvas = self.inner.lock().unwrap();
    canvas.scale()
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
    let dirty = canvas_rc.lock().unwrap().dirty_flag();
    layer.set_canvas(&canvas_rc, dirty);

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
      canvas.mark_dirty();

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
    layer_rc.lock().unwrap().set_canvas(&canvas_rc, canvas.dirty_flag());
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

  /// The canvases inside this one, in the order they were added. Each is the same canvas as the one that was added, so changing it (its
  /// rotation, scale or position) changes this canvas.
  pub fn canvases(&self) -> Vec<Canvas<'a>> {
    let canvas = self.inner.lock().unwrap();
    canvas.canvases.iter().map(|child| Canvas { inner: child.clone() }).collect()
  }

  /// The four corners of this canvas as it sits in its parent, in the parent's pixels, for drawing an outline around it or testing a
  /// touch against it. They follow its position, its scale about its middle and its rotation about its middle. Top-left, top-right,
  /// bottom-right, bottom-left of the canvas's own picture, so a turn carries each corner round.
  pub fn quad_in_parent(&self) -> abra_core::Quad {
    self.inner.lock().unwrap().quad_in_parent()
  }

  /// The canvas inside this one that is on top at `p_point` (in this canvas's pixels): the last added of the canvases that cover it,
  /// as they are drawn. A canvas that cannot be seen (no opacity, or scaled to nothing) is not found. `None` when no canvas is there.
  /// Layers are not looked at, only the canvases inside.
  pub fn child_at(&self, p_point: impl Into<abra_core::PointF>) -> Option<Canvas<'a>> {
    let point = p_point.into();
    let canvas = self.inner.lock().unwrap();
    canvas
      .canvases
      .iter()
      .rev()
      .find(|child| {
        let child = child.lock().unwrap();
        let quad = child.quad_in_parent();
        child.opacity() > 0.0
          && child.scale() > 0.0
          && abra_core::polygon_contains(&[quad.top_left, quad.top_right, quad.bottom_right, quad.bottom_left], point)
      })
      .map(|child| Canvas { inner: child.clone() })
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

  /// Limits where the canvas shows; the mask must be the size of the canvas. `None` removes it.
  pub fn set_mask(&self, p_mask: Option<mask::Mask>) {
    let mut canvas = self.inner.lock().unwrap();
    canvas.set_mask(p_mask);
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
  use abra_core::Color;

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

  #[test]
  fn canvas_mask_limits_everything_inside_even_a_pass_through_child() {
    let root = Canvas::new_blank("root", 2, 1);
    let child = root.add_canvas(Canvas::new_blank("child", 2, 1), None);
    child.set_pass_through(true);
    child.add_layer_from_image("red", Image::new_from_color(2, 1, Color::from_rgba(255, 0, 0, 255)), None);
    child.set_mask(Some(mask::Mask::from_values(2, 1, vec![255, 0])));

    let image = root.as_image();

    assert_eq!(image.get_pixel(0, 0).unwrap().3, 255, "inside the mask");
    assert_eq!(image.get_pixel(1, 0).unwrap().3, 0, "outside the mask");
  }

  /// A red 4 x 4 canvas in a 20 x 20 root, at `x`, `y`.
  fn red_child_at(x: i32, y: i32) -> (Canvas<'static>, Canvas<'static>) {
    let root = Canvas::new_blank("root", 20, 20);
    let child = Canvas::new_blank("child", 4, 4);
    child.add_layer_from_image("red", Image::new_from_color(4, 4, Color::from_rgba(255, 0, 0, 255)), None);
    let child = root.add_canvas(child, Some(AddCanvasOptions { position: Some((x, y)), ..Default::default() }));
    (root, child)
  }

  #[test]
  fn a_canvas_outside_its_parent_is_not_made_and_shows_when_moved_in() {
    let (root, child) = red_child_at(100, 100);
    root.as_image();
    // It was never brought up to date, so it is still waiting to be, but that does not keep the parent dirty.
    assert!(!root.is_dirty(), "a canvas that is out of sight does not make the parent compose again");

    child.set_position(5, 5);
    let image = root.as_image();

    assert_eq!(image.get_pixel(6, 6), Some((255, 0, 0, 255)), "it shows once it is moved in");
  }

  #[test]
  fn a_canvas_scaled_or_turned_into_view_is_drawn() {
    let (root, child) = red_child_at(30, 8);
    assert_eq!(root.as_image().get_pixel(18, 10), Some((0, 0, 0, 0)), "out of sight at first");

    // Scaled about its middle (32, 10) by 8, it reaches back over the root.
    child.set_scale(8.0);
    // The scaled edge is soft, so the pixel is mostly, not wholly, red.
    assert!(root.as_image().get_pixel(19, 10).unwrap().3 > 100, "scaled into view");
  }

  #[test]
  fn a_canvas_knows_its_corners_as_placed_in_its_parent() {
    let (_root, child) = red_child_at(10, 6);
    let upright = child.quad_in_parent();
    assert_eq!((upright.top_left.x, upright.top_left.y, upright.bottom_right.x, upright.bottom_right.y), (10.0, 6.0, 14.0, 10.0));

    // Scaled by 2 about its middle (12, 8): it reaches 2 further each way.
    child.set_scale(2.0);
    let scaled = child.quad_in_parent();
    assert_eq!((scaled.top_left.x, scaled.top_left.y, scaled.bottom_right.x, scaled.bottom_right.y), (8.0, 4.0, 16.0, 12.0));

    // A quarter turn clockwise about the middle carries the top-left corner to where the top-right was.
    child.set_scale(1.0);
    child.set_rotation(90.0);
    let turned = child.quad_in_parent();
    assert!((turned.top_left.x - 14.0).abs() < 1e-3 && (turned.top_left.y - 6.0).abs() < 1e-3, "{turned:?}");
  }

  /// Where the red marker in the top-left corner of a 20 x 20 child turns up in the root, as its center, after the child is placed at
  /// (30, 30) in a 80 x 80 root, scaled and turned.
  fn marker_after(p_scale: f32, p_degrees: f32) -> ((f32, f32), (f32, f32)) {
    marker_after_with(p_scale, p_degrees, false)
  }

  /// [], with a drop shadow on the child when  is set.
  fn marker_after_with(p_scale: f32, p_degrees: f32, p_shadow: bool) -> ((f32, f32), (f32, f32)) {
    let root = Canvas::new_blank("root", 80, 80);
    let child = Canvas::new_blank("child", 20, 20);
    let marker = child.add_layer_from_image("marker", Image::new_from_color(4, 4, Color::from_rgba(255, 0, 0, 255)), None);
    marker.set_global_position(0, 0);
    let child = root.add_canvas(child, Some(AddCanvasOptions { position: Some((30, 30)), ..Default::default() }));
    if p_shadow {
      child.set_effects(crate::LayerEffects::new().with_drop_shadow(crate::DropShadow::new().with_distance(3).with_size(4)));
    }
    child.set_scale(p_scale);
    child.set_rotation(p_degrees);
    let image = root.as_image();
    let (mut sum_x, mut sum_y, mut count) = (0.0, 0.0, 0.0);
    for y in 0..80 {
      for x in 0..80 {
        let (r, _, _, a) = image.get_pixel(x, y).unwrap();
        if r > 200 && a > 200 {
          (sum_x, sum_y, count) = (sum_x + x as f32, sum_y + y as f32, count + 1.0);
        }
      }
    }
    assert!(count > 0.0, "the marker is drawn");
    let corner = child.quad_in_parent().top_left;
    ((sum_x / count, sum_y / count), (corner.x, corner.y))
  }

  #[test]
  fn the_corners_are_where_the_canvas_is_drawn_when_it_is_scaled_and_turned() {
    // The marker sits in the top-left corner, so it is drawn near the quad's top-left corner, a little way in along each edge of the canvas.
    for (scale, degrees) in [(1.0, 0.0), (2.0, 0.0), (1.0, 90.0), (1.0, 45.0), (1.5, 30.0), (1.0, -60.0)] {
      let ((drawn_x, drawn_y), (corner_x, corner_y)) = marker_after(scale, degrees);
      let distance = ((drawn_x - corner_x).powi(2) + (drawn_y - corner_y).powi(2)).sqrt();
      // The marker's middle is 2 pixels along each edge from the corner, so about 2.8 away, times the scale.
      assert!(distance < 2.9 * scale + 2.0, "scale {scale} turn {degrees}: the marker is at ({drawn_x}, {drawn_y}) and the corner is at ({corner_x}, {corner_y})");
    }
  }

  #[test]
  fn the_corners_are_where_the_canvas_is_drawn_with_a_shadow_on_it() {
    for (scale, degrees) in [(1.0, 0.0), (1.0, 90.0), (1.5, 30.0)] {
      let ((drawn_x, drawn_y), (corner_x, corner_y)) = marker_after_with(scale, degrees, true);
      let distance = ((drawn_x - corner_x).powi(2) + (drawn_y - corner_y).powi(2)).sqrt();
      assert!(distance < 2.9 * scale + 2.0, "shadow, scale {scale} turn {degrees}: the marker is at ({drawn_x}, {drawn_y}) and the corner is at ({corner_x}, {corner_y})");
    }
  }

  #[test]
  fn the_canvas_on_top_at_a_point_is_found() {
    let root = Canvas::new_blank("root", 40, 20);
    let make = |x: i32| {
      let child = Canvas::new_blank("child", 10, 10);
      child.add_layer_from_image("red", Image::new_from_color(10, 10, Color::from_rgba(255, 0, 0, 255)), None);
      root.add_canvas(child, Some(AddCanvasOptions { position: Some((x, 5)), ..Default::default() }))
    };
    let (under, over) = (make(0), make(6));
    assert_eq!(root.child_at((2.0, 8.0)).map(|found| found.id()), Some(under.id()));
    assert_eq!(root.child_at((8.0, 8.0)).map(|found| found.id()), Some(over.id()), "the one added last is on top where they overlap");
    assert!(root.child_at((30.0, 8.0)).is_none());

    // A turned canvas is found where it is, not where its upright box was.
    over.set_rotation(45.0);
    assert_eq!(root.child_at((6.2, 5.2)).map(|found| found.id()), Some(under.id()), "its upright corner was turned away, so the one below shows there");
    assert_eq!(root.child_at((11.0, 4.0)).map(|found| found.id()), Some(over.id()), "its top corner now reaches above where it was");
    assert_eq!(root.child_at((11.0, 10.0)).map(|found| found.id()), Some(over.id()));
    over.set_opacity(0.0);
    assert_eq!(root.child_at((11.0, 10.0)).map(|found| found.id()), None, "an invisible canvas is not found");
  }

  #[test]
  fn an_invisible_canvas_is_not_drawn() {
    let (root, child) = red_child_at(5, 5);
    child.set_opacity(0.0);
    assert_eq!(root.as_image().get_pixel(6, 6), Some((0, 0, 0, 0)));
  }

  #[test]
  fn a_small_layer_is_drawn_at_its_position_on_the_canvas() {
    let canvas = Canvas::new_blank("canvas", 100, 80);
    canvas.add_layer_from_image("base", Image::new_from_color(100, 80, Color::from_rgba(255, 255, 255, 255)), None);
    let patch = canvas.add_layer_from_image("patch", Image::new_from_color(10, 6, Color::from_rgba(255, 0, 0, 255)), None);
    // A new layer is anchored to the middle of the canvas; giving it a position replaces that anchor.
    patch.set_global_position(50, 20);

    let image = canvas.as_image();

    assert_eq!(image.get_pixel(55, 22), Some((255, 0, 0, 255)), "inside the patch");
    assert_eq!(image.get_pixel(49, 22), Some((255, 255, 255, 255)), "left of the patch");
    assert_eq!(image.get_pixel(55, 26), Some((255, 255, 255, 255)), "below the patch");
    assert_eq!(image.get_pixel(55, 60), Some((255, 255, 255, 255)), "far below the patch");
  }

  /// A change to a layer on one thread while the canvas composes on another used to wait forever: the change held the layer's lock and
  /// asked for the canvas's, and the compose held the canvas's lock and asked for the layer's.
  #[test]
  fn changing_a_layer_while_the_canvas_composes_does_not_wait_forever() {
    let canvas = Canvas::new_blank("canvas", 64, 64);
    let layer = canvas.add_layer_from_image("base", Image::new_from_color(64, 64, Color::from_rgba(255, 255, 255, 255)), None);
    let (done, finished) = std::sync::mpsc::channel();
    let composer = {
      let (canvas, done) = (canvas.clone(), done.clone());
      std::thread::spawn(move || {
        for _ in 0..3000 {
          canvas.as_image();
        }
        done.send(()).unwrap();
      })
    };
    let changer = std::thread::spawn(move || {
      for step in 0..3000 {
        layer.set_opacity(if step % 2 == 0 { 0.5 } else { 1.0 });
      }
      done.send(()).unwrap();
    });
    for _ in 0..2 {
      finished.recv_timeout(std::time::Duration::from_secs(60)).expect("the canvas and the layer wait for each other");
    }
    composer.join().unwrap();
    changer.join().unwrap();
  }

  #[test]
  fn a_layer_is_found_and_moved_in_the_stack() {
    let canvas = Canvas::new_blank("canvas", 4, 4);
    let white = Image::new_from_color(4, 4, Color::from_rgba(255, 255, 255, 255));
    let (a, b, c) = (
      canvas.add_layer_from_image("a", white.clone(), None),
      canvas.add_layer_from_image("b", white.clone(), None),
      canvas.add_layer_from_image("c", white, None),
    );
    assert_eq!((a.current_index(), b.current_index(), c.current_index()), (Some(0), Some(1), Some(2)));

    a.move_to(crate::LayerMove::Top);
    assert_eq!((a.current_index(), b.current_index(), c.current_index()), (Some(2), Some(0), Some(1)));

    a.move_to(crate::LayerMove::Down);
    assert_eq!((a.current_index(), b.current_index(), c.current_index()), (Some(1), Some(0), Some(2)));

    c.set_index(0);
    assert_eq!((a.current_index(), b.current_index(), c.current_index()), (Some(2), Some(1), Some(0)));

    // Moving past the top does nothing.
    a.move_to(crate::LayerMove::Up);
    assert_eq!(a.current_index(), Some(2));
    assert!(canvas.is_dirty());
  }

  /// Asking a layer where it sits in the stack, or moving it, while the canvas composes on another thread used to wait forever: the layer
  /// held its own lock and asked for the canvas's, and the compose held the canvas's lock and asked for the layer's.
  #[test]
  fn asking_a_layer_for_its_place_while_the_canvas_composes_does_not_wait_forever() {
    let canvas = Canvas::new_blank("canvas", 64, 64);
    let layer = canvas.add_layer_from_image("base", Image::new_from_color(64, 64, Color::from_rgba(255, 255, 255, 255)), None);
    canvas.add_layer_from_image("top", Image::new_from_color(64, 64, Color::from_rgba(255, 0, 0, 255)), None);
    let (done, finished) = std::sync::mpsc::channel();
    let composer = {
      let (canvas, done) = (canvas.clone(), done.clone());
      std::thread::spawn(move || {
        for _ in 0..3000 {
          canvas.as_image();
        }
        done.send(()).unwrap();
      })
    };
    let asker = std::thread::spawn(move || {
      for _ in 0..3000 {
        layer.current_index();
        layer.set_index(0);
      }
      done.send(()).unwrap();
    });
    for _ in 0..2 {
      finished.recv_timeout(std::time::Duration::from_secs(20)).expect("the canvas and the layer wait for each other");
    }
    composer.join().unwrap();
    asker.join().unwrap();
  }
}
