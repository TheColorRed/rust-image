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
  let at = (p_y as usize * width as usize + p_x as usize) * 4;
  blend_pixel(&mut p_pixels[at..at + 4], p_rgba, p_coverage);
}

fn blend_pixel(p_pixel: &mut [u8], p_rgba: [u8; 4], p_coverage: f32) {
  let coverage = p_coverage.clamp(0.0, 1.0);
  if coverage <= 0.0 {
    return;
  }
  // Fast path: fully covered + fully opaque replaces the destination.
  if p_rgba[3] == 255 && coverage >= 1.0 {
    p_pixel.copy_from_slice(&p_rgba);
    return;
  }
  let alpha = p_rgba[3] as f32 / 255.0 * coverage;
  if alpha <= 0.0 {
    return;
  }
  for channel in 0..3 {
    p_pixel[channel] = (p_rgba[channel] as f32 * alpha + p_pixel[channel] as f32 * (1.0 - alpha)).round() as u8;
  }
  p_pixel[3] = ((alpha + p_pixel[3] as f32 / 255.0 * (1.0 - alpha)) * 255.0).round() as u8;
}

/// Paints a fully covered row, copying opaque runs and preserving the shape painter's alpha rounding elsewhere.
fn blend_span(p_destination: &mut [u8], p_source: &[u8]) {
  let mut index = 0;
  while index < p_source.len() {
    match p_source[index + 3] {
      0 => index += 4,
      255 => {
        let mut end = index + 4;
        while end < p_source.len() && p_source[end + 3] == 255 {
          end += 4;
        }
        p_destination[index..end].copy_from_slice(&p_source[index..end]);
        index = end;
      }
      _ => {
        let color = p_source[index..index + 4].try_into().unwrap();
        blend_pixel(&mut p_destination[index..index + 4], color, 1.0);
        index += 4;
      }
    }
  }
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
  p_size: (u32, u32), p_rect: (i32, i32, u32, u32), p_radius: f32, p_clip: Option<(i32, i32, u32, u32)>,
  mut p_paint: impl FnMut(i64, i64, f32),
) {
  let (x, y, width, height) = p_rect;
  // One extra pixel each way, for the smoothed edge.
  let (mut left, mut top) = ((x as i64 - 1).max(0), (y as i64 - 1).max(0));
  let (mut right, mut bottom) =
    ((x as i64 + width as i64 + 1).min(p_size.0 as i64), (y as i64 + height as i64 + 1).min(p_size.1 as i64));

  if let Some((cx, cy, cw, ch)) = p_clip {
    let clip_left = cx.max(0) as i64;
    let clip_top = cy.max(0) as i64;
    let clip_right = (cx as i64 + cw as i64).min(p_size.0 as i64);
    let clip_bottom = (cy as i64 + ch as i64).min(p_size.1 as i64);
    left = left.max(clip_left);
    top = top.max(clip_top);
    right = right.min(clip_right);
    bottom = bottom.min(clip_bottom);
  }

  if left >= right || top >= bottom {
    return;
  }

  let rect = (x as f32, y as f32, width as f32, height as f32);
  for row in top..bottom {
    for column in left..right {
      p_paint(column, row, rounded_rect_distance(column as f32 + 0.5, row as f32 + 0.5, rect, p_radius));
    }
  }
}

impl Canvas {
  /// Blends an image behind content, clipped to this canvas's box and rounded corners.
  /// The pixels are RGBA bytes, `p_width * p_height * 4` in length.
  pub fn draw_background_pixels(&self, p_x: i32, p_y: i32, p_width: u32, p_height: u32, p_rgba: &[u8]) {
    assert_eq!(
      p_rgba.len(),
      p_width as usize * p_height as usize * 4,
      "background image pixel length must match its size"
    );
    if p_width == 0 || p_height == 0 {
      return;
    }
    let mut pixels = self.pixels.lock().unwrap();
    if !self.intact(&pixels) {
      return;
    }
    let (x, y) = (p_x as i64, p_y as i64);
    let (mut left, mut top) = (x.max(0), y.max(0));
    let (mut right, mut bottom) =
      ((x + p_width as i64).min(self.width as i64), (y + p_height as i64).min(self.height as i64));
    if let Some((cx, cy, cw, ch)) = self.clip {
      left = left.max(cx as i64);
      top = top.max(cy as i64);
      right = right.min(cx as i64 + cw as i64);
      bottom = bottom.min(cy as i64 + ch as i64);
    }
    if left >= right || top >= bottom {
      return;
    }
    let radius = self.radius.clamp(0.0, self.width.min(self.height) as f32 / 2.0);
    let corner = radius.ceil() as i64;
    let rect = (0.0, 0.0, self.width as f32, self.height as f32);
    for row in top..bottom {
      // Away from corner rows the whole row has full coverage. In corner rows the central strip does too.
      let (middle_left, middle_right) = if row >= corner && row < self.height as i64 - corner {
        (left, right)
      } else {
        let start = left.max(corner).min(right);
        (start, right.min(self.width as i64 - corner).max(start))
      };
      let source_start = ((row - y) as usize * p_width as usize + (middle_left - x) as usize) * 4;
      let destination_start = (row as usize * self.width as usize + middle_left as usize) * 4;
      let bytes = (middle_right - middle_left) as usize * 4;
      blend_span(
        &mut pixels[destination_start..destination_start + bytes],
        &p_rgba[source_start..source_start + bytes],
      );
      for column in (left..middle_left).chain(middle_right..right) {
        let at = ((row - y) as usize * p_width as usize + (column - x) as usize) * 4;
        let color = p_rgba[at..at + 4].try_into().unwrap();
        let destination = (row as usize * self.width as usize + column as usize) * 4;
        let distance = rounded_rect_distance(column as f32 + 0.5, row as f32 + 0.5, rect, radius);
        blend_pixel(&mut pixels[destination..destination + 4], color, 0.5 - distance);
      }
    }
  }

  /// Fills the rectangle with its top left corner at (`p_x`, `p_y`) with `p_rgba`, rounding the corners by `p_radius`
  /// pixels (0 for square corners; half the shorter side makes a pill or, on a square, a circle).
  pub fn fill_rounded_rect(&self, p_x: i32, p_y: i32, p_width: u32, p_height: u32, p_radius: f32, p_rgba: [u8; 4]) {
    let mut pixels = self.pixels.lock().unwrap();
    if !self.intact(&pixels) {
      return;
    }
    let size = (self.width, self.height);
    for_rounded_rect(size, (p_x, p_y, p_width, p_height), p_radius, self.clip, |column, row, distance| {
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
    for_rounded_rect(size, (p_x, p_y, p_width, p_height), p_radius, self.clip, |column, row, distance| {
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
    let (clip_left, clip_top, clip_right, clip_bottom) = match self.clip {
      None => (0i64, 0i64, self.width as i64, self.height as i64),
      Some((x, y, w, h)) => {
        let left = x.max(0) as i64;
        let top = y.max(0) as i64;
        let right = (x as i64 + w as i64).min(self.width as i64);
        let bottom = (y as i64 + h as i64).min(self.height as i64);
        (left, top, right, bottom)
      }
    };
    if clip_left >= clip_right || clip_top >= clip_bottom {
      return;
    }

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
          let x = left + column;
          let y = top + row;
          if x >= clip_left && y >= clip_top && x < clip_right && y < clip_bottom {
            blend(&mut pixels, size, x, y, p_rgba, *covered as f32 / 255.0);
          }
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
              let x = left + column * scale + dx;
              let y = p_y as i64 + row as i64 * scale + dy;
              if x >= clip_left && y >= clip_top && x < clip_right && y < clip_bottom {
                blend(&mut pixels, size, x, y, p_rgba, 1.0);
              }
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

  #[test]
  #[ignore = "manual rendering benchmark: run with --ignored --nocapture"]
  fn background_paint_benchmark() {
    use std::{hint::black_box, time::Instant};

    let (width, height) = (512, 384);
    let canvas = Canvas::new(width, height);
    let source = [190, 120, 70, 255].repeat((width * height) as usize);
    let radius = 20.0;
    let iterations = 100;
    let mut fast = canvas.clone();
    fast.set_box(radius, (0, 0, width, height));
    let start = Instant::now();
    for _ in 0..iterations {
      fast.draw_background_pixels(0, 0, width, height, black_box(&source));
    }
    let optimized = start.elapsed();
    black_box(fast.pixels());
    let start = Instant::now();
    for _ in 0..iterations {
      let source = black_box(&source);
      let mut pixels = canvas.pixels.lock().unwrap();
      for_rounded_rect((width, height), (0, 0, width, height), radius, None, |column, row, distance| {
        let at = (row as usize * width as usize + column as usize) * 4;
        blend(&mut pixels, (width, height), column, row, source[at..at + 4].try_into().unwrap(), 0.5 - distance);
      });
    }
    let per_pixel = start.elapsed();
    black_box(canvas.pixels());
    eprintln!(
      "{iterations} opaque {width}x{height} backgrounds: rows={optimized:?}, per-pixel={per_pixel:?}, speedup={:.2}x",
      per_pixel.as_secs_f64() / optimized.as_secs_f64()
    );
  }

  #[test]
  fn background_row_paths_match_per_pixel_painting_with_clips_alpha_and_fractional_corners() {
    for (width, height) in [(1, 1), (7, 5), (24, 17)] {
      let source: Vec<u8> = (0..width * height)
        .flat_map(|index| {
          [
            (index * 19) as u8,
            (index * 31) as u8,
            (index * 7) as u8,
            [0, 1, 127, 254, 255, 255, 255][index as usize % 7],
          ]
        })
        .collect();
      for radius in [0.0, 0.25, 0.75, 1.5, 3.125, 12.0, 100.0] {
        for clip in [
          None,
          Some((-2, 1, 7, 4)),
          Some((3, 2, 11, 8)),
          Some((100, 100, 2, 2)),
          Some((0, 0, 0, 0)),
        ] {
          for (x, y) in [(0, 0), (-3, -1), (2, 1), (100, 100)] {
            let mut actual = Canvas::new(width, height);
            let mut expected = Canvas::new(width, height);
            actual.fill([91, 53, 17, 113]);
            expected.fill([91, 53, 17, 113]);
            actual.set_box(radius, (0, 0, width, height));
            actual.set_clip(clip);
            expected.set_clip(clip);
            actual.draw_background_pixels(x, y, width, height, &source);
            {
              let mut pixels = expected.pixels.lock().unwrap();
              for_rounded_rect((width, height), (0, 0, width, height), radius, clip, |column, row, distance| {
                let (sx, sy) = (column - x as i64, row - y as i64);
                if sx >= 0 && sy >= 0 && sx < width as i64 && sy < height as i64 {
                  let at = (sy as usize * width as usize + sx as usize) * 4;
                  blend(
                    &mut pixels,
                    (width, height),
                    column,
                    row,
                    source[at..at + 4].try_into().unwrap(),
                    0.5 - distance,
                  );
                }
              });
            }
            assert_eq!(
              actual.pixels(),
              expected.pixels(),
              "size={width}x{height}, radius={radius}, clip={clip:?}, at={x},{y}"
            );
          }
        }
      }
    }
  }

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
