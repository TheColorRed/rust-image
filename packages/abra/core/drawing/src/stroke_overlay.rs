use crate::Brush;
use abra_core::image::gpu::{GpuAux, GpuPass};
use abra_core::image::recipe::{Generator, generated};
use abra_core::{Channels, Fill, Image};
use rayon::prelude::*;
use std::sync::Arc;

/// Most points one stroke is drawn from. A line with more is thinned evenly (keeping both ends) before it is drawn, so a long
/// stroke still fits in the shader's uniform buffer and costs a bounded amount per pixel.
pub const MAX_STROKE_POINTS: usize = 256;

/// What a brush looks like as a line, in the terms a shader draws it: how far from the line it reaches, how wide its fade is and its
/// color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeLook {
  /// Half the brush's size, in pixels.
  pub radius: f32,
  /// How wide the fade at the edge is, in pixels: the part of the radius that the brush's softness covers, and at least one so the
  /// edge is never jagged.
  pub edge: f32,
  /// The color, straight (not multiplied by its alpha), each from 0 to 1, the last being the color's alpha times the brush's opacity at the
  /// start of a stroke, since a renderer that draws a line a piece at a time in one style has one opacity. A brush that is not a solid
  /// color is black.
  pub color: [f32; 4],
}

/// The color of a brush, straight, each from 0 to 1, with its own alpha and not its opacity. A brush that is not a solid color is black.
fn solid_color(p_brush: &Brush) -> [f32; 4] {
  let color = match p_brush.color() {
    Fill::Solid(color) => color.rgba(),
    _ => (0, 0, 0, 255),
  };
  [color.0 as f32 / 255.0, color.1 as f32 / 255.0, color.2 as f32 / 255.0, color.3 as f32 / 255.0]
}

impl StrokeLook {
  /// How `p_brush` draws a line in one style: its size, and its opacity at the start of the stroke. A width or an opacity that changes along
  /// the line needs a renderer that reads it along the line, as [`stroke_overlay`] does.
  pub fn of(p_brush: &Brush) -> StrokeLook {
    let radius = p_brush.size() as f32 / 2.0;
    let mut color = solid_color(p_brush);
    color[3] *= p_brush.opacity().opacity_at(0.0);
    StrokeLook {
      radius,
      edge: ((1.0 - p_brush.hardness()) * radius).max(1.0),
      color,
    }
  }
}

/// How many cells the grid that finds the pieces of the line near a pixel has along its longer side, at most.
const GRID_CELLS: usize = 16;
/// How many pieces of the line one cell lists. A cell that would list more is marked to check every piece instead.
const CELL_SEGMENTS: usize = 63;
/// The count that marks a cell that checks every piece of the line.
const ALL_SEGMENTS: u8 = 255;
/// The bytes of a cell in the grid: its count, then the places of its pieces. A piece's place fits a byte, since a line of at most
/// [`MAX_STROKE_POINTS`] points has fewer than 255 pieces.
const CELL_BYTES: usize = 1 + CELL_SEGMENTS;

/// A grid laid over the box around the line, where each cell lists the pieces of the line that reach into it. A pixel then only tests the
/// pieces in its cell and not the whole line, so what a pixel costs does not grow with the length of the line. The shader reads the same
/// lists as the CPU, from a texture.
#[derive(Debug)]
struct Grid {
  /// The top left corner of the grid, in pixels.
  origin: (f32, f32),
  /// How wide and how tall each cell is, in pixels.
  cell: f32,
  cols: usize,
  rows: usize,
  /// `CELL_BYTES` for each cell, row by row: how many pieces it lists (or `ALL_SEGMENTS`), then their places.
  bytes: Vec<u8>,
}

impl Grid {
  /// The grid for a line through `p_points`, whose pieces each reach as far as the larger of `p_reaches` at their ends.
  fn new(p_points: &[(f32, f32)], p_reaches: &[f32]) -> Grid {
    if p_points.is_empty() {
      return Grid { origin: (0.0, 0.0), cell: 1.0, cols: 1, rows: 1, bytes: vec![0; CELL_BYTES] };
    }
    let widest = p_reaches.iter().copied().fold(0.0, f32::max);
    let left = p_points.iter().map(|point| point.0).fold(f32::MAX, f32::min) - widest;
    let right = p_points.iter().map(|point| point.0).fold(f32::MIN, f32::max) + widest;
    let top = p_points.iter().map(|point| point.1).fold(f32::MAX, f32::min) - widest;
    let bottom = p_points.iter().map(|point| point.1).fold(f32::MIN, f32::max) + widest;
    let cell = ((right - left).max(bottom - top) / GRID_CELLS as f32).max(1.0);
    let cols = (((right - left) / cell).ceil() as usize).clamp(1, GRID_CELLS);
    let rows = (((bottom - top) / cell).ceil() as usize).clamp(1, GRID_CELLS);
    let mut counts = vec![0usize; cols * rows];
    let mut bytes = vec![0u8; cols * rows * CELL_BYTES];
    let last = p_points.len() - 1;
    // The cells from `p_low` to `p_high`, in pixels along one side, that are on the grid.
    let span = |p_low: f32, p_high: f32, p_origin: f32, p_count: usize| {
      let first = (((p_low - p_origin) / cell).floor().max(0.0) as usize).min(p_count - 1);
      let end = (((p_high - p_origin) / cell).floor().max(0.0) as usize).min(p_count - 1);
      first..=end
    };
    for piece in 0..last.max(1) {
      let (a, b) = (p_points[piece], p_points[(piece + 1).min(last)]);
      let reach = p_reaches[piece].max(p_reaches[(piece + 1).min(last)]);
      for row in span(a.1.min(b.1) - reach, a.1.max(b.1) + reach, top, rows) {
        for col in span(a.0.min(b.0) - reach, a.0.max(b.0) + reach, left, cols) {
          let at = row * cols + col;
          counts[at] += 1;
          if counts[at] <= CELL_SEGMENTS {
            bytes[at * CELL_BYTES + counts[at]] = piece as u8;
          }
        }
      }
    }
    for (at, count) in counts.iter().enumerate() {
      bytes[at * CELL_BYTES] = if *count > CELL_SEGMENTS { ALL_SEGMENTS } else { *count as u8 };
    }
    Grid { origin: (left, top), cell, cols, rows, bytes }
  }

  /// The cell at `(x, y)`, or nothing off the grid, where the line has no color.
  fn cell_at(&self, p_x: f32, p_y: f32) -> Option<usize> {
    let (x, y) = ((p_x - self.origin.0) / self.cell, (p_y - self.origin.1) / self.cell);
    (x >= 0.0 && y >= 0.0 && x < self.cols as f32 && y < self.rows as f32).then(|| y as usize * self.cols + x as usize)
  }

  /// The grid as the texture the shader reads: a row of `CELL_BYTES / 4` pixels for each cell.
  fn texture(&self) -> GpuAux {
    GpuAux { width: (CELL_BYTES / 4) as u32, height: (self.cols * self.rows) as u32, rgba: Arc::from(self.bytes.as_slice()) }
  }
}

/// A line through points, drawn with a brush's width, softness, opacity and color. Made when it is needed: on the CPU by
/// [`render`](Generator::render), or by a shader.
#[derive(Debug)]
struct StrokeOverlay {
  points: Vec<(f32, f32)>,
  /// How far from the line the brush reaches at each point: half the brush's width there. The radius between two points changes evenly.
  radii: Vec<f32>,
  /// How much of the radius at a place the fade at the edge covers, from `0` (a sharp edge) to `1` (the whole radius), and at least one
  /// pixel wide, so the edge is never jagged.
  softness: f32,
  /// How opaque the line is at each point, from `0` to `1`. The opacity between two points changes evenly.
  opacities: Vec<f32>,
  /// The brush color, straight, each from 0 to 1, the last being its own alpha, without the opacity.
  color: [f32; 4],
  /// Which pieces of the line are near each part of the picture.
  grid: Grid,
}

impl StrokeOverlay {
  fn new(p_points: &[(f32, f32)], p_brush: &Brush) -> StrokeOverlay {
    let points = if p_points.len() <= MAX_STROKE_POINTS {
      p_points.to_vec()
    } else {
      (0..MAX_STROKE_POINTS).map(|i| p_points[i * (p_points.len() - 1) / (MAX_STROKE_POINTS - 1)]).collect()
    };
    // The width of the brush at each point, by how far along the line the point is: `0` at the first and `1` at the last, by length.
    let width = p_brush.width();
    let mut lengths = vec![0.0_f32; points.len()];
    for at in 1..points.len() {
      lengths[at] = lengths[at - 1] + ((points[at].0 - points[at - 1].0).powi(2) + (points[at].1 - points[at - 1].1).powi(2)).sqrt();
    }
    let total = lengths.last().copied().unwrap_or(0.0);
    let along = |length: &f32| if total > 0.0 { length / total } else { 0.0 };
    let radii: Vec<f32> = lengths.iter().map(|length| width.width_at(along(length)) / 2.0).collect();
    let opacity = p_brush.opacity();
    let opacities: Vec<f32> = lengths.iter().map(|length| opacity.opacity_at(along(length))).collect();
    let softness = 1.0 - p_brush.hardness();
    // How far each point's color reaches: its radius and the fade past it.
    let reaches: Vec<f32> = radii.iter().map(|radius| radius + (softness * radius).max(1.0)).collect();
    let grid = Grid::new(&points, &reaches);
    StrokeOverlay { points, radii, softness, opacities, color: solid_color(p_brush), grid }
  }

  /// How many pieces the line has: one between each two points, and a single point is one piece, a dot.
  fn pieces(&self) -> usize {
    (self.points.len().saturating_sub(1)).max(1)
  }

  /// How much of the color shows at `(x, y)` from one piece of the line, from `0` to `1`: where the piece reaches as far as the radius
  /// there, fading over its soft edge, and as opaque as the line is there.
  fn covered_by(&self, p_piece: usize, p_x: f32, p_y: f32) -> f32 {
    let last = self.points.len() - 1;
    let (a, b) = (self.points[p_piece], self.points[(p_piece + 1).min(last)]);
    let (ra, rb) = (self.radii[p_piece], self.radii[(p_piece + 1).min(last)]);
    let (oa, ob) = (self.opacities[p_piece], self.opacities[(p_piece + 1).min(last)]);
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let length_squared = abx * abx + aby * aby;
    let along = if length_squared > 0.0 { (((p_x - a.0) * abx + (p_y - a.1) * aby) / length_squared).clamp(0.0, 1.0) } else { 0.0 };
    let distance = ((p_x - (a.0 + abx * along)).powi(2) + (p_y - (a.1 + aby * along)).powi(2)).sqrt();
    let radius = ra + (rb - ra) * along;
    if radius <= 0.0 {
      return 0.0;
    }
    ((radius - distance) / (self.softness * radius).max(1.0) + 0.5).clamp(0.0, 1.0) * (oa + (ob - oa) * along)
  }

  /// How much of the color shows at `(x, y)`: the most of any piece of the line near it.
  fn coverage(&self, p_x: f32, p_y: f32) -> f32 {
    let Some(cell) = self.grid.cell_at(p_x, p_y) else {
      return 0.0;
    };
    let listed = &self.grid.bytes[cell * CELL_BYTES..(cell + 1) * CELL_BYTES];
    if listed[0] == ALL_SEGMENTS {
      (0..self.pieces()).map(|piece| self.covered_by(piece, p_x, p_y)).fold(0.0, f32::max)
    } else {
      listed[1..=listed[0] as usize].iter().map(|piece| self.covered_by(*piece as usize, p_x, p_y)).fold(0.0, f32::max)
    }
  }

  /// [`coverage`](Self::coverage) by checking every piece of the line, which a test compares the grid with.
  #[cfg(test)]
  fn coverage_of_every_piece(&self, p_x: f32, p_y: f32) -> f32 {
    (0..self.pieces()).map(|piece| self.covered_by(piece, p_x, p_y)).fold(0.0, f32::max)
  }
}

impl Generator for StrokeOverlay {
  fn render(&self, p_width: u32, p_height: u32) -> Image {
    let width = p_width as usize;
    let mut pixels = vec![0u8; width * p_height as usize * 4];
    if self.points.is_empty() || pixels.is_empty() {
      return Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA);
    }
    let [red, green, blue, _] = self.color.map(|channel| (channel * 255.0 + 0.5) as u8);
    pixels.par_chunks_exact_mut(width * 4).enumerate().for_each(|(y, row)| {
      for (x, pixel) in row.chunks_exact_mut(4).enumerate() {
        // Off the grid, which is the box around the line as wide as the brush and its fade at their widest, the pixel stays transparent.
        let coverage = self.coverage(x as f32 + 0.5, y as f32 + 0.5) * self.color[3];
        if coverage > 0.0 {
          pixel.copy_from_slice(&[red, green, blue, (coverage * 255.0 + 0.5) as u8]);
        }
      }
    });
    Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA)
  }

  fn passes(&self, _p_width: u32, _p_height: u32) -> Option<Vec<GpuPass>> {
    let mut uniforms: Vec<u8> = Vec::with_capacity(48 + MAX_STROKE_POINTS * 16);
    uniforms.extend(self.color.into_iter().flat_map(f32::to_le_bytes));
    uniforms.extend((self.points.len() as u32).to_le_bytes());
    uniforms.extend(self.softness.to_le_bytes());
    uniforms.extend((self.grid.cols as u32).to_le_bytes());
    uniforms.extend((self.grid.rows as u32).to_le_bytes());
    uniforms.extend([self.grid.origin.0, self.grid.origin.1, self.grid.cell, 0.0].into_iter().flat_map(f32::to_le_bytes));
    for index in 0..MAX_STROKE_POINTS {
      let (x, y) = self.points.get(index).copied().unwrap_or((0.0, 0.0));
      let radius = self.radii.get(index).copied().unwrap_or(0.0);
      let opacity = self.opacities.get(index).copied().unwrap_or(0.0);
      uniforms.extend([x, y, radius, opacity].into_iter().flat_map(f32::to_le_bytes));
    }
    Some(vec![GpuPass::new(include_str!("./stroke_overlay.wgsl"), uniforms).with_window().with_aux(self.grid.texture())])
  }
}

/// An image of `p_width` x `p_height` pixels holding a line drawn through `p_points` (in pixels of the image) with `p_brush`'s
/// width, hardness, opacity and solid color. Pixels away from the line are transparent, so the image is made to be blended over a
/// photo; a brush that is not a solid color draws in black. A line of more than [`MAX_STROKE_POINTS`] points is thinned evenly.
///
/// The brush's width and opacity are read along the line, `0` at its first point and `1` at its last, by length (see `Brush::with_width` and
/// `Brush::with_opacity`), so a line can taper, swell or fade. A brush with only a size is that wide all the way, and one with only a number
/// for its opacity is that opaque all the way. A line that gains points changes the places the width is read at,
/// so a taper to a point follows the end of the line as it grows.
///
/// The pixels are made on the CPU now. [`stroke_overlay_deferred`] leaves them to the GPU when there is one.
pub fn stroke_overlay(p_width: u32, p_height: u32, p_points: &[(f32, f32)], p_brush: &Brush) -> Image {
  StrokeOverlay::new(p_points, p_brush).render(p_width, p_height)
}

/// [`stroke_overlay`] as an image that is made by the GPU when its pixels are first read (see `abra_core::image::recipe`). A
/// line that gains a point is a new image that costs one shader run, so it can follow a finger frame after frame. Without a GPU
/// it is made now, with the same pixels either way.
pub fn stroke_overlay_deferred(p_width: u32, p_height: u32, p_points: &[(f32, f32)], p_brush: &Brush) -> Image {
  generated(p_width, p_height, Arc::new(StrokeOverlay::new(p_points, p_brush)))
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Color, StrokeOpacity};

  fn alpha_at(p_image: &Image, p_x: usize, p_y: usize) -> u8 {
    p_image.rgba()[(p_y * p_image.dimensions::<u32>().0 as usize + p_x) * 4 + 3]
  }

  #[test]
  fn the_line_is_solid_on_it_soft_at_its_edge_and_clear_away_from_it() {
    let red = Color::red();
    let brush = Brush::new().with_size(9).with_color(&red).with_hardness(1.0);
    let image = stroke_overlay(40, 20, &[(5.0, 10.0), (35.0, 10.0)], &brush);
    assert_eq!(alpha_at(&image, 20, 10), 255);
    assert_eq!(alpha_at(&image, 20, 2), 0);
    // The brush is 4.5 pixels from the line to its edge, which is exactly where the middle of the pixel in row 14 is.
    let edge = alpha_at(&image, 20, 14);
    assert!(edge > 0 && edge < 255, "{edge}");
    assert_eq!(alpha_at(&stroke_overlay(8, 8, &[], &brush), 4, 4), 0);
  }

  #[test]
  fn a_line_with_a_width_that_tapers_is_thinner_toward_its_end_and_ends_in_a_point() {
    let brush = Brush::new().with_size(20).with_width(abra_core::StrokeWidth::from_to(20.0, 0.0)).with_hardness(1.0);
    let image = stroke_overlay(100, 40, &[(10.0, 20.0), (90.0, 20.0)], &brush);
    let height_at = |x: usize| (0..40).filter(|y| alpha_at(&image, x, *y) > 127).count();
    assert!(height_at(15) > height_at(50) && height_at(50) > height_at(85), "{} {} {}", height_at(15), height_at(50), height_at(85));
    // A hard edge is solid over all of the radius here, so the width at 15 is 17.5 and at 50 is 10.
    assert!((16..=19).contains(&height_at(15)), "{}", height_at(15));
    // Nothing at the point itself: a radius of nothing covers nothing, not even half a pixel.
    assert_eq!(alpha_at(&image, 90, 20), 0);
  }

  #[test]
  fn a_line_with_an_opacity_that_fades_is_clearer_toward_its_end() {
    let brush = Brush::new().with_size(10).with_hardness(1.0).with_opacity(StrokeOpacity::from_to(1.0, 0.0));
    let image = stroke_overlay(100, 30, &[(10.0, 15.0), (90.0, 15.0)], &brush);
    let (start, middle, end) = (alpha_at(&image, 14, 15), alpha_at(&image, 50, 15), alpha_at(&image, 86, 15));
    assert!(start > 220 && (110..=145).contains(&middle) && end < 40, "{start} {middle} {end}");
    // A number is that opaque the whole way, and a brush without one is solid.
    let half = stroke_overlay(100, 30, &[(10.0, 15.0), (90.0, 15.0)], &Brush::new().with_size(10).with_hardness(1.0).with_opacity(0.5));
    assert!((alpha_at(&half, 50, 15) as i32 - 128).abs() <= 2);
    assert_eq!(alpha_at(&stroke_overlay(100, 30, &[(10.0, 15.0), (90.0, 15.0)], &Brush::new().with_size(10).with_hardness(1.0)), 50, 15), 255);
  }

  #[test]
  fn a_width_that_never_changes_draws_what_the_size_does() {
    let by_size = Brush::new().with_size(12).with_hardness(0.5);
    let by_width = Brush::new().with_size(3).with_width(abra_core::StrokeWidth::constant(12.0)).with_hardness(0.5);
    let points = [(5.0, 10.0), (20.0, 25.0), (45.0, 12.0)];
    assert_eq!(stroke_overlay(50, 30, &points, &by_size).rgba(), stroke_overlay(50, 30, &points, &by_width).rgba());
  }

  /// A loop of `p_count` points around `(100, 100)` that wobbles, with a width that swells and tapers.
  fn wobbly_loop(p_count: usize) -> Vec<(f32, f32)> {
    (0..p_count)
      .map(|i| {
        let angle = i as f32 / p_count as f32 * std::f32::consts::TAU;
        let wobble = 1.0 + 0.1 * (angle * 5.0).sin();
        (100.0 + 70.0 * wobble * angle.cos(), 100.0 + 70.0 * wobble * angle.sin())
      })
      .collect()
  }

  #[test]
  fn the_grid_gives_the_same_color_as_checking_every_piece_of_the_line() {
    let width = abra_core::StrokeWidth::new(vec![abra_core::WidthStop::new(3.0, 0.0), abra_core::WidthStop::new(18.0, 0.4), abra_core::WidthStop::new(0.0, 1.0)]);
    for (points, hardness) in [(wobbly_loop(255), 0.0), (wobbly_loop(40), 0.7), (wobbly_loop(3), 1.0), (vec![(50.0, 50.0)], 0.5), (vec![(20.0, 20.0), (180.0, 30.0)], 0.2)] {
      let brush = Brush::new().with_size(10).with_width(width.clone()).with_hardness(hardness);
      let overlay = StrokeOverlay::new(&points, &brush);
      for y in (0..220).step_by(3) {
        for x in (0..220).step_by(3) {
          let (x, y) = (x as f32 + 0.5, y as f32 + 0.5);
          assert_eq!(overlay.coverage(x, y), overlay.coverage_of_every_piece(x, y), "{} points, at {x} {y}", points.len());
        }
      }
    }
  }

  #[test]
  fn a_crowded_cell_checks_every_piece_and_gives_the_right_color() {
    // Two hundred pieces in a few pixels, far more than a cell lists.
    let scribble: Vec<(f32, f32)> = (0..200).map(|i| (50.0 + (i % 7) as f32, 50.0 + (i % 5) as f32)).collect();
    let overlay = StrokeOverlay::new(&scribble, &Brush::new().with_size(6).with_hardness(0.5));
    assert!(overlay.grid.bytes.chunks(CELL_BYTES).any(|cell| cell[0] == ALL_SEGMENTS), "no cell was crowded");
    for y in 40..70 {
      for x in 40..70 {
        let (x, y) = (x as f32 + 0.5, y as f32 + 0.5);
        assert_eq!(overlay.coverage(x, y), overlay.coverage_of_every_piece(x, y));
      }
    }
  }

  #[test]
  fn a_line_that_leaves_the_pixels_it_is_drawn_on_is_clipped_not_wrapped() {
    let image = stroke_overlay(30, 30, &[(-40.0, 15.0), (-10.0, 15.0), (15.0, 15.0)], &Brush::new().with_size(6).with_hardness(1.0));
    assert_eq!(alpha_at(&image, 15, 15), 255);
    assert_eq!(alpha_at(&image, 2, 25), 0);
  }

  #[test]
  fn a_long_line_is_thinned_to_the_most_points_the_shader_takes() {
    let points: Vec<(f32, f32)> = (0..1000).map(|i| (i as f32 / 10.0, 5.0)).collect();
    let overlay = StrokeOverlay::new(&points, &Brush::new());
    assert_eq!(overlay.points.len(), MAX_STROKE_POINTS);
    assert_eq!(overlay.points[0], points[0]);
    assert_eq!(overlay.points[MAX_STROKE_POINTS - 1], points[999]);
  }

  /// The shader on a real GPU gives the CPU's pixels, for a hard and a soft brush. The real provider is wrapped in one that counts,
  /// so the test fails if the work quietly fell back to the CPU (which would match perfectly).
  #[test]
  fn the_gpu_gives_the_cpu_s_pixels() -> anyhow::Result<()> {
    use abra_core::BlendMode;
    use abra_core::image::gpu::{GPU_PROVIDER_LOCK, GpuProvider, get_gpu_provider, register_gpu_provider};
    use abra_core::image::recipe::blended;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static REAL: OnceLock<GpuProvider> = OnceLock::new();
    static RUNS: AtomicUsize = AtomicUsize::new(0);

    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    gpu::GpuContext::new_default_blocking()?;
    gpu::register();
    let real = *REAL.get_or_init(|| get_gpu_provider().expect("the gpu crate registers a provider"));
    register_gpu_provider(GpuProvider {
      process: |effects, width, height, pixels| {
        RUNS.fetch_add(1, Ordering::SeqCst);
        (REAL.get().unwrap().process)(effects, width, height, pixels)
      },
      ..real
    });
    RUNS.store(0, Ordering::SeqCst);

    let (width, height) = (97, 61);
    let photo = Image::new_from_pixels(width, height, [90u8, 120, 150, 255].repeat((width * height) as usize), Channels::RGBA);
    let white = Color::white();
    let short: Vec<(f32, f32)> = vec![(8.0, 50.0), (30.0, 12.0), (60.5, 40.0), (90.0, 8.0)];
    // A long loop, so that the grid has many cells with a few pieces each, and a scribble that crowds one cell into checking every piece.
    let long: Vec<(f32, f32)> = (0..255).map(|i| (i as f32 / 255.0 * std::f32::consts::TAU)).map(|a| (48.0 + 40.0 * a.cos() * (1.0 + 0.1 * (a * 5.0).sin()), 30.0 + 24.0 * a.sin())).collect();
    let scribble: Vec<(f32, f32)> = (0..200).map(|i| (40.0 + (i % 7) as f32, 30.0 + (i % 5) as f32)).collect();
    let tapered = abra_core::StrokeWidth::from_to(20.0, 1.0);
    let fading = abra_core::StrokeOpacity::new(vec![abra_core::OpacityStop::new(0.2, 0.0), abra_core::OpacityStop::new(1.0, 0.5), abra_core::OpacityStop::new(0.0, 1.0)]);
    let swelling = abra_core::StrokeWidth::new(vec![abra_core::WidthStop::new(2.0, 0.0), abra_core::WidthStop::new(24.0, 0.4), abra_core::WidthStop::new(0.0, 1.0)]);
    for (points, size, hardness, along, fade) in [
      (&short, 9, 1.0, None, None),
      (&short, 22, 0.0, None, None),
      (&short, 14, 0.5, None, None),
      (&short, 14, 0.5, Some(tapered.clone()), None),
      (&short, 14, 1.0, Some(swelling.clone()), None),
      (&short, 12, 0.5, None, Some(fading.clone())),
      (&long, 6, 0.5, None, None),
      (&long, 8, 1.0, Some(tapered), Some(fading.clone())),
      (&scribble, 5, 0.5, Some(swelling), Some(fading)),
    ] {
      let mut brush = Brush::new().with_size(size).with_color(&white).with_hardness(hardness).with_opacity(fade.unwrap_or(StrokeOpacity::constant(0.8)));
      if let Some(along) = along {
        brush = brush.with_width(along);
      }
      let expected = blended(&photo, &stroke_overlay(width, height, points, &brush), BlendMode::Normal, 1.0);
      let made = blended(&photo, &stroke_overlay_deferred(width, height, points, &brush), BlendMode::Normal, 1.0);
      assert!(made.deferred().is_some(), "with a GPU the result is not made until it is read");
      let worst = made.rgba().iter().zip(expected.rgba()).map(|(a, e)| a.abs_diff(*e)).max().unwrap_or(0);
      assert!(worst <= 2, "size {size} hardness {hardness}: the GPU differs from the CPU by {worst}");
    }
    assert!(RUNS.load(Ordering::SeqCst) >= 9, "the GPU did not run the shader");
    abra_core::image::gpu::clear_gpu_provider();
    Ok(())
  }
}
