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
