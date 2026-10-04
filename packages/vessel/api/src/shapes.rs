//! Shapes and text for a [`Canvas`]: rectangles with rounded corners, borders, and small bitmap text.
//!
//! Colors are RGBA bytes and every shape is blended over what is already there by its alpha, so a translucent color lets
//! what is underneath show through. Edges are smoothed (anti-aliased) by how much of each pixel the shape covers.

use font8x8::legacy::BASIC_LEGACY;

use crate::Canvas;

/// The width and height in pixels of one letter of the built-in font, before scaling.
const GLYPH: u32 = 8;

/// Blends `p_rgba` over the pixel at (`p_x`, `p_y`) of a picture of `p_size`, as much as `p_coverage` (0 to 1) of the pixel
/// is covered. Pixels outside the picture are ignored.
fn blend(p_pixels: &mut [u8], p_size: (u32, u32), p_x: i64, p_y: i64, p_rgba: [u8; 4], p_coverage: f32) {
  let (width, height) = p_size;
  if p_x < 0 || p_y < 0 || p_x >= width as i64 || p_y >= height as i64 {
    return;
  }
  let alpha = p_rgba[3] as f32 / 255.0 * p_coverage.clamp(0.0, 1.0);
  if alpha <= 0.0 {
    return;
  }
  let at = (p_y as usize * width as usize + p_x as usize) * 4;
  let pixel = &mut p_pixels[at..at + 4];
  for channel in 0..3 {
    pixel[channel] = (p_rgba[channel] as f32 * alpha + pixel[channel] as f32 * (1.0 - alpha)).round() as u8;
  }
  pixel[3] = ((alpha + pixel[3] as f32 / 255.0 * (1.0 - alpha)) * 255.0).round() as u8;
}

/// How far the point (`p_x`, `p_y`) is from the edge of a rounded rectangle, in pixels: negative inside, positive outside.
/// The pixels are measured from their centers.
fn rounded_rect_distance(p_x: f32, p_y: f32, p_rect: (f32, f32, f32, f32), p_radius: f32) -> f32 {
  let (x, y, width, height) = p_rect;
  let (half_width, half_height) = (width / 2.0, height / 2.0);
  let radius = p_radius.clamp(0.0, half_width.min(half_height));
  let (from_center_x, from_center_y) = ((p_x - x - half_width).abs(), (p_y - y - half_height).abs());
  let (beyond_x, beyond_y) = (from_center_x - (half_width - radius), from_center_y - (half_height - radius));
  beyond_x.max(0.0).hypot(beyond_y.max(0.0)) + beyond_x.max(beyond_y).min(0.0) - radius
}

/// Visits every pixel of a picture of `p_size` that a rounded rectangle can touch, with how far the pixel is from its edge.
fn for_rounded_rect(
  p_size: (u32, u32), p_rect: (i32, i32, u32, u32), p_radius: f32, mut p_paint: impl FnMut(i64, i64, f32),
) {
  let (x, y, width, height) = p_rect;
  // One extra pixel each way, for the smoothed edge.
  let (left, top) = ((x as i64 - 1).max(0), (y as i64 - 1).max(0));
  let (right, bottom) =
    ((x as i64 + width as i64 + 1).min(p_size.0 as i64), (y as i64 + height as i64 + 1).min(p_size.1 as i64));
  let rect = (x as f32, y as f32, width as f32, height as f32);
  for row in top..bottom {
    for column in left..right {
      p_paint(column, row, rounded_rect_distance(column as f32 + 0.5, row as f32 + 0.5, rect, p_radius));
    }
  }
}

impl Canvas {
  /// Fills the rectangle with its top left corner at (`p_x`, `p_y`) with `p_rgba`, rounding the corners by `p_radius`
  /// pixels (0 for square corners; half the shorter side makes a pill or, on a square, a circle).
  pub fn fill_rounded_rect(&self, p_x: i32, p_y: i32, p_width: u32, p_height: u32, p_radius: f32, p_rgba: [u8; 4]) {
    let mut pixels = self.pixels.lock().unwrap();
    if !self.intact(&pixels) {
      return;
    }
    let size = (self.width, self.height);
    for_rounded_rect(size, (p_x, p_y, p_width, p_height), p_radius, |column, row, distance| {
      blend(&mut pixels, size, column, row, p_rgba, 0.5 - distance);
    });
  }

  /// Draws a border `p_thickness` pixels wide just inside the edge of the rectangle, with the same corner rounding as
  /// [`fill_rounded_rect`](Self::fill_rounded_rect). The inside is left as it is.
  pub fn stroke_rounded_rect(
    &self, p_x: i32, p_y: i32, p_width: u32, p_height: u32, p_radius: f32, p_thickness: f32, p_rgba: [u8; 4],
  ) {
    let mut pixels = self.pixels.lock().unwrap();
    if !self.intact(&pixels) {
      return;
    }
    let size = (self.width, self.height);
    for_rounded_rect(size, (p_x, p_y, p_width, p_height), p_radius, |column, row, distance| {
      let outside = (0.5 - distance).clamp(0.0, 1.0);
      let inside = (0.5 - (distance + p_thickness)).clamp(0.0, 1.0);
      blend(&mut pixels, size, column, row, p_rgba, outside - inside);
    });
  }

  /// The size in pixels that `p_text` takes when drawn with [`draw_text`](Self::draw_text) in the canvas's font, for
  /// placing it.
  pub fn text_size(&self, p_text: &str) -> (u32, u32) {
    if let Some(font) = self.font.face.outline() {
      let px = self.font.size as f32;
      let width: f32 = p_text.chars().map(|letter| font.metrics(letter, px).advance_width).sum();
      let height = font.horizontal_line_metrics(px).map_or(px, |line| line.ascent - line.descent);
      return (width.ceil() as u32, if p_text.is_empty() { 0 } else { height.ceil() as u32 });
    }
    let scale = self.font.bitmap_scale();
    let letters = p_text.chars().count() as u32;
    (letters * GLYPH * scale, if letters == 0 { 0 } else { GLYPH * scale })
  }

  /// Draws `p_text` with its top left corner at (`p_x`, `p_y`) in the canvas's font. The built-in bitmap face covers plain
  /// ASCII only (other characters show as blanks); an outline face draws whatever its file has.
  pub fn draw_text(&self, p_x: i32, p_y: i32, p_text: &str, p_rgba: [u8; 4]) {
    let mut pixels = self.pixels.lock().unwrap();
    if !self.intact(&pixels) {
      return;
    }
    let size = (self.width, self.height);
    if let Some(font) = self.font.face.outline() {
      let px = self.font.size as f32;
      let ascent = font.horizontal_line_metrics(px).map_or(px, |line| line.ascent);
      let baseline = p_y as f32 + ascent;
      let mut pen = p_x as f32;
      for letter in p_text.chars() {
        let (metrics, coverage) = font.rasterize(letter, px);
        let left = (pen + metrics.xmin as f32).round() as i64;
        let top = (baseline - metrics.height as f32 - metrics.ymin as f32).round() as i64;
        for (index, covered) in coverage.iter().enumerate().filter(|(_, covered)| **covered > 0) {
          let (column, row) = ((index % metrics.width) as i64, (index / metrics.width) as i64);
          blend(&mut pixels, size, left + column, top + row, p_rgba, *covered as f32 / 255.0);
        }
        pen += metrics.advance_width;
      }
      return;
    }

    let scale = self.font.bitmap_scale() as i64;
    for (position, letter) in p_text.chars().enumerate() {
      let glyph = BASIC_LEGACY.get(letter as usize).copied().unwrap_or([0; 8]);
      let left = p_x as i64 + position as i64 * GLYPH as i64 * scale;
      for (row, bits) in glyph.iter().enumerate() {
        for column in 0..GLYPH as i64 {
          // The lowest bit of a row is its leftmost pixel.
          if bits >> column & 1 == 0 {
            continue;
          }
          for dy in 0..scale {
            for dx in 0..scale {
              blend(&mut pixels, size, left + column * scale + dx, p_y as i64 + row as i64 * scale + dy, p_rgba, 1.0);
            }
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{Face, Font};

  fn pixel(p_canvas: &Canvas, p_x: u32, p_y: u32) -> [u8; 4] {
    let at = ((p_y * p_canvas.width() + p_x) * 4) as usize;
    p_canvas.pixels()[at..at + 4].try_into().unwrap()
  }

  #[test]
  fn a_square_rectangle_fills_exactly_its_pixels_and_clips_at_the_edges() {
    let canvas = Canvas::new(6, 4);
    canvas.fill_rounded_rect(1, 1, 3, 2, 0.0, [200, 0, 0, 255]);
    assert_eq!(pixel(&canvas, 1, 1), [200, 0, 0, 255]);
    assert_eq!(pixel(&canvas, 3, 2), [200, 0, 0, 255]);
    assert_eq!(pixel(&canvas, 0, 1), [0, 0, 0, 0]);
    assert_eq!(pixel(&canvas, 4, 1), [0, 0, 0, 0]);
    assert_eq!(pixel(&canvas, 1, 3), [0, 0, 0, 0]);

    // Partly or wholly outside the canvas is not an error.
    canvas.fill_rounded_rect(-5, -5, 7, 7, 3.0, [1, 1, 1, 255]);
    canvas.fill_rounded_rect(100, 100, 5, 5, 0.0, [1, 1, 1, 255]);
  }

  #[test]
  fn rounded_corners_are_cut_away_and_smoothed() {
    let canvas = Canvas::new(20, 20);
    canvas.fill_rounded_rect(0, 0, 20, 20, 8.0, [255, 255, 255, 255]);
    assert_eq!(pixel(&canvas, 10, 10), [255, 255, 255, 255], "the middle is solid");
    assert_eq!(pixel(&canvas, 0, 0)[3], 0, "the corner is cut away");
    assert_eq!(pixel(&canvas, 10, 0), [255, 255, 255, 255], "the straight edge is solid");
    let edge = pixel(&canvas, 2, 2)[3];
    assert!(edge > 0 && edge < 255, "the curve is smoothed, got alpha {edge}");
  }

  #[test]
  fn a_radius_of_half_the_size_makes_a_circle() {
    let canvas = Canvas::new(10, 10);
    canvas.fill_rounded_rect(0, 0, 10, 10, 5.0, [9, 9, 9, 255]);
    assert_eq!(pixel(&canvas, 5, 5)[3], 255);
    assert_eq!(pixel(&canvas, 0, 0)[3], 0);
    assert_eq!(pixel(&canvas, 9, 9)[3], 0);
  }

  #[test]
  fn translucent_colors_blend_over_what_is_underneath() {
    let canvas = Canvas::new(2, 1);
    canvas.fill([100, 100, 100, 255]);
    canvas.fill_rounded_rect(0, 0, 1, 1, 0.0, [200, 200, 200, 128]);
    let blended = pixel(&canvas, 0, 0);
    assert!(blended[0].abs_diff(150) <= 1, "{blended:?}");
    assert_eq!(blended[3], 255);
    assert_eq!(pixel(&canvas, 1, 0), [100, 100, 100, 255]);
  }

  #[test]
  fn a_border_is_drawn_inside_the_edge_and_leaves_the_middle() {
    let canvas = Canvas::new(10, 10);
    canvas.stroke_rounded_rect(0, 0, 10, 10, 0.0, 2.0, [0, 0, 255, 255]);
    assert_eq!(pixel(&canvas, 0, 5), [0, 0, 255, 255]);
    assert_eq!(pixel(&canvas, 1, 5), [0, 0, 255, 255]);
    assert_eq!(pixel(&canvas, 2, 5)[3], 0, "past the border width is untouched");
    assert_eq!(pixel(&canvas, 5, 5)[3], 0);
    assert_eq!(pixel(&canvas, 5, 9), [0, 0, 255, 255]);
  }

  #[test]
  fn text_has_a_size_that_follows_its_length_and_scale() {
    let mut canvas = Canvas::new(1, 1);
    canvas.set_font(Font::new(Face::bitmap(), 8));
    assert_eq!(canvas.text_size("abc"), (24, 8));
    canvas.set_font(Font::new(Face::bitmap(), 16));
    assert_eq!(canvas.text_size("abc"), (48, 16));
    canvas.set_font(Font::new(Face::bitmap(), 24));
    assert_eq!(canvas.text_size(""), (0, 0));
  }

  #[test]
  fn text_draws_the_letters_shape_at_a_scale() {
    // 'I' is a vertical bar with serifs: its top row is lit and its first column is not.
    let mut canvas = Canvas::new(32, 16);
    canvas.set_font(Font::new(Face::bitmap(), 8));
    canvas.draw_text(0, 0, "I", [255, 255, 255, 255]);
    let lit: usize = canvas.pixels().chunks_exact(4).filter(|pixel| pixel[3] == 255).count();
    assert!(lit > 8 && lit < 40, "a letter lights some of its 64 pixels, got {lit}");

    let mut big = Canvas::new(32, 16);
    big.set_font(Font::new(Face::bitmap(), 16));
    big.draw_text(0, 0, "I", [255, 255, 255, 255]);
    let big_lit = big.pixels().chunks_exact(4).filter(|pixel| pixel[3] == 255).count();
    assert_eq!(big_lit, lit * 4, "doubling the scale quadruples the pixels");
  }

  #[test]
  fn text_goes_where_it_is_asked_and_clips_at_the_edges() {
    let canvas = Canvas::new(8, 8);
    canvas.draw_text(100, 100, "Hi", [255, 255, 255, 255]);
    canvas.draw_text(-4, -4, "Hi", [255, 255, 255, 255]);
    canvas.draw_text(0, 0, "é", [255, 255, 255, 255]); // not ASCII: drawn as nothing
    let other = Canvas::new(8, 8);
    other.draw_text(-4, -4, "Hi", [255, 255, 255, 255]);
    assert_eq!(canvas, other);
  }

  #[test]
  fn an_outline_face_has_letters_of_different_widths_and_smooth_edges() {
    let face = Face::system();
    if !face.is_outline() {
      return; // this machine has none of the usual system fonts
    }
    let mut canvas = Canvas::new(120, 40);
    canvas.set_font(Font::new(face, 24));
    let (i_width, height) = canvas.text_size("i");
    let (w_width, _) = canvas.text_size("W");
    assert!(i_width < w_width, "i is narrower than W: {i_width} vs {w_width}");
    assert!((20..=40).contains(&height), "a 24 pixel line is about that tall: {height}");
    assert_eq!(canvas.text_size(""), (0, 0));

    canvas.draw_text(2, 2, "Hello", [255, 255, 255, 255]);
    let alphas: Vec<u8> = canvas.pixels().chunks_exact(4).map(|pixel| pixel[3]).filter(|alpha| *alpha > 0).collect();
    assert!(alphas.iter().any(|alpha| *alpha == 255), "the letters have solid pixels");
    assert!(alphas.iter().any(|alpha| *alpha > 0 && *alpha < 255), "and smoothed edges");
    let lowest =
      canvas.pixels().chunks_exact(4 * 120).rposition(|row| row.chunks_exact(4).any(|pixel| pixel[3] > 0)).unwrap();
    assert!(lowest < 2 + height as usize, "the text stays inside its line box");
  }
}
