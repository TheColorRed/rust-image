//! Transform operations for canvases.

use std::sync::Arc;
use std::sync::Mutex;

use crate::transform::{Crop, Resize, Rotate, TransformAlgorithm};

use super::canvas_inner::CanvasInner;

/// A proxy for applying transform operations to a canvas.
/// This type owns the Arc<Mutex<CanvasInner>> and can be used to chain transform operations.
pub struct CanvasTransform {
  pub(super) canvas: Arc<Mutex<CanvasInner>>,
}

impl CanvasTransform {
  /// Creates a new CanvasTransform from an Arc<Mutex<CanvasInner>>
  pub(super) fn new(canvas: Arc<Mutex<CanvasInner>>) -> Self {
    CanvasTransform { canvas }
  }
}

/// Resizes all layers proportionally based on the scaling factor(s).
/// Also updates layer positions to maintain proportional positioning.
///
/// # Arguments
/// * `canvas` - The canvas whose layers should be resized
/// * `scale_x` - The scaling factor for the x-axis (if None, only scale y)
/// * `scale_y` - The scaling factor for the y-axis (if None, only scale x)
/// * `algorithm` - The resize algorithm to use
fn resize_all_layers(
  p_canvas: &mut CanvasInner, p_scale_x: impl Into<Option<f32>>, p_scale_y: impl Into<Option<f32>>,
  p_algorithm: impl Into<Option<TransformAlgorithm>>,
) {
  let algorithm = p_algorithm.into();
  let scale_x = p_scale_x.into();
  let scale_y = p_scale_y.into();
  for i in 0..p_canvas.layers.len() {
    let mut layer = p_canvas.layers[i].lock().unwrap();
    let (old_layer_width, old_layer_height) = layer.dimensions::<u32>();

    let new_layer_width = if let Some(sx) = scale_x {
      (old_layer_width as f32 * sx).round() as u32
    } else {
      old_layer_width
    };

    let new_layer_height = if let Some(sy) = scale_y {
      (old_layer_height as f32 * sy).round() as u32
    } else {
      old_layer_height
    };

    layer.image_mut().resize(new_layer_width, new_layer_height, algorithm);

    // Scale the layer's position proportionally
    let (old_x, old_y) = layer.position();
    let new_x = if let Some(sx) = scale_x {
      (old_x as f32 * sx).round() as i32
    } else {
      old_x
    };
    let new_y = if let Some(sy) = scale_y {
      (old_y as f32 * sy).round() as i32
    } else {
      old_y
    };
    layer.set_position_internal(new_x, new_y);
  }
}
/// Recenter layers horizontally or vertically based on canvas dimensions.
///
/// # Arguments
/// * `canvas` - The canvas whose layers should be recentered
/// * `recenter_x` - If true, recenter horizontally; if false, recenter vertically
fn recenter_layers(canvas: &mut CanvasInner, recenter_x: bool) {
  let (canvas_width, canvas_height) = (canvas.width.get(), canvas.height.get());

  for i in 0..canvas.layers.len() {
    let mut layer = canvas.layers[i].lock().unwrap();
    let (new_layer_width, new_layer_height) = layer.dimensions::<i32>();
    let (x, y) = layer.position();

    if recenter_x {
      let center_x = (canvas_width as i32 - new_layer_width) / 2;
      layer.set_position_internal(center_x, y);
    } else {
      let center_y = (canvas_height as i32 - new_layer_height) / 2;
      layer.set_position_internal(x, center_y);
    }
  }
}

/// Updates the canvas dimensions from the first layer and marks it as needing recomposition.
fn update_canvas_dimensions(canvas: &mut CanvasInner) {
  if let Some(layer) = canvas.layers.get(0) {
    let (new_width, new_height) = layer.lock().unwrap().dimensions::<u32>();
    canvas.width.set(new_width);
    canvas.height.set(new_height);
    canvas.needs_recompose.set(true);
  }
}

/// Crops all layers by calculating their intersection with the crop region.
/// Layers that don't intersect are made empty (0x0).
///
/// # Arguments
/// * `canvas` - The canvas whose layers should be cropped
/// * `crop_x` - The x-coordinate of the crop region
/// * `crop_y` - The y-coordinate of the crop region
/// * `width` - The width of the crop region
/// * `height` - The height of the crop region
fn crop_all_layers(canvas: &mut CanvasInner, crop_x: u32, crop_y: u32, width: u32, height: u32) {
  for i in 0..canvas.layers.len() {
    let mut layer = canvas.layers[i].lock().unwrap();
    let (layer_x, layer_y) = layer.position();
    let (layer_width, layer_height) = layer.image().dimensions::<i32>();

    // Calculate the intersection of the layer with the crop box
    let crop_x_i32 = crop_x as i32;
    let crop_y_i32 = crop_y as i32;
    let width_i32 = width as i32;
    let height_i32 = height as i32;

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

      layer
        .image_mut()
        .crop(crop_left, crop_top, intersect_width, intersect_height);

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

impl Resize for CanvasTransform {
  fn resize(&mut self, p_width: u32, p_height: u32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    {
      let mut canvas = self.canvas.lock().unwrap();

      let old_width = canvas.width.get();
      let old_height = canvas.height.get();

      // Only resize if dimensions have changed
      if p_width != old_width || p_height != old_height {
        let scale_x = if old_width > 0 {
          Some(p_width as f32 / old_width as f32)
        } else {
          Some(1.0)
        };
        let scale_y = if old_height > 0 {
          Some(p_height as f32 / old_height as f32)
        } else {
          Some(1.0)
        };

        resize_all_layers(&mut canvas, scale_x, scale_y, algorithm);

        canvas.width.set(p_width);
        canvas.height.set(p_height);
        canvas.needs_recompose.set(true);
      }
    }
    self
  }

  fn resize_percentage(&mut self, percentage: f32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    let canvas = self.canvas.lock().unwrap();
    let (old_width, old_height) = (canvas.width.get(), canvas.height.get());
    drop(canvas);

    let new_width = (old_width as f32 * percentage).max(1.0) as u32;
    let new_height = (old_height as f32 * percentage).max(1.0) as u32;

    self.resize(new_width, new_height, algorithm)
  }

  fn resize_width(&mut self, p_width: u32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    {
      let mut canvas = self.canvas.lock().unwrap();

      // Store the old canvas width to calculate the scaling factor
      let old_width = canvas.width.get();
      let scale = if old_width > 0 {
        Some(p_width as f32 / old_width as f32)
      } else {
        Some(1.0)
      };

      // Resize all layers proportionally along x-axis only
      resize_all_layers(&mut canvas, scale, None, algorithm);
      update_canvas_dimensions(&mut canvas);
    }
    self
  }

  fn resize_height(&mut self, p_height: u32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    {
      let mut canvas = self.canvas.lock().unwrap();

      // Store the old canvas height to calculate the scaling factor
      let old_height = canvas.height.get();
      let scale = if old_height > 0 {
        Some(p_height as f32 / old_height as f32)
      } else {
        Some(1.0)
      };

      // Resize all layers proportionally along y-axis only
      resize_all_layers(&mut canvas, None, scale, algorithm);
      update_canvas_dimensions(&mut canvas);
    }
    self
  }

  fn resize_width_relative(&mut self, p_width: i32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    {
      let mut canvas = self.canvas.lock().unwrap();
      let algorithm = algorithm.into();

      // Resize all layers
      for i in 0..canvas.layers.len() {
        canvas.layers[i]
          .lock()
          .unwrap()
          .image_mut()
          .resize_width_relative(p_width, algorithm);
      }

      // Update dimensions and recenter horizontally
      update_canvas_dimensions(&mut canvas);
      recenter_layers(&mut canvas, true);
    }
    self
  }

  fn resize_height_relative(&mut self, p_height: i32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    {
      let mut canvas = self.canvas.lock().unwrap();
      let algorithm = algorithm.into();

      // Resize all layers
      for i in 0..canvas.layers.len() {
        canvas.layers[i]
          .lock()
          .unwrap()
          .image_mut()
          .resize_height_relative(p_height, algorithm);
      }

      // Update dimensions and recenter vertically
      update_canvas_dimensions(&mut canvas);
      recenter_layers(&mut canvas, false);
    }
    self
  }
}

impl Crop for CanvasTransform {
  fn crop(&mut self, crop_x: u32, crop_y: u32, width: u32, height: u32) -> &mut Self {
    {
      let mut canvas = self.canvas.lock().unwrap();
      crop_all_layers(&mut canvas, crop_x, crop_y, width, height);
      canvas.width.set(width);
      canvas.height.set(height);
      canvas.needs_recompose.set(true);
    }
    self
  }
}

impl Rotate for CanvasTransform {
  fn rotate(&mut self, p_degrees: impl Into<f64>, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    {
      let canvas = self.canvas.lock().unwrap();
      let algorithm = p_algorithm.into();
      let degrees = p_degrees.into();
      for i in 0..canvas.layers.len() {
        canvas.layers[i].lock().unwrap().image_mut().rotate(degrees, algorithm);
      }
      canvas.needs_recompose.set(true);
    }
    self
  }

  fn flip_horizontal(&mut self) -> &mut Self {
    {
      let canvas = self.canvas.lock().unwrap();
      for i in 0..canvas.layers.len() {
        canvas.layers[i].lock().unwrap().image_mut().flip_horizontal();
      }
      canvas.needs_recompose.set(true);
    }
    self
  }

  fn flip_vertical(&mut self) -> &mut Self {
    {
      let canvas = self.canvas.lock().unwrap();
      for i in 0..canvas.layers.len() {
        canvas.layers[i].lock().unwrap().image_mut().flip_vertical();
      }
      canvas.needs_recompose.set(true);
    }
    self
  }
}
