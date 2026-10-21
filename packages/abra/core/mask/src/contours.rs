use std::collections::HashMap;

use abra_core::{Area, Color, Image, LineJoin, PointF, simplify_points, smoothed_area};
use drawing::fill;

use crate::Mask;

/// Which side of a cell an outline crosses: a horizontal edge between two samples side by side, or a vertical one between two
/// samples one above the other. The numbers are the position of the first sample of the pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Edge {
  Horizontal(i32, i32),
  Vertical(i32, i32),
}

/// What to do in a cell, from which of its corners are on: the outlines to draw, each from the side it comes in by to the side it goes
/// out by, with the inside on the left of the way it goes (as seen on the screen). The corners are counted `top left = 1`, `top right = 2`,
/// `bottom right = 4` and `bottom left = 8`, and the sides are the top (`T`), the right (`R`), the bottom (`B`) and the left (`L`).
/// Two corners on across a cell (`5` and `10`) are joined when the middle of the cell is on and are apart when it is off.
fn cell_lines(p_corners: u8, p_middle_on: bool) -> &'static [(char, char)] {
  match p_corners {
    1 => &[('L', 'T')],
    2 => &[('T', 'R')],
    4 => &[('R', 'B')],
    8 => &[('B', 'L')],
    3 => &[('L', 'R')],
    6 => &[('T', 'B')],
    12 => &[('R', 'L')],
    9 => &[('B', 'T')],
    14 => &[('T', 'L')],
    13 => &[('R', 'T')],
    11 => &[('B', 'R')],
    7 => &[('L', 'B')],
    5 if p_middle_on => &[('R', 'T'), ('L', 'B')],
    5 => &[('L', 'T'), ('R', 'B')],
    10 if p_middle_on => &[('T', 'L'), ('B', 'R')],
    10 => &[('T', 'R'), ('B', 'L')],
    _ => &[],
  }
}

impl Mask {
  /// The outlines of the shapes in the mask, found by marching squares. Each outline is a closed loop of points (the last point joins the
  /// first and is not repeated), in the pixels of the mask, where the pixel at `x`, `y` has its middle at `x + 0.5`, `y + 0.5`. Use it to
  /// stroke or fill a shape as a path, to simplify and smooth its edge, or to trace a selection.
  ///
  /// A pixel is inside the shape when its value is at or above `p_level`. Where two neighbors are on either side of the level, the
  /// outline crosses between them in proportion to their values, so a soft edge is placed where the mask passes the level and not on the
  /// pixel grid. A shape that touches the edge of the mask is closed along the edge. Outlines of the inside of a shape have the inside on
  /// the left as they are drawn on the screen, so an outer outline runs counterclockwise and the outline of a hole runs clockwise; fill
  /// them with the even-odd rule, or by the winding, to get the holes.
  ///
  /// A mask of hard `0` and `255` values has its corners cut by half a pixel, since the points are on the sides between pixels and not at
  /// their corners; simplifying the outline takes the steps out and keeps the cut small next to a shape of any size.
  /// - `p_level`: The value at which the edge of a shape is, `1` to `255` (`0` counts as `1`, since the outside of the mask is `0`).
  pub fn contours(&self, p_level: u8) -> Vec<Vec<PointF>> {
    let (width, height) = (self.width() as i32, self.height() as i32);
    let level = p_level.max(1) as f32;
    let values = self.values();
    // The value at a sample: the pixel of the mask, and nothing outside it, so that a shape at the edge is closed there.
    let at = |x: i32, y: i32| -> f32 {
      if x < 0 || y < 0 || x >= width || y >= height {
        0.0
      } else {
        values[(y * width + x) as usize] as f32
      }
    };
    // Where an outline crosses a side: between its two samples, in proportion to how far each is from the level.
    let point = |edge: Edge| -> PointF {
      let (first, second, x, y) = match edge {
        Edge::Horizontal(x, y) => (at(x, y), at(x + 1, y), x, y),
        Edge::Vertical(x, y) => (at(x, y), at(x, y + 1), x, y),
      };
      let along = if second == first { 0.5 } else { ((level - first) / (second - first)).clamp(0.0, 1.0) };
      match edge {
        Edge::Horizontal(..) => PointF { x: x as f32 + 0.5 + along, y: y as f32 + 0.5 },
        Edge::Vertical(..) => PointF { x: x as f32 + 0.5, y: y as f32 + 0.5 + along },
      }
    };

    // Every cell has four samples around it, for the cells from one before the mask to its last, so that the edges of the mask are cells too.
    let mut lines: HashMap<Edge, Edge> = HashMap::new();
    for y in -1..height {
      for x in -1..width {
        let (top_left, top_right, bottom_right, bottom_left) = (at(x, y), at(x + 1, y), at(x + 1, y + 1), at(x, y + 1));
        let corners = (top_left >= level) as u8
          | (top_right >= level) as u8 * 2
          | (bottom_right >= level) as u8 * 4
          | (bottom_left >= level) as u8 * 8;
        let middle_on = (top_left + top_right + bottom_right + bottom_left) / 4.0 >= level;
        for (from, to) in cell_lines(corners, middle_on) {
          let side = |name: char| match name {
            'T' => Edge::Horizontal(x, y),
            'B' => Edge::Horizontal(x, y + 1),
            'L' => Edge::Vertical(x, y),
            _ => Edge::Vertical(x + 1, y),
          };
          lines.insert(side(*from), side(*to));
        }
      }
    }

    // Each side an outline crosses has one line leaving it, so following them from any side comes back to it.
    let mut loops = Vec::new();
    while let Some(&start) = lines.keys().next() {
      let mut points = Vec::new();
      let mut edge = start;
      while let Some(next) = lines.remove(&edge) {
        points.push(point(edge));
        edge = next;
        if edge == start {
          break;
        }
      }
      if points.len() >= 3 {
        loops.push(points);
      }
    }
    loops
  }
}

impl Mask {
  /// The shapes in the mask as smooth areas: each outline from [`contours`](Mask::contours) with the points it does not need dropped
  /// (see `simplify_points`) and curves through the rest (see `smoothed_area`). An area is a closed shape, so it can be stroked, filled,
  /// clipped or written out as it is, and it stays sharp at any size. An outline too small to keep three points after simplifying is left out.
  /// - `p_level`: The value at which the edge of a shape is, as for [`contours`](Mask::contours).
  /// - `p_tolerance`: How far, in pixels, an outline may be moved to drop its points. A pixel and a half takes out the steps of the grid.
  /// - `p_corner_degrees`: How sharp a turn has to be to stay a corner, from `0` (all corners: straight lines) to `180` (none).
  pub fn traced(&self, p_level: u8, p_tolerance: f32, p_corner_degrees: f32) -> Vec<Area> {
    self
      .contours(p_level)
      .into_iter()
      .filter_map(|outline| {
        let simple = simplify_points(&outline, p_tolerance, true);
        (simple.len() >= 3).then(|| smoothed_area(&simple, p_corner_degrees))
      })
      .collect()
  }

  /// A mask of lines drawn along the edges of the shapes in this mask, the size of this mask: `255` on a line and `0` elsewhere, with the
  /// edges of the lines smooth. The lines follow [`traced`](Mask::traced), so they are smooth curves with sharp corners kept, and they are
  /// as thick as `p_width`, centered on the edge. Use it to draw outlines: fill the result with a color (`fill_image`), or take the lines
  /// out of a picture. It is made in one go, so the points of the outlines are never copied out one by one.
  /// - `p_level`, `p_tolerance`, `p_corner_degrees`: As for [`traced`](Mask::traced).
  /// - `p_width`: How thick the lines are, in pixels.
  pub fn outlined(&self, p_level: u8, p_tolerance: f32, p_corner_degrees: f32, p_width: f32) -> Mask {
    let mut lines = Image::new_from_color(self.width(), self.height(), Color::black());
    for area in self.traced(p_level, p_tolerance, p_corner_degrees) {
      let line = area.stroke(p_width).with_join(LineJoin::Round).to_area();
      fill(line, Color::white()).apply(&mut lines);
    }
    Mask::from_image(lines)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A mask from rows of `.` (off) and `#` (on).
  fn drawn(p_rows: &[&str]) -> Mask {
    let values = p_rows.iter().flat_map(|row| row.chars().map(|c| if c == '#' { 255 } else { 0 })).collect();
    Mask::from_values(p_rows[0].len() as u32, p_rows.len() as u32, values)
  }

  /// The area the loop runs around, negative for a loop that runs counterclockwise on the screen (the inside on the left).
  fn signed_area(p_points: &[PointF]) -> f32 {
    let mut sum = 0.0;
    for (at, point) in p_points.iter().enumerate() {
      let next = p_points[(at + 1) % p_points.len()];
      sum += point.x * next.y - next.x * point.y;
    }
    sum / 2.0
  }

  #[test]
  fn an_empty_mask_has_no_outline() {
    assert!(Mask::empty(5, 5).contours(128).is_empty());
  }

  #[test]
  fn a_block_has_one_outline_around_it_with_its_corners_cut() {
    let loops = drawn(&["....", ".##.", ".##.", "...."]).contours(128);
    assert_eq!(loops.len(), 1);
    // Four sides of two points, each between the middle of a pixel in and a pixel out, so the corners are cut: 3.5 of the 4 pixels.
    assert_eq!(loops[0].len(), 8);
    assert!((signed_area(&loops[0]).abs() - 3.5).abs() < 0.1, "{}", signed_area(&loops[0]));
    for point in &loops[0] {
      assert!(point.x >= 0.99 && point.x <= 3.01 && point.y >= 0.99 && point.y <= 3.01, "{point:?}");
    }
  }

  #[test]
  fn an_outer_outline_runs_counterclockwise_and_a_hole_runs_clockwise() {
    let loops = drawn(&[".....", ".###.", ".#.#.", ".###.", "....."]).contours(128);
    assert_eq!(loops.len(), 2);
    let areas: Vec<f32> = loops.iter().map(|each| signed_area(each)).collect();
    assert_eq!(areas.iter().filter(|area| **area < 0.0).count(), 1, "{areas:?}");
    assert_eq!(areas.iter().filter(|area| **area > 0.0).count(), 1, "{areas:?}");
    // The outer outline is the bigger of the two.
    let outer = areas.iter().cloned().fold(0.0, |most: f32, area| if area.abs() > most.abs() { area } else { most });
    assert!(outer < 0.0, "{areas:?}");
  }

  #[test]
  fn two_shapes_have_an_outline_each() {
    assert_eq!(drawn(&["##...", "##...", ".....", "...##", "...##"]).contours(128).len(), 2);
  }

  #[test]
  fn a_shape_that_touches_the_edge_is_closed_along_the_edge() {
    let loops = drawn(&["###", "###", "###"]).contours(128);
    assert_eq!(loops.len(), 1);
    let (low, high) = loops[0].iter().fold((f32::MAX, f32::MIN), |(low, high), point| (low.min(point.x.min(point.y)), high.max(point.x.max(point.y))));
    assert!(low.abs() < 0.01 && (high - 3.0).abs() < 0.01, "{low} {high}");
  }

  #[test]
  fn a_soft_edge_is_placed_where_the_mask_passes_the_level() {
    // From 0 to 200 across two pixels: the level of 100 is half way, so the outline crosses half way between their middles.
    let loops = Mask::from_values(3, 3, vec![0, 0, 0, 0, 200, 200, 0, 200, 200]).contours(100);
    assert_eq!(loops.len(), 1);
    let left = loops[0].iter().map(|point| point.x).fold(f32::MAX, f32::min);
    assert!((left - 1.0).abs() < 0.01, "{left}");
  }

  #[test]
  fn corners_across_a_cell_are_joined_or_apart_by_the_middle() {
    // Two pixels on a diagonal with their other corners off are separate shapes (the middle of the cell is off).
    assert_eq!(drawn(&["#.", ".#"]).contours(128).len(), 2);
    // With a third on, they are one shape.
    assert_eq!(drawn(&["#.", "##"]).contours(128).len(), 1);
  }

  /// A mask of a solid square of `p_side` pixels, in the middle of a mask `p_size` across.
  fn square(p_size: u32, p_side: u32) -> Mask {
    let edge = (p_size - p_side) / 2;
    let values = (0..p_size * p_size)
      .map(|at| {
        let (x, y) = (at % p_size, at / p_size);
        if (edge..edge + p_side).contains(&x) && (edge..edge + p_side).contains(&y) { 255 } else { 0 }
      })
      .collect();
    Mask::from_values(p_size, p_size, values)
  }

  #[test]
  fn a_shape_is_traced_into_a_smooth_area_that_holds_what_was_inside_it() {
    let areas = square(40, 20).traced(128, 1.5, 50.0);
    assert_eq!(areas.len(), 1);
    assert!(areas[0].contains((20.0, 20.0)));
    assert!(!areas[0].contains((5.0, 5.0)));
    assert!(areas[0].contains((12.0, 20.0)) && !areas[0].contains((8.0, 20.0)));
  }

  #[test]
  fn an_outline_too_small_to_keep_is_left_out() {
    // One pixel on its own traces to a diamond; with a tolerance wider than it, there is nothing left of it.
    assert!(Mask::from_values(5, 5, vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).traced(128, 5.0, 50.0).is_empty());
  }

  #[test]
  fn the_lines_of_an_outline_are_on_the_edge_and_nowhere_else() {
    let lines = square(60, 30).outlined(128, 1.5, 50.0, 4.0);
    assert_eq!((lines.width(), lines.height()), (60, 60));
    // The square is from 15 to 45. A line 4 thick is centered on each side: about 13 to 17 across the left side (simplifying may move
    // the edge by a fraction of the tolerance, so only the pixels well inside it are solid).
    assert_eq!(lines.get(15, 30), Some(255));
    assert_eq!(lines.get(14, 30), Some(255));
    assert_eq!(lines.get(16, 30), Some(255));
    assert_eq!(lines.get(10, 30), Some(0));
    assert_eq!(lines.get(20, 30), Some(0));
    assert_eq!(lines.get(30, 30), Some(0));
    assert_eq!(lines.get(2, 2), Some(0));
  }

  #[test]
  fn the_width_of_the_lines_is_the_width_asked_for() {
    let thin = square(60, 30).outlined(128, 1.5, 50.0, 2.0).coverage(128);
    let thick = square(60, 30).outlined(128, 1.5, 50.0, 6.0).coverage(128);
    assert!(thick > thin * 2.0, "{thin} {thick}");
  }

  #[test]
  fn a_hole_gets_its_own_line() {
    let mut values = vec![0u8; 50 * 50];
    for y in 10..40 {
      for x in 10..40 {
        values[y * 50 + x] = if (20..30).contains(&x) && (20..30).contains(&y) { 0 } else { 255 };
      }
    }
    let lines = Mask::from_values(50, 50, values).outlined(128, 1.5, 50.0, 2.0);
    assert_eq!(lines.get(10, 25), Some(255));
    assert_eq!(lines.get(20, 25), Some(255));
    assert_eq!(lines.get(15, 25), Some(0));
  }

  #[test]
  fn a_mask_with_nothing_in_it_has_no_lines() {
    assert_eq!(Mask::empty(20, 20).outlined(128, 1.5, 50.0, 3.0).coverage(0), 0.0);
  }

  #[test]
  fn a_level_of_zero_counts_as_one() {
    assert!(Mask::empty(3, 2).contours(0).is_empty());
    assert_eq!(Mask::from_values(3, 3, vec![0, 0, 0, 0, 1, 0, 0, 0, 0]).contours(0).len(), 1);
  }
}
