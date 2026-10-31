use crate::Shader;
use abra_core::{Gradient, Path, PointF, StrokeOpacity, StrokeWidth};

/// Stroke brush shader: applies hardness-based falloff relative to a path centerline.
///
/// Wraps another shader and modulates its alpha along a path. The shader
/// computes the closest point on the provided `Path` and uses the
/// perpendicular distance to that centerline to reduce alpha according to
/// the given `max_distance` and `hardness` values.
///
/// Example
/// ```ignore
/// let stroke = StrokeBrushShader::new(inner_shader, path.clone(), 6.0, 0.6);
/// ```
pub(crate) struct StrokeBrushShader {
  /// Inner shader providing base RGBA values.
  inner: Box<dyn Shader + Send + Sync>,
  fallback_point: PointF,
  /// How wide the stroke is along the path, `0` at its start and `1` at its end. Half of it is the influence distance from the centerline.
  width: StrokeWidth,
  /// How opaque the stroke is along the path, `0` at its start and `1` at its end.
  opacity: StrokeOpacity,
  /// How far along the path each flattened point is, from `0` at the start to `1` at the end, by length.
  along: Vec<f32>,
  /// Falloff hardness in [0.0, 1.0].
  hardness: f32,
  // Pre-flattened path points for fast closest-point queries during shading.
  flattened: Vec<PointF>,
  /// The segments near each place, so that a sample looks at the few that can matter instead of every one. See [`SegmentGrid`].
  grid: SegmentGrid,
  /// A gradient that runs along this same path, read where the closest place on the path is, so that the closest place is found once per
  /// sample and not again by the inner shader. When it is set the inner shader is not used.
  gradient: Option<Gradient>,
}

/// The segments of a flattened path sorted into square cells, each cell holding the segments within `reach` of it. A stroke only draws
/// within half its widest width of its line, so a sample in a cell has to look at that cell's segments alone to find the closest one
/// that matters; a sample whose cell is empty, or outside the grid, is too far from the line to be drawn.
struct SegmentGrid {
  left: f32,
  top: f32,
  /// The side of a cell.
  cell: f32,
  columns: usize,
  rows: usize,
  /// For each cell, row by row, the segments (named by the index of the point they start at) that come within `reach` of it.
  cells: Vec<Vec<u32>>,
}

/// The most cells a grid may have, so a long path with a wide reach does not use a lot of memory; its cells are made larger instead.
const MAX_GRID_CELLS: usize = 1 << 18;

impl SegmentGrid {
  /// Sorts the segments between `p_points` into cells, each segment into every cell that is within `p_reach` of it.
  fn new(p_points: &[PointF], p_reach: f32) -> Self {
    let empty = SegmentGrid { left: 0.0, top: 0.0, cell: 1.0, columns: 0, rows: 0, cells: Vec::new() };
    if p_points.len() < 2 || !p_reach.is_finite() {
      return empty;
    }
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for point in p_points {
      (min_x, min_y, max_x, max_y) = (min_x.min(point.x), min_y.min(point.y), max_x.max(point.x), max_y.max(point.y));
    }
    if !(min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite()) {
      return empty;
    }
    let reach = p_reach.max(0.5) + 1.0;
    let (left, top) = (min_x - reach, min_y - reach);
    let (width, height) = (max_x + reach - left, max_y + reach - top);
    // Cells about as wide as the stroke keep a few segments in each; they are made larger if there would be too many.
    let mut cell = (reach * 2.0).max(4.0);
    while (width / cell).ceil() * (height / cell).ceil() > MAX_GRID_CELLS as f32 {
      cell *= 2.0;
    }
    let (columns, rows) = ((width / cell).ceil() as usize + 1, (height / cell).ceil() as usize + 1);
    let mut cells = vec![Vec::new(); columns * rows];
    for at in 0..p_points.len() - 1 {
      let (from, to) = (p_points[at], p_points[at + 1]);
      let first_column = ((from.x.min(to.x) - reach - left) / cell).floor().max(0.0) as usize;
      let last_column = (((from.x.max(to.x) + reach - left) / cell).floor().max(0.0) as usize).min(columns - 1);
      let first_row = ((from.y.min(to.y) - reach - top) / cell).floor().max(0.0) as usize;
      let last_row = (((from.y.max(to.y) + reach - top) / cell).floor().max(0.0) as usize).min(rows - 1);
      for row in first_row..=last_row {
        for column in first_column..=last_column {
          cells[row * columns + column].push(at as u32);
        }
      }
    }
    SegmentGrid { left, top, cell, columns, rows, cells }
  }

  /// The segments that can matter to a sample at `(p_x, p_y)`, none when it is outside the grid.
  fn near(&self, p_x: f32, p_y: f32) -> &[u32] {
    let (column, row) = ((p_x - self.left) / self.cell, (p_y - self.top) / self.cell);
    if !(column >= 0.0 && row >= 0.0) || column as usize >= self.columns || row as usize >= self.rows {
      return &[];
    }
    &self.cells[row as usize * self.columns + column as usize]
  }
}

impl StrokeBrushShader {
  /// Creates a new `StrokeBrushShader` that wraps a shader and applies
  /// alpha falloff relative to the provided `Path`.
  ///
  /// Parameters
  /// - `p_inner`: boxed inner shader providing base RGBA values
  /// - `p_path`: path forming the stroke centerline
  /// - `p_width`: how wide the stroke is along the path; half of it is the influence distance from the centerline
  /// - `p_opacity`: how opaque the stroke is along the path
  /// - `p_hardness`: falloff hardness in [0.0, 1.0]
  ///
  /// Example
  /// ```ignore
  /// let s = StrokeBrushShader::new(inner, path.clone(), 6.0, 0.7);
  /// ```
  pub fn new(
    p_inner: Box<dyn Shader + Send + Sync>, p_path: &Path, p_width: StrokeWidth, p_opacity: StrokeOpacity, p_hardness: impl Into<f64>,
  ) -> Self {
    // Pre-flatten the path to a set of points; choose a tolerance that balances accuracy and performance.
    let flattened = p_path.flatten(1.0);
    let mut along = vec![0.0; flattened.len()];
    for at in 1..flattened.len() {
      let (from, to) = (flattened[at - 1], flattened[at]);
      along[at] = along[at - 1] + ((to.x - from.x).powi(2) + (to.y - from.y).powi(2)).sqrt();
    }
    let total = along.last().copied().unwrap_or(0.0);
    if total > 0.0 {
      along.iter_mut().for_each(|length| *length /= total);
    }
    let grid = SegmentGrid::new(&flattened, p_width.widest() / 2.0);
    StrokeBrushShader {
      inner: p_inner,
      fallback_point: p_path.start(),
      width: p_width,
      opacity: p_opacity,
      along,
      hardness: p_hardness.into().clamp(0.0, 1.0) as f32,
      flattened,
      grid,
      gradient: None,
    }
  }

  /// Colors the stroke with a gradient that runs along the stroke's own path, in place of the inner shader. The color is the one the
  /// gradient has at the closest place on the path, the same as a gradient shader given this path would find, but without searching the
  /// path a second time for every sample. It does nothing for a path that is closed, which the gradient shader treats differently.
  /// - `p_gradient`: The gradient. Its direction must be this stroke's path, or the colors will not be the ones the inner shader gives.
  pub fn with_gradient_along_path(mut self, p_gradient: Gradient) -> Self {
    let closed = self.flattened.len() > 1 && self.flattened[0].distance_to(self.flattened[self.flattened.len() - 1]) < 0.1;
    if !closed {
      self.gradient = Some(p_gradient);
    }
    self
  }

  /// Computes the falloff (a factor in 0..1) given a perpendicular distance
  /// from the stroke's centerline. This function is used by `shade` to
  /// modulate the alpha channel.
  /// - `p_distance`: perpendicular distance from the stroke centerline.
  /// - `p_max_distance`: how far from the centerline the stroke reaches there: half its width.
  fn compute_alpha_falloff_from_distance(&self, p_distance: f32, p_max_distance: f32) -> f32 {
    if p_distance >= p_max_distance {
      return 0.0;
    }
    let normalized_dist = p_distance / p_max_distance;

    if self.hardness < 0.5 {
      let t = self.hardness * 2.0;
      let quadratic = 1.0 - (normalized_dist * normalized_dist);
      let linear = 1.0 - normalized_dist;
      (quadratic * (1.0 - t) + linear * t).max(0.0)
    } else {
      let t = (self.hardness - 0.5) * 2.0;
      let linear = 1.0 - normalized_dist;
      let hard_edge = if normalized_dist < 0.5 { 1.0 } else { 0.0 };
      (linear * (1.0 - t) + hard_edge * t).max(0.0)
    }
  }
}

impl Shader for StrokeBrushShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let (distance_sq, along, time) = self.closest_on_path(p_x, p_y);
    let (r, g, b, mut a) = match &self.gradient {
      Some(gradient) => gradient.color_at(time).rgba(),
      None => self.inner.shade(p_x, p_y),
    };
    let falloff = self.compute_alpha_falloff_from_distance(distance_sq.sqrt(), self.width.width_at(along) / 2.0);
    a = ((a as f32) * falloff * self.opacity.opacity_at(along)) as u8;
    (r, g, b, a)
  }
}

impl StrokeBrushShader {
  // Finds the closest point on the pre-flattened path to (p_x, p_y).
  /// Finds the closest point on the pre-flattened path to `(p_x, p_y)`.
  ///
  /// This projects the point onto each segment of the flattened polyline that is near it, and takes the closest. A point
  /// farther from the line than the stroke reaches gets the largest distance, so nothing is drawn there.
  ///
  /// Parameters
  /// - `p_x`, `p_y`: the query point in device coordinates.
  ///
  /// Returns `(distance squared, along, time)`: how far the closest point is, how far along the path it is by length (`0` to `1`), and
  /// its `time` as [`Path::closest_time`] counts it, by segments (`0` to `1`).
  fn closest_on_path(&self, p_x: f32, p_y: f32) -> (f32, f32, f32) {
    if self.flattened.len() < 2 {
      let dx = p_x - self.fallback_point.x;
      let dy = p_y - self.fallback_point.y;
      return (dx * dx + dy * dy, 0.0, 0.0);
    }
    let mut min_distance_sq = f32::MAX;
    let mut along = 0.0;
    let mut time = 0.0;
    for &at in self.grid.near(p_x, p_y) {
      let i = at as usize;
      let p1 = self.flattened[i];
      let p2 = self.flattened[i + 1];
      let seg_x = p2.x - p1.x;
      let seg_y = p2.y - p1.y;
      let seg_len_sq = seg_x * seg_x + seg_y * seg_y;
      if seg_len_sq == 0.0 {
        continue;
      }
      let t = (((p_x - p1.x) * seg_x + (p_y - p1.y) * seg_y) / seg_len_sq).clamp(0.0, 1.0);
      let dx = p_x - (p1.x + seg_x * t);
      let dy = p_y - (p1.y + seg_y * t);
      let distance_sq = dx * dx + dy * dy;
      if distance_sq < min_distance_sq {
        min_distance_sq = distance_sq;
        along = self.along[i] + (self.along[i + 1] - self.along[i]) * t;
        time = (i as f32 + t) / (self.flattened.len() - 1) as f32;
      }
    }
    (min_distance_sq, along, time)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Path;

  /// A wavy path like a flare's trail, long and curved so that most of the box around it is empty.
  fn wavy() -> Path {
    let points: Vec<(f32, f32)> = (0..=24)
      .map(|i| {
        let t = i as f32 / 24.0;
        (300.0 - 280.0 * t + 25.0 * (t * 9.0).sin(), 20.0 + 380.0 * t * t)
      })
      .collect();
    let mut path = Path::new();
    path.move_to(points[0]);
    for pair in points.windows(2) {
      path.quad_to(pair[0], ((pair[0].0 + pair[1].0) / 2.0, (pair[0].1 + pair[1].1) / 2.0));
    }
    path
  }

  /// The closest place on the path found by looking at every segment, which the grid has to agree with wherever the stroke is drawn.
  fn by_every_segment(p_shader: &StrokeBrushShader, p_x: f32, p_y: f32) -> (f32, f32) {
    let (mut best, mut along) = (f32::MAX, 0.0);
    for i in 0..p_shader.flattened.len() - 1 {
      let (p1, p2) = (p_shader.flattened[i], p_shader.flattened[i + 1]);
      let (seg_x, seg_y) = (p2.x - p1.x, p2.y - p1.y);
      let length_sq = seg_x * seg_x + seg_y * seg_y;
      if length_sq == 0.0 {
        continue;
      }
      let t = (((p_x - p1.x) * seg_x + (p_y - p1.y) * seg_y) / length_sq).clamp(0.0, 1.0);
      let distance_sq = (p_x - (p1.x + seg_x * t)).powi(2) + (p_y - (p1.y + seg_y * t)).powi(2);
      if distance_sq < best {
        best = distance_sq;
        along = p_shader.along[i] + (p_shader.along[i + 1] - p_shader.along[i]) * t;
      }
    }
    (best, along)
  }

  #[test]
  fn the_segment_grid_finds_what_looking_at_every_segment_finds_wherever_the_stroke_is_drawn() {
    struct Solid;
    impl Shader for Solid {
      fn shade(&self, _p_x: f32, _p_y: f32) -> (u8, u8, u8, u8) {
        (255, 255, 255, 255)
      }
    }
    let width = StrokeWidth::new(vec![abra_core::WidthStop::new(3.0, 0.0), abra_core::WidthStop::new(11.0, 1.0)]);
    let shader = StrokeBrushShader::new(Box::new(Solid), &wavy(), width.clone(), StrokeOpacity::default(), 0.5);
    let reach = width.widest() / 2.0;
    let (mut inside, mut outside) = (0, 0);
    for y in (0..420).step_by(2) {
      for x in (0..330).step_by(2) {
        let (p_x, p_y) = (x as f32 + 0.3, y as f32 + 0.7);
        let (expected, expected_along) = by_every_segment(&shader, p_x, p_y);
        let (found, found_along, _) = shader.closest_on_path(p_x, p_y);
        if expected.sqrt() < reach {
          inside += 1;
          assert_eq!((found, found_along), (expected, expected_along), "at ({p_x}, {p_y})");
        } else {
          // Too far from the line to be drawn: the shade has to be clear, which is a distance past the reach.
          outside += 1;
          assert!(found.sqrt() >= reach, "at ({p_x}, {p_y}) {found} is within the reach");
        }
      }
    }
    assert!(inside > 500 && outside > 5000, "inside {inside}, outside {outside}");
  }

  #[test]
  fn a_gradient_read_along_the_path_is_the_color_a_gradient_shader_gives() {
    let path = wavy();
    let gradient = abra_core::Gradient::evenly(vec![
      abra_core::Color::white(),
      abra_core::Color::from_hex(0xff8a30),
      abra_core::Color::from_hex(0x203060),
    ])
    .with_direction(path.clone());
    let make = || {
      let inner = crate::shader_from_fill_with_path(abra_core::Fill::Gradient(std::borrow::Cow::Owned(gradient.clone())), Some(path.clone()));
      StrokeBrushShader::new(inner, &path, StrokeWidth::from(8.0), StrokeOpacity::default(), 0.5)
    };
    let (searched, read_along) = (make(), make().with_gradient_along_path(gradient.clone()));
    let mut compared = 0;
    for y in (0..420).step_by(3) {
      for x in (0..330).step_by(3) {
        let (p_x, p_y) = (x as f32 + 0.4, y as f32 + 0.6);
        let (expected, found) = (searched.shade(p_x, p_y), read_along.shade(p_x, p_y));
        if expected.3 > 0 {
          compared += 1;
          assert_eq!(found, expected, "at ({p_x}, {p_y})");
        }
      }
    }
    assert!(compared > 300, "only {compared} samples were on the stroke");
  }
}
