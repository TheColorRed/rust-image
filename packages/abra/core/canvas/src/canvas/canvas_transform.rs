//! Transform operations for canvases.

use abra_core::{FlipAxis, IntoNumber, ResizeTarget, Size, Transform, TransformAlgorithm};
use std::sync::Arc;
use std::sync::Mutex;

use super::canvas_inner::CanvasInner;

/// A proxy for applying transform operations to a canvas.
/// This type owns the Arc<Mutex<CanvasInner>> and can be used to chain transform operations.
pub struct CanvasTransform<'a> {
  pub(super) canvas: Arc<Mutex<CanvasInner<'a>>>,
}

/// Describes a canvas resize operation.
///
/// Canvas stretch targets intentionally differ from image fit targets: they scale
/// one canvas axis without preserving the aspect ratio of its layer tree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanvasResizeTarget {
  /// Resize both axes to exact dimensions.
  Exact(Size),
  /// Stretch only the horizontal axis.
  StretchWidth(u32),
  /// Stretch only the vertical axis.
  StretchHeight(u32),
  /// Scale both axes by a factor.
  Scale(f32),
  /// Change every layer's width by a pixel amount and recenter horizontally.
  RelativeWidth(i32),
  /// Change every layer's height by a pixel amount and recenter vertically.
  RelativeHeight(i32),
}

impl<'a> CanvasTransform<'a> {
  /// Creates a new CanvasTransform from an Arc<Mutex<CanvasInner>>
  pub(super) fn new(p_canvas: Arc<Mutex<CanvasInner<'a>>>) -> Self {
    CanvasTransform { canvas: p_canvas }
  }
}

/// Recenter layers horizontally or vertically based on canvas dimensions.
///
/// # Arguments
/// * `p_canvas` - The canvas whose layers should be recentered
/// * `p_recenter_x` - If true, recenter horizontally; if false, recenter vertically
fn recenter_layers(p_canvas: &mut CanvasInner, p_recenter_x: bool) {
  let (canvas_width, canvas_height) = (p_canvas.width.get(), p_canvas.height.get());

  for i in 0..p_canvas.layers.len() {
    let mut layer = p_canvas.layers[i].lock().unwrap();
    let (new_layer_width, new_layer_height) = layer.dimensions::<i32>();
    let (x, y) = layer.position();

    if p_recenter_x {
      let center_x = (canvas_width as i32 - new_layer_width) / 2;
      layer.set_position_internal(center_x, y);
    } else {
      let center_y = (canvas_height as i32 - new_layer_height) / 2;
      layer.set_position_internal(x, center_y);
    }
  }
}

/// Updates the canvas dimensions from the first layer and marks it as needing recomposition.
fn update_canvas_dimensions(p_canvas: &mut CanvasInner) {
  if let Some(layer) = p_canvas.layers.get(0) {
    let (new_width, new_height) = layer.lock().unwrap().dimensions::<u32>();
    p_canvas.width.set(new_width);
    p_canvas.height.set(new_height);
    p_canvas.mark_dirty();
  }
}

/// Crops all layers by calculating their intersection with the crop region.
/// Layers that don't intersect are made empty (0x0).
///
/// # Arguments
/// * `p_canvas` - The canvas whose layers should be cropped
/// * `p_crop_x` - The x-coordinate of the crop region
/// * `p_crop_y` - The y-coordinate of the crop region
/// * `p_width` - The width of the crop region
/// * `p_height` - The height of the crop region
fn crop_all_layers(p_canvas: &mut CanvasInner, p_crop_x: u32, p_crop_y: u32, p_width: u32, p_height: u32) {
  for i in 0..p_canvas.layers.len() {
    let mut layer = p_canvas.layers[i].lock().unwrap();
    let (layer_x, layer_y) = layer.position();
    let (layer_width, layer_height) = layer.image().dimensions::<i32>();

    // Calculate the intersection of the layer with the crop box
    let crop_x_i32 = p_crop_x as i32;
    let crop_y_i32 = p_crop_y as i32;
    let width_i32 = p_width as i32;
    let height_i32 = p_height as i32;

    // Find the intersection rectangle
    let intersect_left = (layer_x).max(crop_x_i32);
    let intersect_top = (layer_y).max(crop_y_i32);
    let intersect_right = (layer_x + layer_width).min(crop_x_i32 + width_i32);
    let intersect_bottom = (layer_y + layer_height).min(crop_y_i32 + height_i32);

    if intersect_left < intersect_right && intersect_top < intersect_bottom {
      // There is an intersection - crop the layer to this intersection
      let crop_left = (intersect_left - layer_x) as u32;
      let crop_top = (intersect_top - layer_y) as u32;
      let intersect_width = (intersect_right - intersect_left) as u32;
      let intersect_height = (intersect_bottom - intersect_top) as u32;

      layer.image_mut().crop(crop_left, crop_top, intersect_width, intersect_height);

      // Update layer position to be relative to the new canvas
      let new_x = intersect_left - crop_x_i32;
      let new_y = intersect_top - crop_y_i32;
      layer.set_position_internal(new_x, new_y);
    } else {
      // No intersection - this layer won't be visible after crop
      layer.image_mut().crop(0, 0, 0, 0);
      layer.set_position_internal(0, 0);
    }
  }
}

impl<'a> CanvasTransform<'a> {
  fn resize_exact(&mut self, p_width: u32, p_height: u32, p_algorithm: Option<TransformAlgorithm>) {
    {
      let mut canvas = self.canvas.lock().unwrap();
      let old_width = canvas.width.get();
      let old_height = canvas.height.get();

      // Only resize if dimensions have changed
      if p_width != old_width || p_height != old_height {
        let scale_x = if old_width > 0 { Some(p_width as f32 / old_width as f32) } else { Some(1.0) };
        let scale_y = if old_height > 0 { Some(p_height as f32 / old_height as f32) } else { Some(1.0) };

        canvas.rescale_tree(scale_x, scale_y, p_algorithm);

        canvas.width.set(p_width);
        canvas.height.set(p_height);
        canvas.mark_dirty();
      }
    }
  }

  fn stretch_width(&mut self, p_width: u32, p_algorithm: Option<TransformAlgorithm>) {
    {
      let mut canvas = self.canvas.lock().unwrap();

      // Store the old canvas width to calculate the scaling factor
      let old_width = canvas.width.get();
      let scale = if old_width > 0 { Some(p_width as f32 / old_width as f32) } else { Some(1.0) };

      canvas.rescale_tree(scale, None, p_algorithm);
      canvas.width.set(p_width);
      canvas.mark_dirty();
    }
  }

  fn stretch_height(&mut self, p_height: u32, p_algorithm: Option<TransformAlgorithm>) {
    {
      let mut canvas = self.canvas.lock().unwrap();

      // Store the old canvas height to calculate the scaling factor
      let old_height = canvas.height.get();
      let scale = if old_height > 0 { Some(p_height as f32 / old_height as f32) } else { Some(1.0) };

      canvas.rescale_tree(None, scale, p_algorithm);
      canvas.height.set(p_height);
      canvas.mark_dirty();
    }
  }

  fn resize_relative_width(&mut self, p_width: i32, p_algorithm: Option<TransformAlgorithm>) {
    {
      let mut canvas = self.canvas.lock().unwrap();
      // Resize all layers
      for i in 0..canvas.layers.len() {
        canvas.layers[i].lock().unwrap().image_mut().resize(ResizeTarget::RelativeWidth(p_width), p_algorithm);
      }

      // Update dimensions and recenter horizontally
      update_canvas_dimensions(&mut canvas);
      recenter_layers(&mut canvas, true);
    }
  }

  fn resize_relative_height(&mut self, p_height: i32, p_algorithm: Option<TransformAlgorithm>) {
    {
      let mut canvas = self.canvas.lock().unwrap();
      // Resize all layers
      for i in 0..canvas.layers.len() {
        canvas.layers[i].lock().unwrap().image_mut().resize(ResizeTarget::RelativeHeight(p_height), p_algorithm);
      }

      // Update dimensions and recenter vertically
      update_canvas_dimensions(&mut canvas);
      recenter_layers(&mut canvas, false);
    }
  }
}

impl<'a> Transform for CanvasTransform<'a> {
  type Target = CanvasResizeTarget;

  /// Resize the canvas and its layer tree according to the supplied target.
  fn resize(&mut self, p_target: CanvasResizeTarget, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
    let algorithm = p_algorithm.into();
    match p_target {
      CanvasResizeTarget::Exact(size) => {
        self.resize_exact(size.width.max(0.0) as u32, size.height.max(0.0) as u32, algorithm)
      }
      CanvasResizeTarget::StretchWidth(width) => self.stretch_width(width, algorithm),
      CanvasResizeTarget::StretchHeight(height) => self.stretch_height(height, algorithm),
      CanvasResizeTarget::Scale(scale) => {
        let canvas = self.canvas.lock().unwrap();
        let (width, height) = (canvas.width.get(), canvas.height.get());
        drop(canvas);
        self.resize_exact((width as f32 * scale).max(1.0) as u32, (height as f32 * scale).max(1.0) as u32, algorithm);
      }
      CanvasResizeTarget::RelativeWidth(amount) => self.resize_relative_width(amount, algorithm),
      CanvasResizeTarget::RelativeHeight(amount) => self.resize_relative_height(amount, algorithm),
    }
  }

  fn crop(
    &mut self, p_crop_x: impl IntoNumber, p_crop_y: impl IntoNumber, p_width: impl IntoNumber,
    p_height: impl IntoNumber,
  ) {
    let (p_crop_x, p_crop_y, p_width, p_height) =
      (p_crop_x.into::<u32>(), p_crop_y.into::<u32>(), p_width.into::<u32>(), p_height.into::<u32>());
    {
      let mut canvas = self.canvas.lock().unwrap();
      crop_all_layers(&mut canvas, p_crop_x, p_crop_y, p_width, p_height);
      canvas.width.set(p_width);
      canvas.height.set(p_height);
      canvas.mark_dirty();
    }
  }

  fn rotate(&mut self, p_degrees: impl IntoNumber, p_algorithm: impl Into<Option<TransformAlgorithm>>) {
    {
      let canvas = self.canvas.lock().unwrap();
      let algorithm = p_algorithm.into();
      let degrees = p_degrees.into::<f64>();
      for i in 0..canvas.layers.len() {
        canvas.layers[i].lock().unwrap().image_mut().rotate(degrees, algorithm);
      }
      canvas.mark_dirty();
    }
  }

  fn flip(&mut self, p_axis: FlipAxis) {
    {
      let canvas = self.canvas.lock().unwrap();
      for i in 0..canvas.layers.len() {
        canvas.layers[i].lock().unwrap().image_mut().flip(p_axis);
      }
      canvas.mark_dirty();
    }
  }
}
