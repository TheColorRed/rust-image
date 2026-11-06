mod camera;
pub mod handles;
pub mod color_bar_picker;
pub mod color_picker;
pub mod curve_editor;
pub mod gradient_editor;
pub mod image_preview;
pub mod gizmos;
pub mod thumbnail_preview;

use abra::abra_core::{Color, Image as Pixels};
use vessel::prelude::{Component, Frame};

pub(crate) fn frame_of(image: &abra::prelude::Image) -> Frame {
  let (width, height) = image.dimensions::<u32>();
  Frame { width, height, pixels: image.rgba().to_vec().into() }
}

/// What the picker components (the color picker, the curve editor, the gradient editor) are made of: a child component that shows the picture
/// `p_paint` makes at the child's own size. Vessel keeps the child's picture until its size changes or a property that `p_paint` reads (with
/// `value()`) changes, so a layer that does not depend on a drag is not drawn again by it, and a mark that is only moved (`Component::place`) is
/// drawn once. The only thing here is turning abra's image into the Frame Vessel shows.
pub(crate) fn layer(p_name: &str, p_paint: impl Fn((u32, u32)) -> Pixels + Send + Sync + 'static) -> Component {
  Component::painted(p_name, move |size| frame_of(&p_paint(size)))
}

/// A color from a list of numbers: red, green, blue and alpha, each 0 to 255.
///
/// # Panics
/// When there are fewer than four.
pub(crate) fn color_of(p_color: &[u8]) -> Color {
  Color::from_rgba(p_color[0], p_color[1], p_color[2], p_color[3])
}

/// The RGBA of the pixel at `p_x`, `p_y` of a frame that Vessel drew.
#[cfg(test)]
pub(crate) fn pixel(p_frame: &Frame, p_x: u32, p_y: u32) -> [u8; 4] {
  let at = ((p_y * p_frame.width + p_x) * 4) as usize;
  p_frame.pixels[at..at + 4].try_into().unwrap()
}
