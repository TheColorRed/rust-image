//! Healing: replacing part of an image with texture from nearby, blended into its surroundings.
//!
//! [`heal`] takes any shape, such as a spot, a brush stroke or a polygon, and fills it so it matches what is around
//! it. The texture is borrowed from a nearby part of the image that looks alike, while the color and lighting come
//! from the pixels just outside the shape, so the result blends in without a visible edge. It is the building
//! block for blemish and object removal, spot healing, and healing brushes.

use abra_core::{Image, IntoNumber, PointF};
use drawing::{CoverageMap, CoverageMask, SampleGrid};
use rayon::prelude::*;

/// How many points per pixel side are tested to find how much of each pixel the shape covers.
const SAMPLES: u32 = 4;

/// A heal that has been described but not yet run. Create one with [`heal`], optionally configure it, then run it
/// with [`Heal::apply`].
pub struct Heal<'a> {
  shape: &'a dyn CoverageMask,
  distance: f32,
  feather: f32,
  source: Option<(i32, i32)>,
}

impl<'a> Heal<'a> {
  /// Sets how far around the shape, in pixels, the surroundings it is matched to reach. Defaults to 3.
  ///
  /// A wider ring matches the fill to more of its surroundings, which suits larger shapes and busier images.
  pub fn with_distance(mut self, p_distance: impl IntoNumber) -> Self {
    self.distance = p_distance.into::<f32>().max(1.0);
    self
  }

  /// Sets how many pixels in from the shape's edge the heal fades in, so the edge blends softly into the original.
  /// Defaults to 0, a hard edge.
  pub fn with_feather(mut self, p_feather: impl IntoNumber) -> Self {
    self.feather = p_feather.into::<f32>().max(0.0);
    self
  }

  /// Borrows texture from the pixels at this offset from the shape, such as `(-40, 0)` for 40 pixels to the left,
  /// instead of searching for the best match. The color and lighting still come from the shape's surroundings.
  pub fn with_source(mut self, p_offset: impl Into<PointF>) -> Self {
    let offset = p_offset.into();
    self.source = Some((offset.x.round() as i32, offset.y.round() as i32));
    self
  }

  /// Heals the image in place. Only color changes; alpha is left as it is. A shape that misses the image changes
  /// nothing.
  pub fn apply(&self, p_image: &mut Image) {
    let (width, height) = p_image.dimensions::<u32>();
    let Some(healed) = self.healed_pixels(p_image.rgba(), width as i32, height as i32) else {
      return;
    };
    let pixels = p_image.colors();
    for (index, color) in healed {
      let offset = index * 4;
      pixels[offset] = color[0];
      pixels[offset + 1] = color[1];
      pixels[offset + 2] = color[2];
    }
  }

  /// Heals the shape out of an RGBA buffer.
  ///
  /// The steps are:
  /// 1. Rasterize the shape into an anti-aliased coverage mask over the part of the image it touches.
  /// 2. Find the ring of pixels within the distance around the shape. That is the context the fill is matched to.
  /// 3. Search nearby for the offset whose ring looks most like this one, to borrow texture from, unless one was
  ///    given. If nothing fits, the hole is filled smoothly from the ring alone.
  /// 4. Solve for pixels inside the shape whose detail comes from the borrowed patch, and whose edge matches the
  ///    surrounding pixels exactly, so the color and lighting come from the destination.
  /// 5. Blend the result over the original, softening the inside edge by the feather.
  ///
  /// Returns `(pixel index, healed r/g/b)` for every pixel that changed, or `None` when the shape does not touch
  /// the image.
  fn healed_pixels(&self, p_rgba: &[u8], p_width: i32, p_height: i32) -> Option<Vec<(usize, [u8; 3])>> {
    let (min_x, min_y, max_x, max_y) = self.shape.bounds().unwrap_or((0.0, 0.0, p_width as f32, p_height as f32));
    if !(max_x > min_x && max_y > min_y) {
      return None;
    }

    // The area of the shape plus its ring, before it is clipped to the image. A patch borrowed from elsewhere needs
    // all of this to be in the image, so its size is kept.
    let outer = Window {
      x: (min_x - self.distance).floor() as i32,
      y: (min_y - self.distance).floor() as i32,
      width: ((max_x - min_x) + 2.0 * self.distance).ceil() as usize + 2,
      height: ((max_y - min_y) + 2.0 * self.distance).ceil() as usize + 2,
    };

    // Clip to the image.
    let (x0, y0) = (outer.x.max(0), outer.y.max(0));
    let x1 = (outer.x + outer.width as i32).min(p_width);
    let y1 = (outer.y + outer.height as i32).min(p_height);
    if x1 <= x0 || y1 <= y0 {
      return None;
    }
    let window = Window {
      x: x0,
      y: y0,
      width: (x1 - x0) as usize,
      height: (y1 - y0) as usize,
    };

    // 1. Coverage mask.
    let coverage =
      CoverageMap::rasterize(self.shape, x0, y0, window.width, window.height, SampleGrid::from_aa_level(SAMPLES));
    if coverage.is_empty() {
      return None;
    }
    let hole = coverage.covered();

    // 2. Distance from the shape (outside) and from the edge (inside).
    let outside = coverage.distance_outside();
    let inside = coverage.distance_inside();
    let ring: Vec<usize> = (0..hole.len()).filter(|&i| !hole[i] && outside[i] <= self.distance).collect();

    // Reads past the edge of the image repeat the edge, so a patch that overhangs it is still safe to read.
    let pixel = |p_x: i32, p_y: i32| -> [f32; 3] {
      let (p_x, p_y) = (p_x.clamp(0, p_width - 1), p_y.clamp(0, p_height - 1));
      let offset = ((p_y * p_width + p_x) as usize) * 4;
      [
        p_rgba[offset] as f32,
        p_rgba[offset + 1] as f32,
        p_rgba[offset + 2] as f32,
      ]
    };

    // 3. Where to borrow texture from.
    let source = self.source.or_else(|| find_source(&ring, &hole, &window, &outer, (p_width, p_height), &pixel));

    // 4. Solve.
    let solved = solve(&hole, &window, &pixel, source);

    // 5. Blend.
    let values = coverage.values();
    let mut pixels = Vec::new();
    for local_y in 0..window.height {
      for local_x in 0..window.width {
        let local = window.index(local_x, local_y);
        let mut alpha = values[local];
        if alpha <= 0.0 {
          continue;
        }
        if self.feather > 0.5 {
          // Distance 1 is the pixel on the edge, so measure from the middle of that pixel.
          let t = ((inside[local] - 0.5) / self.feather).clamp(0.0, 1.0);
          alpha *= t * t * (3.0 - 2.0 * t);
        }
        let (x, y) = (window.x + local_x as i32, window.y + local_y as i32);
        let original = pixel(x, y);
        let healed = solved[local];
        let mut color = [0u8; 3];
        for channel in 0..3 {
          let value = original[channel] + (healed[channel] - original[channel]) * alpha;
          color[channel] = value.round().clamp(0.0, 255.0) as u8;
        }
        pixels.push(((y * p_width + x) as usize, color));
      }
    }
    Some(pixels)
  }
}

/// Heals the part of the image covered by `p_shape`, filling it with texture borrowed from a nearby part of the
/// image that looks alike, blended so its color and lighting match the pixels around the shape.
///
/// Any coverage mask works as the shape, such as a spot, a brush stroke or a polygon:
/// ```ignore
/// heal(&StrokeCoverage::new(&[PointF::new(120, 80)], 6.0)).with_distance(4).with_feather(2).apply(&mut image);
/// ```
/// # Arguments
/// - `p_shape`: The part of the image to heal, in image pixels.
pub fn heal(p_shape: &dyn CoverageMask) -> Heal<'_> {
  Heal {
    shape: p_shape,
    distance: 3.0,
    feather: 0.0,
    source: None,
  }
}

/// A rectangle of the image, in pixels, that the heal works within.
struct Window {
  x: i32,
  y: i32,
  width: usize,
  height: usize,
}

impl Window {
  fn index(&self, p_x: usize, p_y: usize) -> usize {
    p_y * self.width + p_x
  }
}

/// Finds the offset `(dx, dy)` such that the pixels at `ring + offset` look most like the ring, or `None` when
/// there is no place in the image to borrow from.
///
/// A candidate must lie entirely inside the image and must not land on the hole, so it never borrows from the
/// blemish. How well it matches is how alike the two rings' pixels are, mostly ignoring an overall brightness or
/// color shift, because solving for the fill corrects that.
/// - `p_ring`: The ring, as indices into the window.
/// - `p_hole`: Which pixels of the window are the hole.
fn find_source(
  p_ring: &[usize], p_hole: &[bool], p_window: &Window, p_outer: &Window, p_image: (i32, i32),
  p_pixel: &(impl Fn(i32, i32) -> [f32; 3] + Sync),
) -> Option<(i32, i32)> {
  if p_ring.is_empty() {
    return None;
  }

  // Comparing every ring pixel is unnecessary; a spread of them is enough.
  const MAX_SAMPLES: usize = 400;
  let stride = p_ring.len().div_ceil(MAX_SAMPLES);
  let samples: Vec<(i32, i32, [f32; 3])> = p_ring
    .iter()
    .step_by(stride)
    .map(|&local| {
      let (x, y) = (p_window.x + (local % p_window.width) as i32, p_window.y + (local / p_window.width) as i32);
      (x, y, p_pixel(x, y))
    })
    .collect();

  // A spread of the hole's pixels, to check a candidate does not land on it.
  const MAX_HOLE_SAMPLES: usize = 1000;
  let hole_pixels: Vec<usize> = (0..p_hole.len()).filter(|&i| p_hole[i]).collect();
  let hole_stride = hole_pixels.len().div_ceil(MAX_HOLE_SAMPLES).max(1);
  let hole_samples: Vec<(i32, i32)> = hole_pixels
    .iter()
    .step_by(hole_stride)
    .map(|&local| (p_window.x + (local % p_window.width) as i32, p_window.y + (local / p_window.width) as i32))
    .collect();
  let on_hole = |x: i32, y: i32| {
    let (local_x, local_y) = (x - p_window.x, y - p_window.y);
    local_x >= 0
      && local_y >= 0
      && (local_x as usize) < p_window.width
      && (local_y as usize) < p_window.height
      && p_hole[local_y as usize * p_window.width + local_x as usize]
  };

  let (outer_width, outer_height) = (p_outer.width as i32, p_outer.height as i32);
  // Where the pixels of the hole and ring would come from must be in the image and not be the hole. It is the
  // pixels that matter, not the bounding box: a long diagonal stroke's box covers most of the image, but the
  // stroke itself leaves plenty of room beside it.
  let usable = |x: i32, y: i32| x >= 0 && y >= 0 && x < p_image.0 && y < p_image.1 && !on_hole(x, y);
  let valid = |dx: i32, dy: i32| {
    hole_samples.iter().all(|&(x, y)| usable(x + dx, y + dy)) && samples.iter().all(|&(x, y, _)| usable(x + dx, y + dy))
  };

  let cost = |dx: i32, dy: i32| -> f32 {
    let mut sum = [0.0f32; 3];
    let mut sum_squares = [0.0f32; 3];
    for &(x, y, color) in &samples {
      let other = p_pixel(x + dx, y + dy);
      for c in 0..3 {
        let difference = other[c] - color[c];
        sum[c] += difference;
        sum_squares[c] += difference * difference;
      }
    }
    let count = samples.len() as f32;
    (0..3)
      .map(|c| {
        let mean = sum[c] / count;
        let variance = sum_squares[c] / count - mean * mean;
        // Mostly texture, but a candidate that is also the right color is preferred.
        variance + 0.25 * mean * mean
      })
      .sum()
  };

  // Rank by cost, then by offset, so equal costs always resolve the same way.
  let best_of = |candidates: Vec<(i32, i32)>| -> Option<(i32, i32)> {
    candidates
      .into_par_iter()
      .filter(|&(dx, dy)| valid(dx, dy))
      .map(|(dx, dy)| (cost(dx, dy), dx, dy))
      .min_by(|a, b| a.0.total_cmp(&b.0).then((a.1, a.2).cmp(&(b.1, b.2))))
      .map(|(_, dx, dy)| (dx, dy))
  };

  // Search within a reach that keeps the borrowed patch nearby, and never further than the image is wide.
  let reach = 3 * outer_width.max(outer_height);
  let (max_dx, max_dy) = (reach.min(p_image.0), reach.min(p_image.1));
  let (min_dx, min_dy) = (-max_dx, -max_dy);

  // Coarse search over a grid of offsets. Most of the grid is rejected at once, and the offsets that are left can
  // be a narrow band beside a long stroke, so the grid is fine, but kept to a fixed size for a large image.
  const GRID: i32 = 96;
  let span = (max_dx - min_dx).max(max_dy - min_dy);
  let mut step = ((span + GRID - 1) / GRID).max(1);
  let mut coarse = Vec::new();
  let mut dy = min_dy;
  while dy <= max_dy {
    let mut dx = min_dx;
    while dx <= max_dx {
      coarse.push((dx, dy));
      dx += step;
    }
    dy += step;
  }
  let mut best = best_of(coarse)?;

  // Refine around the best one on a finer grid, until it is down to single pixels.
  while step > 1 {
    let spacing = (step / 4).max(1);
    let mut fine = Vec::new();
    let mut dy = -step;
    while dy <= step {
      let mut dx = -step;
      while dx <= step {
        fine.push((best.0 + dx, best.1 + dy));
        dx += spacing;
      }
      dy += spacing;
    }
    best = best_of(fine).unwrap_or(best);
    step = spacing;
  }
  Some(best)
}

/// Solves for the color of every pixel of the hole, one channel at a time.
///
/// Each hole pixel differs from its neighbors the way the borrowed pixels differ from theirs, and pixels just
/// outside the hole are held at their real color. Without a borrowed patch nothing differs, and this fills the
/// hole with a smooth blend of its surroundings.
///
/// Returns colors for the whole window; only those of hole pixels are meaningful.
fn solve(
  p_hole: &[bool], p_window: &Window, p_pixel: &impl Fn(i32, i32) -> [f32; 3], p_source: Option<(i32, i32)>,
) -> Vec<[f32; 3]> {
  let (width, height) = (p_window.width, p_window.height);
  let at = |x: usize, y: usize| p_pixel(p_window.x + x as i32, p_window.y + y as i32);
  // The borrowed pixel at a position, or zero when there is no patch, which leaves nothing to differ.
  let guide = |x: usize, y: usize| -> [f32; 3] {
    match p_source {
      Some((dx, dy)) => p_pixel(p_window.x + x as i32 + dx, p_window.y + y as i32 + dy),
      None => [0.0; 3],
    }
  };

  const NEIGHBORS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
  let neighbors = |x: usize, y: usize| {
    NEIGHBORS.iter().filter_map(move |&(dx, dy)| {
      let (nx, ny) = (x as i32 + dx, y as i32 + dy);
      // Pixels outside the window are outside the image or beyond the ring; treat both as not there.
      (nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height).then_some((nx as usize, ny as usize))
    })
  };

  let mut colors: Vec<[f32; 3]> = (0..width * height).map(|i| at(i % width, i / width)).collect();
  // For each hole pixel: what it is pulled toward (the fixed neighbors' colors plus how it differs from the
  // borrowed ones), and how many neighbors it has.
  let mut targets: Vec<([f32; 3], f32)> = vec![([0.0; 3], 0.0); width * height];
  let mut hole_pixels = Vec::new();
  let mut edge_sum = [0.0f32; 3];
  let mut edge_count = 0.0f32;

  for y in 0..height {
    for x in 0..width {
      let local = y * width + x;
      if !p_hole[local] {
        continue;
      }
      let (mut fixed, mut count) = ([0.0f32; 3], 0.0f32);
      let own_guide = guide(x, y);
      for (nx, ny) in neighbors(x, y) {
        count += 1.0;
        let neighbor_guide = guide(nx, ny);
        for c in 0..3 {
          fixed[c] += own_guide[c] - neighbor_guide[c];
        }
        if !p_hole[ny * width + nx] {
          let color = colors[ny * width + nx];
          for c in 0..3 {
            fixed[c] += color[c];
            edge_sum[c] += color[c];
          }
          edge_count += 1.0;
        }
      }
      targets[local] = (fixed, count);
      hole_pixels.push((x, y));
    }
  }
  if edge_count == 0.0 {
    // The hole has no surroundings to match: leave it as it is.
    return colors;
  }

  // Start from the average surrounding color, so the solver only has to smooth the detail in.
  let average = [
    edge_sum[0] / edge_count,
    edge_sum[1] / edge_count,
    edge_sum[2] / edge_count,
  ];
  for &(x, y) in &hole_pixels {
    colors[y * width + x] = average;
  }

  // Gauss-Seidel with over-relaxation. Information travels a pixel per pass, so a bigger hole needs more of them.
  const OVER_RELAXATION: f32 = 1.9;
  const TOLERANCE: f32 = 0.05;
  let max_passes = (4 * width.max(height)).clamp(200, 4000);
  for _ in 0..max_passes {
    let mut largest_change = 0.0f32;
    for &(x, y) in &hole_pixels {
      let local = y * width + x;
      let (fixed, count) = targets[local];
      let mut sums = fixed;
      for (nx, ny) in neighbors(x, y) {
        if p_hole[ny * width + nx] {
          let neighbor = colors[ny * width + nx];
          for c in 0..3 {
            sums[c] += neighbor[c];
          }
        }
      }
      for c in 0..3 {
        let current = colors[local][c];
        // Colors far outside the displayable range only make the neighbors' updates worse.
        let updated = (current + OVER_RELAXATION * (sums[c] / count - current)).clamp(-32.0, 288.0);
        largest_change = largest_change.max((updated - current).abs());
        colors[local][c] = updated;
      }
    }
    if largest_change < TOLERANCE {
      break;
    }
  }
  colors
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;
  use drawing::{PolygonCoverage, RectCoverage, StrokeCoverage};

  /// An opaque image whose color at each pixel is `color(x, y)`.
  fn image_of(p_width: u32, p_height: u32, p_color: impl Fn(u32, u32) -> [u8; 3]) -> Image {
    let mut bytes = Vec::new();
    for y in 0..p_height {
      for x in 0..p_width {
        let [r, g, b] = p_color(x, y);
        bytes.extend_from_slice(&[r, g, b, 255]);
      }
    }
    Image::new_from_pixels(p_width, p_height, &bytes, Channels::RGBA)
  }

  fn rgb(p_image: &Image, p_x: u32, p_y: u32) -> [u8; 3] {
    let (r, g, b, _) = p_image.get_pixel(p_x, p_y).unwrap();
    [r, g, b]
  }

  fn assert_close(p_actual: [u8; 3], p_expected: [u8; 3], p_tolerance: i32) {
    for c in 0..3 {
      assert!(
        (p_actual[c] as i32 - p_expected[c] as i32).abs() <= p_tolerance,
        "{p_actual:?} is not within {p_tolerance} of {p_expected:?}"
      );
    }
  }

  const BLEMISH: [u8; 3] = [200, 30, 30];

  /// A 7x7 blemish centered on `(p_x, p_y)`.
  fn with_blemish(p_x: u32, p_y: u32, p_base: impl Fn(u32, u32) -> [u8; 3]) -> Image {
    image_of(100, 100, |x, y| if x.abs_diff(p_x) <= 3 && y.abs_diff(p_y) <= 3 { BLEMISH } else { p_base(x, y) })
  }

  fn spot(p_x: f32, p_y: f32, p_radius: f32) -> StrokeCoverage {
    StrokeCoverage::new(&[PointF::new(p_x, p_y)], p_radius)
  }

  #[test]
  fn removes_a_blemish_from_a_flat_background() {
    let mut image = with_blemish(50, 50, |_, _| [120, 140, 160]);
    heal(&spot(50.0, 50.0, 6.0)).with_distance(8).apply(&mut image);
    for (x, y) in [(50, 50), (48, 52), (53, 47), (47, 47), (53, 53)] {
      assert_close(rgb(&image, x, y), [120, 140, 160], 2);
    }
  }

  #[test]
  fn follows_the_lighting_of_a_gradient() {
    let base = |x: u32, _y: u32| [(x * 2) as u8, 100, 100];
    let mut image = with_blemish(50, 50, base);
    heal(&spot(50.0, 50.0, 6.0)).with_distance(8).apply(&mut image);
    // Inside the blemish the red channel should continue the ramp rather than settle on an average.
    assert_close(rgb(&image, 47, 50), base(47, 50), 6);
    assert_close(rgb(&image, 53, 50), base(53, 50), 6);
  }

  #[test]
  fn borrows_texture_when_there_is_some() {
    // Vertical stripes with a period of 6 pixels: filling flat would leave no stripes at all.
    let base = |x: u32, _y: u32| if x % 6 < 3 { [60, 60, 60] } else { [180, 180, 180] };
    let mut image = with_blemish(50, 50, base);
    heal(&spot(50.0, 50.0, 6.0)).with_distance(8).apply(&mut image);
    let values: Vec<u8> = (46..=54).map(|x| rgb(&image, x, 50)[0]).collect();
    let spread = values.iter().max().unwrap() - values.iter().min().unwrap();
    assert!(spread > 60, "stripes were smoothed away: {values:?}");
  }

  #[test]
  fn a_given_source_is_used_instead_of_searching() {
    // Flat gray, with a lone dark dot 20 pixels to the left of the blemish. Searching would never pick the dot, so
    // it only shows up in the healed spot when that source is given.
    let mut image = image_of(100, 100, |x, y| {
      if x.abs_diff(50) <= 3 && y.abs_diff(50) <= 3 {
        BLEMISH
      } else if x == 30 && y == 50 {
        [0, 0, 0]
      } else {
        [128, 128, 128]
      }
    });
    heal(&spot(50.0, 50.0, 6.0)).with_distance(8).with_source((-20, 0)).apply(&mut image);
    assert!(rgb(&image, 50, 50)[0] < 100, "the dot was not borrowed: {:?}", rgb(&image, 50, 50));
    assert_close(rgb(&image, 47, 47), [128, 128, 128], 4);
  }

  #[test]
  fn only_changes_color_inside_the_shape() {
    let mut image = with_blemish(50, 50, |x, y| [(x * 2) as u8, (y * 2) as u8, 100]);
    let before = image.clone();
    heal(&spot(50.0, 50.0, 6.0)).with_distance(8).apply(&mut image);
    for y in 0..100 {
      for x in 0..100 {
        let distance = ((x as f32 - 50.0).powi(2) + (y as f32 - 50.0).powi(2)).sqrt();
        if distance > 7.5 {
          assert_eq!(image.get_pixel(x, y), before.get_pixel(x, y), "pixel ({x}, {y}) changed");
        }
      }
    }
  }

  #[test]
  fn keeps_alpha() {
    let mut bytes = vec![0u8; 100 * 100 * 4];
    for (i, pixel) in bytes.chunks_mut(4).enumerate() {
      pixel.copy_from_slice(&[100, 100, 100, (i % 200) as u8 + 20]);
    }
    let mut image = Image::new_from_pixels(100, 100, &bytes, Channels::RGBA);
    heal(&RectCoverage::centered((50, 50), (10, 10))).with_distance(6).apply(&mut image);
    for i in 0..100 * 100 {
      assert_eq!(image.rgba()[i * 4 + 3], bytes[i * 4 + 3]);
    }
  }

  #[test]
  fn handles_shapes_at_and_past_the_image_edge() {
    let mut image = image_of(40, 40, |x, y| [(x * 5) as u8, (y * 5) as u8, 0]);
    heal(&spot(0.0, 0.0, 8.0)).with_distance(6).apply(&mut image);
    heal(&spot(39.0, 20.0, 8.0)).with_distance(6).apply(&mut image);
    heal(&StrokeCoverage::new(&[PointF::new(5, 39), PointF::new(35, 45)], 3.0)).with_distance(6).apply(&mut image);
    // Covers the whole image, so no neighboring patch can fit.
    heal(&RectCoverage::centered((20, 20), (80, 80))).with_distance(6).apply(&mut image);
  }

  #[test]
  fn ignores_shapes_that_miss_the_image() {
    let mut image = image_of(40, 40, |x, y| [x as u8, y as u8, 7]);
    let before = image.clone();
    heal(&spot(500.0, 500.0, 8.0)).with_distance(6).apply(&mut image);
    heal(&spot(20.0, 20.0, 0.0)).with_distance(6).apply(&mut image);
    heal(&PolygonCoverage::new(Vec::new())).apply(&mut image);
    assert_eq!(image.rgba(), before.rgba());
  }

  #[test]
  fn a_long_diagonal_stroke_still_finds_texture_to_borrow() {
    // Stripes, with a diagonal blemish across the middle. Its bounding box covers most of the image, so there is
    // nowhere for a bounding box to fit beside it, but the pixels beside the stroke itself are free to borrow.
    let base = |x: u32, _y: u32| if x % 8 < 4 { [60, 60, 60] } else { [180, 180, 180] };
    let mut image =
      image_of(160, 160, |x, y| if x.abs_diff(y) <= 1 && (30..=130).contains(&x) { BLEMISH } else { base(x, y) });
    heal(&StrokeCoverage::new(&[PointF::new(30, 30), PointF::new(130, 130)], 3.0)).with_distance(6).apply(&mut image);
    for i in [40u32, 60, 90, 120] {
      let healed = rgb(&image, i, i)[0] as i32;
      let expected = base(i, i)[0] as i32;
      assert!((healed - expected).abs() <= 40, "at ({i}, {i}) got {healed}, wanted about {expected}");
    }
  }
}
