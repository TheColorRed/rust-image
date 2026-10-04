use std::sync::{Arc, Mutex};

use crate::{Font, Theme};

/// A rectangle of RGBA pixels that a component draws on.
///
/// It can fill, compute colors per pixel, blend pictures in, and (see the `shapes` module) draw rounded rectangles,
/// borders and small bitmap text.
///
/// A canvas is a cheap handle to shared pixels, so it is the event a component's `draw()` stream sends: every listener
/// gets the same canvas and draws on it, one after another, and its methods take `&self`.
#[derive(Clone, Debug)]
pub struct Canvas {
  pub(crate) width: u32,
  pub(crate) height: u32,
  pub(crate) pixels: Arc<Mutex<Vec<u8>>>,
  pub(crate) font: Font,
  pub(crate) theme: Theme,
  pub(crate) radius: f32,
  pub(crate) content: (i32, i32, u32, u32),
}

impl PartialEq for Canvas {
  fn eq(&self, p_other: &Self) -> bool {
    self.width == p_other.width && self.height == p_other.height && self.pixels() == p_other.pixels()
  }
}

impl Canvas {
  /// A canvas of `p_width` x `p_height` transparent black pixels, with the default font.
  pub fn new(p_width: u32, p_height: u32) -> Self {
    Self {
      width: p_width,
      height: p_height,
      pixels: Arc::new(Mutex::new(vec![0; p_width as usize * p_height as usize * 4])),
      font: Font::default(),
      theme: Theme::default(),
      radius: 0.0,
      content: (0, 0, p_width, p_height),
    }
  }

  /// The width in pixels.
  pub fn width(&self) -> u32 {
    self.width
  }

  /// The height in pixels.
  pub fn height(&self) -> u32 {
    self.height
  }

  /// The font text is drawn in: the one the component being drawn inherited, or its own.
  pub fn font(&self) -> Font {
    self.font.clone()
  }

  /// How much the component being drawn has its corners rounded, so what it draws can follow its shape.
  pub fn radius(&self) -> f32 {
    self.radius
  }

  /// The area inside the component's border and padding as (left, top, width, height): where its content belongs.
  pub fn content_rect(&self) -> (i32, i32, u32, u32) {
    self.content
  }

  /// Sets the corner rounding and the content area; the component being drawn has them set already.
  pub fn set_box(&mut self, p_radius: f32, p_content: (i32, i32, u32, u32)) {
    self.radius = p_radius;
    self.content = p_content;
  }

  /// The theme the component being drawn inherited, for the colors and shapes it should look like.
  pub fn theme(&self) -> Theme {
    self.theme.clone()
  }

  /// Sets the theme.
  pub fn set_theme(&mut self, p_theme: Theme) {
    self.theme = p_theme;
  }

  /// Sets the font text is drawn in.
  pub fn set_font(&mut self, p_font: Font) {
    self.font = p_font;
  }

  /// A copy of the pixels, `width * height` of them, as RGBA bytes row by row.
  pub fn pixels(&self) -> Vec<u8> {
    self.pixels.lock().unwrap().clone()
  }

  /// Takes the pixels out of the canvas, for the picture that is finished. Other handles to it are left empty and draw
  /// nothing.
  pub fn into_pixels(self) -> Vec<u8> {
    std::mem::take(&mut *self.pixels.lock().unwrap())
  }

  /// Whether the pixels are all there (they are not once taken with [`into_pixels`](Self::into_pixels)).
  pub(crate) fn intact(&self, p_pixels: &[u8]) -> bool {
    p_pixels.len() == self.width as usize * self.height as usize * 4
  }

  /// Sets every pixel to `p_rgba`.
  pub fn fill(&self, p_rgba: [u8; 4]) {
    for pixel in self.pixels.lock().unwrap().chunks_exact_mut(4) {
      pixel.copy_from_slice(&p_rgba);
    }
  }

  /// Draws RGBA pixels with their top left corner at (`p_x`, `p_y`), blending each over what is already there by its
  /// alpha and clipping at the canvas edges. This is how a parent puts a child's picture into its own.
  /// - `p_rgba`: `p_width * p_height` RGBA pixels.
  pub fn draw_pixels(&self, p_x: i32, p_y: i32, p_width: u32, p_height: u32, p_rgba: &[u8]) {
    if p_rgba.len() < p_width as usize * p_height as usize * 4 {
      return;
    }
    let mut pixels = self.pixels.lock().unwrap();
    if !self.intact(&pixels) {
      return;
    }
    // The part of the source that lands inside the canvas.
    let first_column = (-p_x).max(0) as usize;
    let last_column = ((self.width as i64 - p_x as i64).clamp(0, p_width as i64)) as usize;
    if first_column >= last_column {
      return;
    }
    for row in 0..p_height as usize {
      let destination_row = p_y as i64 + row as i64;
      if destination_row < 0 || destination_row >= self.height as i64 {
        continue;
      }
      let source = &p_rgba[(row * p_width as usize + first_column) * 4..(row * p_width as usize + last_column) * 4];
      let start = (destination_row as usize * self.width as usize + (p_x as i64 + first_column as i64) as usize) * 4;
      let destination = &mut pixels[start..start + source.len()];
      for (from, to) in source.chunks_exact(4).zip(destination.chunks_exact_mut(4)) {
        match from[3] {
          0 => {}
          255 => to.copy_from_slice(from),
          alpha => {
            let alpha = alpha as u32;
            for channel in 0..3 {
              to[channel] = ((from[channel] as u32 * alpha + to[channel] as u32 * (255 - alpha)) / 255) as u8;
            }
            to[3] = (alpha + to[3] as u32 * (255 - alpha) / 255) as u8;
          }
        }
      }
    }
  }

  /// Sets every pixel to the opaque color `p_color` returns for it. `p_color` gets the pixel's position as fractions of
  /// the canvas, from 0 to 1 across and down, and returns red, green and blue from 0 to 1 (values outside are clamped).
  pub fn fill_with(&self, p_color: impl Fn(f32, f32) -> [f32; 3]) {
    let (width, height) = (self.width.max(1) as f32, self.height.max(1) as f32);
    for (index, pixel) in self.pixels.lock().unwrap().chunks_exact_mut(4).enumerate() {
      let (x, y) =
        ((index as u32 % self.width.max(1)) as f32 / width, (index as u32 / self.width.max(1)) as f32 / height);
      let [red, green, blue] = p_color(x, y);
      pixel.copy_from_slice(&[to_byte(red), to_byte(green), to_byte(blue), 255]);
    }
  }
}

fn to_byte(p_unit: f32) -> u8 {
  (p_unit.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_new_canvas_is_transparent_and_the_right_size() {
    let canvas = Canvas::new(3, 2);
    assert_eq!((canvas.width(), canvas.height()), (3, 2));
    assert_eq!(canvas.pixels().len(), 3 * 2 * 4);
    assert!(canvas.pixels().iter().all(|&byte| byte == 0));
  }

  #[test]
  fn fill_sets_every_pixel() {
    let canvas = Canvas::new(2, 2);
    canvas.fill([1, 2, 3, 255]);
    assert!(canvas.pixels().chunks_exact(4).all(|pixel| pixel == [1, 2, 3, 255]));
  }

  #[test]
  fn fill_with_gets_each_pixels_place_as_a_fraction() {
    let canvas = Canvas::new(2, 1);
    canvas.fill_with(|x, _| [x * 2.0, 0.0, 0.0]);
    // x is 0.0 for the first pixel and 0.5 for the second, so red is 0 and 255 (1.0 after the multiplication).
    assert_eq!(&canvas.pixels()[..8], &[0, 0, 0, 255, 255, 0, 0, 255]);
  }

  #[test]
  fn fill_with_clamps_colors_outside_zero_to_one() {
    let canvas = Canvas::new(1, 1);
    canvas.fill_with(|_, _| [2.0, -1.0, 0.5]);
    assert_eq!(canvas.pixels(), &[255, 0, 128, 255]);
  }

  #[test]
  fn draw_pixels_copies_opaque_pixels_at_a_position() {
    let canvas = Canvas::new(3, 2);
    canvas.draw_pixels(1, 1, 2, 1, &[9, 8, 7, 255, 6, 5, 4, 255]);
    assert_eq!(
      canvas.pixels(),
      &[
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 8, 7, 255, 6, 5, 4, 255
      ]
    );
  }

  #[test]
  fn draw_pixels_clips_at_every_edge() {
    let canvas = Canvas::new(2, 2);
    let red = [255, 0, 0, 255].repeat(4); // a 2x2 source
    canvas.draw_pixels(-1, -1, 2, 2, &red); // only its bottom right pixel is inside
    assert_eq!(&canvas.pixels()[..4], &[255, 0, 0, 255]);
    assert_eq!(&canvas.pixels()[4..], &[0; 12]);
    let canvas = Canvas::new(2, 2);
    canvas.draw_pixels(1, 1, 2, 2, &red); // only its top left pixel is inside
    assert_eq!(&canvas.pixels()[12..], &[255, 0, 0, 255]);
    assert_eq!(&canvas.pixels()[..12], &[0; 12]);
    canvas.draw_pixels(5, 5, 2, 2, &red); // wholly outside
    canvas.draw_pixels(-5, 0, 2, 2, &red);
  }

  #[test]
  fn draw_pixels_blends_by_alpha_and_skips_transparent_pixels() {
    let canvas = Canvas::new(3, 1);
    canvas.fill([100, 100, 100, 255]);
    canvas.draw_pixels(0, 0, 3, 1, &[200, 200, 200, 0, 200, 200, 200, 255, 200, 0, 0, 128]);
    assert_eq!(&canvas.pixels()[..4], &[100, 100, 100, 255], "transparent leaves it alone");
    assert_eq!(&canvas.pixels()[4..8], &[200, 200, 200, 255], "opaque replaces it");
    let pixels = canvas.pixels();
    let blended = &pixels[8..12];
    assert!(
      blended[0].abs_diff(150) <= 1 && blended[1].abs_diff(50) <= 1 && blended[2].abs_diff(50) <= 1,
      "{blended:?}"
    );
    assert_eq!(blended[3], 255);
  }
}
