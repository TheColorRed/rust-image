//! Abra wrapper module around the `drawing` crate.
use abra_core::Channels;
use abra_core::Fill;

// Re-export the raw drawing crate as `abra::drawing::raw`.
pub use drawing as raw;
// Re-export common drawing items for Abra users.
pub use drawing::Brush;
pub use drawing::fill_area_with_brush;
pub use drawing::paint_with_brush;
pub use drawing::stroke_with_brush;

use crate::Image;
use crate::abra_core::Area;

/// Fill an area with the specified fill style, returning an `abra::Image` wrapper.
pub fn fill(p_area: impl Into<Area>, p_fill: impl Into<Fill>) -> Image {
  let core_img = drawing::fill(p_area, p_fill);
  let (w, h) = core_img.dimensions::<u32>();
  Image::new_from_pixels(w, h, core_img.to_rgba_vec(), Channels::RGBA)
}

/// Fill an area using an `abra::Image` as a source image, returning an `abra::Image`.
pub fn fill_from_image(p_area: impl Into<Area>, p_image: &Image) -> Image {
  let core_img = drawing::fill(p_area, p_image);
  let (w, h) = core_img.dimensions::<u32>();
  Image::new_from_pixels(w, h, core_img.to_rgba_vec(), Channels::RGBA)
}
