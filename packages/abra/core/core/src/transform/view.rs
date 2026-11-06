//! Looking at part of an image through a window: [`view`] makes the picture of an image seen through a rectangle that can be turned, zoomed
//! out past the image's edges, and shown at any size. It is what a viewer draws when it has to show the image without a GPU.

use super::{EdgeMode, Interpolation, ResizeTarget, remap, transform};
use crate::{Channels, Color, Image, Rect, Size};

/// A rectangle laid over an image, in the image's pixels, that decides what [`view`] shows. It can reach past the image's edges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewWindow {
  /// The left edge of the rectangle.
  pub x: f32,
  /// The top edge of the rectangle.
  pub y: f32,
  /// The width of the rectangle.
  pub width: f32,
  /// The height of the rectangle.
  pub height: f32,
  /// How far the image is turned, in radians clockwise, about the middle of the rectangle. The rectangle stays upright in the result.
  pub angle: f32,
}

impl ViewWindow {
  /// An upright rectangle with its top left corner at `p_x`, `p_y`.
  pub fn new(p_x: f32, p_y: f32, p_width: f32, p_height: f32) -> Self {
    Self {
      x: p_x,
      y: p_y,
      width: p_width,
      height: p_height,
      angle: 0.0,
    }
  }

  /// The same rectangle with the image turned `p_angle` radians clockwise about its middle.
  pub fn with_angle(mut self, p_angle: f32) -> Self {
    self.angle = p_angle;
    self
  }
}

/// The picture of `p_image` seen through `p_window`, drawn to fill a box of `p_size` pixels. The result is opaque: where the window reaches
/// past the image it shows `p_background`, and the image's own transparency is not shown.
///
/// An upright window cuts that part of the image out and resizes it as [`transform`] does, so a part shown smaller than it is keeps its detail.
/// A turned window looks up each pixel of the result in the image, smoothly, because turning leaves no whole pixels to copy.
/// - `p_image`: The image to look at.
/// - `p_window`: The part to show, in the image's pixels.
/// - `p_size`: The width and height of the result.
/// - `p_background`: The color shown where the window is not over the image.
pub fn view(p_image: &Image, p_window: ViewWindow, p_size: (u32, u32), p_background: Color) -> Image {
  let (width, height) = p_size;
  let background = [p_background.r, p_background.g, p_background.b, 255];
  let mut pixels = background.repeat(width as usize * height as usize);
  let (image_width, image_height) = p_image.dimensions::<u32>();
  let valid = width > 0 && height > 0 && image_width > 0 && image_height > 0 && p_window.width > 0.0 && p_window.height > 0.0;
  if valid {
    if p_window.angle.rem_euclid(std::f32::consts::TAU) > f32::EPSILON {
      view_turned(p_image, &p_window, p_size, &mut pixels);
    } else {
      view_upright(p_image, &p_window, p_size, &mut pixels);
    }
  }
  Image::new_from_pixels(width, height, pixels, Channels::RGBA)
}

/// The part of the window over the image, cut out, resized to where it lands in the box, and copied over `p_pixels`.
fn view_upright(p_image: &Image, p_window: &ViewWindow, p_size: (u32, u32), p_pixels: &mut [u8]) {
  let (image_width, image_height) = p_image.dimensions::<u32>();
  let (width, height) = p_size;
  let (left, top) = (p_window.x.max(0.0), p_window.y.max(0.0));
  let (right, bottom) = ((p_window.x + p_window.width).min(image_width as f32), (p_window.y + p_window.height).min(image_height as f32));
  if right <= left || bottom <= top {
    return;
  }
  // Where the part lands in the box, in whole pixels.
  let to_box = |p_at: f32, p_from: f32, p_extent: f32, p_room: u32| ((p_at - p_from) / p_extent * p_room as f32).round().clamp(0.0, p_room as f32) as u32;
  let x = to_box(left, p_window.x, p_window.width, width).min(width - 1);
  let y = to_box(top, p_window.y, p_window.height, height).min(height - 1);
  let target_width = to_box(right, p_window.x, p_window.width, width).saturating_sub(x).max(1);
  let target_height = to_box(bottom, p_window.y, p_window.height, height).saturating_sub(y).max(1);

  // The pixels that cover the part, cut out as they are, then resized to fit where they land.
  let (cut_left, cut_top) = (left.floor() as u32, top.floor() as u32);
  let (cut_right, cut_bottom) = ((right.ceil() as u32).min(image_width), (bottom.ceil() as u32).min(image_height));
  let (cut_left, cut_top) = (cut_left.min(image_width - 1), cut_top.min(image_height - 1));
  let cut = Rect::new((cut_left, cut_top), (cut_right.saturating_sub(cut_left).max(1), cut_bottom.saturating_sub(cut_top).max(1)));
  let mut part = transform().with_crop(cut).applied(p_image);
  if part.dimensions::<u32>() != (target_width, target_height) {
    transform().with_size(ResizeTarget::Exact(Size::new(target_width as f32, target_height as f32))).apply(&mut part);
  }
  let (part_width, part_height) = part.dimensions::<u32>();
  let part_rgba = part.rgba();
  let (columns, rows) = (part_width.min(width - x) as usize, part_height.min(height - y));
  // Only the colors are copied: the result is opaque, and resizing fades the alpha at the edges of a small cut.
  for row in 0..rows {
    let source = row as usize * part_width as usize * 4;
    let target = ((y + row) as usize * width as usize + x as usize) * 4;
    for column in 0..columns {
      p_pixels[target + column * 4..target + column * 4 + 3].copy_from_slice(&part_rgba[source + column * 4..source + column * 4 + 3]);
    }
  }
}

/// Every pixel of the box looks up the image pixel it shows, smoothly, and keeps the background where that is past the image.
fn view_turned(p_image: &Image, p_window: &ViewWindow, p_size: (u32, u32), p_pixels: &mut [u8]) {
  let (image_width, image_height) = p_image.dimensions::<u32>();
  let (image_width, image_height) = (image_width as f64, image_height as f64);
  let (width, height) = (p_size.0 as f64, p_size.1 as f64);
  let (cos, sin) = (p_window.angle.cos() as f64, p_window.angle.sin() as f64);
  let (scale_x, scale_y) = (p_window.width as f64 / width, p_window.height as f64 / height);
  let (middle_x, middle_y) = (p_window.x as f64 + p_window.width as f64 / 2.0, p_window.y as f64 + p_window.height as f64 / 2.0);
  let looked_up = remap(p_image, p_size.0, p_size.1, Interpolation::Bilinear, EdgeMode::Clamp, |x, y| {
    let (step_x, step_y) = ((x - width / 2.0) * scale_x, (y - height / 2.0) * scale_y);
    let at = (middle_x + cos * step_x + sin * step_y, middle_y - sin * step_x + cos * step_y);
    (at.0 >= 0.0 && at.1 >= 0.0 && at.0 < image_width && at.1 < image_height).then_some(at)
  });
  for (pixel, found) in p_pixels.chunks_exact_mut(4).zip(looked_up.chunks_exact(4)) {
    // Remap leaves a pixel clear when the map gave it no place in the image.
    if found[3] > 0 {
      pixel[..3].copy_from_slice(&found[..3]);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// An image whose red is the column and green the row, times 40, so every pixel tells where it came from.
  fn marked(p_width: u32, p_height: u32) -> Image {
    let pixels = (0..p_height).flat_map(|row| (0..p_width).flat_map(move |column| [(column * 40) as u8, (row * 40) as u8, 0, 255])).collect::<Vec<u8>>();
    Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA)
  }

  fn at(p_image: &Image, p_x: u32, p_y: u32) -> [u8; 4] {
    let (width, _) = p_image.dimensions::<u32>();
    let start = ((p_y * width + p_x) * 4) as usize;
    p_image.rgba()[start..start + 4].try_into().unwrap()
  }

  #[test]
  fn a_window_over_the_whole_image_at_its_own_size_shows_it_as_it_is() {
    let image = marked(4, 4);
    let seen = view(&image, ViewWindow::new(0.0, 0.0, 4.0, 4.0), (4, 4), Color::black());
    assert_eq!(seen.rgba(), image.rgba());
  }

  #[test]
  fn a_window_over_part_of_the_image_shows_that_part_filling_the_box() {
    let image = marked(4, 4);
    // The right half: columns 2 and 3, shown at the same size, so each is one pixel.
    let seen = view(&image, ViewWindow::new(2.0, 0.0, 2.0, 4.0), (2, 4), Color::black());
    assert_eq!(at(&seen, 0, 0), [80, 0, 0, 255]);
    assert_eq!(at(&seen, 1, 3), [120, 120, 0, 255]);
  }

  #[test]
  fn where_the_window_reaches_past_the_image_the_background_shows() {
    let image = marked(4, 4);
    // Twice the image's width with the image on the right: the left half of the box is past the image.
    let seen = view(&image, ViewWindow::new(-4.0, 0.0, 8.0, 4.0), (8, 4), Color::from_rgb(9, 8, 7));
    assert_eq!(at(&seen, 0, 2), [9, 8, 7, 255]);
    assert_eq!(at(&seen, 3, 0), [9, 8, 7, 255]);
    assert_eq!(at(&seen, 4, 0), [0, 0, 0, 255], "the image's own first pixel");
    assert_eq!(at(&seen, 7, 3), [120, 120, 0, 255]);
  }

  #[test]
  fn the_result_is_opaque_whatever_the_image_is() {
    let clear = Image::new_from_color(3, 3, Color::from_rgba(200, 100, 50, 0));
    let seen = view(&clear, ViewWindow::new(0.0, 0.0, 3.0, 3.0), (3, 3), Color::black());
    assert!(seen.rgba().chunks_exact(4).all(|pixel| pixel[3] == 255));
    let turned = view(&clear, ViewWindow::new(0.0, 0.0, 3.0, 3.0).with_angle(0.5), (3, 3), Color::black());
    assert!(turned.rgba().chunks_exact(4).all(|pixel| pixel[3] == 255));
  }

  #[test]
  fn a_quarter_turn_clockwise_puts_the_top_left_corner_at_the_top_right() {
    let image = marked(4, 4);
    let seen = view(&image, ViewWindow::new(0.0, 0.0, 4.0, 4.0).with_angle(std::f32::consts::FRAC_PI_2), (4, 4), Color::black());
    assert_eq!(at(&seen, 3, 0), at(&image, 0, 0));
    assert_eq!(at(&seen, 3, 3), at(&image, 3, 0), "the top right corner goes to the bottom right");
    assert_eq!(at(&seen, 0, 0), at(&image, 0, 3), "and the bottom left goes to the top left");
  }

  #[test]
  fn a_turned_window_shows_the_background_where_the_turn_leaves_the_image() {
    let image = marked(8, 4);
    // A wide image turned a quarter in a box of its own shape: the corners of the box are past the image.
    let seen = view(&image, ViewWindow::new(0.0, 0.0, 8.0, 4.0).with_angle(std::f32::consts::FRAC_PI_2), (8, 4), Color::from_rgb(1, 2, 3));
    assert_eq!(at(&seen, 0, 0), [1, 2, 3, 255]);
    assert_ne!(at(&seen, 4, 2), [1, 2, 3, 255], "the middle is still the image");
  }

  #[test]
  fn nothing_to_show_gives_a_box_of_the_background() {
    let image = marked(4, 4);
    let seen = view(&image, ViewWindow::new(10.0, 10.0, 2.0, 2.0), (3, 2), Color::from_rgb(5, 5, 5));
    assert_eq!(seen.dimensions::<u32>(), (3, 2));
    assert!(seen.rgba().chunks_exact(4).all(|pixel| pixel == [5, 5, 5, 255]));
    assert_eq!(view(&image, ViewWindow::new(0.0, 0.0, 4.0, 4.0), (0, 0), Color::black()).dimensions::<u32>(), (0, 0));
  }
}
