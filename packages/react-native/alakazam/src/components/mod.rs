pub mod color_bar_picker;
pub mod image_preview;
pub mod gizmos;
pub mod thumbnail_preview;

use vessel::prelude::Frame;

pub(crate) fn frame_of(image: &abra::prelude::Image) -> Frame {
  let (width, height) = image.dimensions::<u32>();
  Frame { width, height, pixels: image.rgba().to_vec().into() }
}
