use abra::prelude::*;

use crate::area::Area;
use crate::common::*;

/// Returns the pixel data for a specified rectangular area of the project canvas.
///
/// - `project`: Reference to the project.
/// - `area`: A vector of four `u32` values representing `[x, y, width, height]`.
///
/// # Example
/// ```ignore
/// let pixels = get_pixels(&project, vec![0, 0, 100, 100]);
/// ```
#[napi]
pub fn get_pixels(project: &Project, area: &Area) -> ImageData {
  let image = project.canvas().as_image();
  let image_data = image.get_rgba_in_area(&area.inner);
  let (min_x, min_y, max_x, max_y) = area.inner.bounds::<f32>();
  let width = (max_x - min_x) as u32;
  let height = (max_y - min_y) as u32;

  ImageData {
    data: Buffer::from(image_data),
    width,
    height,
  }
}

#[napi]
pub fn sample_color(
  project: &Project, x: u32, y: u32, width: u32, height: u32, style: String, layer: Option<&Layer>,
) -> crate::color::Color {
  let image = match style.as_str() {
    "all" => project.canvas().as_image(),
    "current-layer" => {
      if let Some(layer) = layer {
        let target_layer_id = layer.id().to_string();

        // SAFETY: We need to extend the lifetime of the canvas reference to satisfy the
        // invariant lifetime parameter of Project. This is safe because:
        // 1. We only use the canvas for the duration of this function
        // 2. We don't store any references beyond the function scope
        // 3. The canvas itself is owned by the project which lives for the entire NAPI call
        let canvas: &abra::canvas::prelude::Canvas<'static> = unsafe { std::mem::transmute(project.canvas()) };

        // Collect current visibility states and layer IDs
        let current_layer_visibility: Vec<(String, bool)> =
          canvas.layers().iter().map(|l| (l.id().to_string(), l.is_visible())).collect();

        // Hide all other layers except the target layer
        for (layer_id, _) in &current_layer_visibility {
          if let Some(l) = canvas.get_layer_by_id(layer_id) {
            l.set_visible(layer_id == &target_layer_id);
          }
        }

        let image_data = canvas.as_image();

        // Restore original visibility of all layers
        for (layer_id, visible) in current_layer_visibility {
          if let Some(l) = canvas.get_layer_by_id(&layer_id) {
            l.set_visible(visible);
          }
        }

        image_data
      } else {
        project.canvas().as_image()
      }
    }
    _ => project.canvas().as_image(),
  };

  #[inline]
  fn get_average_color(pixels: Vec<(u8, u8, u8, u8)>) -> (u8, u8, u8, u8) {
    let mut r_total: u32 = 0;
    let mut g_total: u32 = 0;
    let mut b_total: u32 = 0;
    let mut a_total: u32 = 0;
    let count = pixels.len() as u32;

    for (r, g, b, a) in pixels {
      r_total += r as u32;
      g_total += g as u32;
      b_total += b as u32;
      a_total += a as u32;
    }

    ((r_total / count) as u8, (g_total / count) as u8, (b_total / count) as u8, (a_total / count) as u8)
  }

  let color = if width == 1 && height == 1 {
    image.get_pixel(x, y).unwrap_or((0, 0, 0, 0))
  } else {
    let pixels = image.get_pixels((x, y, width as u32, height as u32));
    get_average_color(pixels)
  };

  crate::color::Color::from_rgba(color.0, color.1, color.2, color.3)
}
