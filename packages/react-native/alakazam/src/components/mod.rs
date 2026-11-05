pub mod color_bar_picker;
pub mod color_square;
pub mod curve_editor;
pub mod gradient_editor;
pub mod image_preview;
pub mod gizmos;
pub mod thumbnail_preview;
mod surface;

use vessel::prelude::Frame;

pub(crate) fn frame_of(image: &abra::prelude::Image) -> Frame {
  let (width, height) = image.dimensions::<u32>();
  Frame { width, height, pixels: image.rgba().to_vec().into() }
}
