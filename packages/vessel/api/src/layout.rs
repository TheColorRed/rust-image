//! Placing children inside a component.
//!
//! There is one kind of component, and every component can hold children. How it places them is a setting on it, like
//! `display` in CSS: [`Display::Stack`] puts them on top of each other (the default), [`Display::Flex`] in a row or a
//! column, and [`Display::Grid`] in cells. A child says how much room it wants with `.width(..)`, `.height(..)`, `.fill()`,
//! `.grow(..)` and `.span(..)`; a child that says nothing fills.
//!
//! The rules follow CSS closely enough to be familiar, with one difference: components have no size of their own (nothing
//! to measure), so a child either has a fixed size or fills.

use std::collections::HashSet;

use crate::theme::{Background, Radius};

/// Space on each side of a box, in pixels: the padding inside it, or the margin around it. Build one with [`Edges::all`],
/// [`Edges::new`] or from a number or an array, in the order CSS uses: `8` is all four sides, `[8, 16]` is up and down then
/// left and right, and `[1, 2, 3, 4]` is top, right, bottom, left.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges {
  /// Space above.
  pub top: u32,
  /// Space to the right.
  pub right: u32,
  /// Space below.
  pub bottom: u32,
  /// Space to the left.
  pub left: u32,
}

impl Edges {
  /// The same space on all four sides.
  pub fn all(p_pixels: u32) -> Self {
    Self::new(p_pixels, p_pixels, p_pixels, p_pixels)
  }

  /// A different space on each side.
  pub fn new(p_top: u32, p_right: u32, p_bottom: u32, p_left: u32) -> Self {
    Self {
      top: p_top,
      right: p_right,
      bottom: p_bottom,
      left: p_left,
    }
  }

  /// Left and right together.
  pub(crate) fn horizontal(&self) -> u32 {
    self.left + self.right
  }

  /// Top and bottom together.
  pub(crate) fn vertical(&self) -> u32 {
    self.top + self.bottom
  }
}

impl From<u32> for Edges {
  fn from(p_pixels: u32) -> Self {
    Self::all(p_pixels)
  }
}

impl From<[u32; 2]> for Edges {
  /// Up and down, then left and right.
  fn from(p_sides: [u32; 2]) -> Self {
    Self::new(p_sides[0], p_sides[1], p_sides[0], p_sides[1])
  }
}

impl From<[u32; 4]> for Edges {
  /// Top, right, bottom, left.
  fn from(p_sides: [u32; 4]) -> Self {
    Self::new(p_sides[0], p_sides[1], p_sides[2], p_sides[3])
  }
}

/// A rectangle in pixels, measured from its parent's top left corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
  /// Distance from the parent's left edge.
  pub x: i32,
  /// Distance from the parent's top edge.
  pub y: i32,
  /// Width in pixels.
  pub width: u32,
  /// Height in pixels.
  pub height: u32,
}

impl Rect {
  /// Whether the point (`p_x`, `p_y`) is inside the rectangle.
  pub fn contains(&self, p_x: f32, p_y: f32) -> bool {
    p_x >= self.x as f32
      && p_y >= self.y as f32
      && p_x < self.x as f32 + self.width as f32
      && p_y < self.y as f32 + self.height as f32
  }
}

/// How a component places its children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Display {
  /// Every child covers the component (or is its own fixed size) at the top left, later children in front.
  #[default]
  Stack,
  /// Children in a line, along the component's [`Direction`], like CSS `display: flex`.
  Flex,
  /// Children in cells, placed by the component's columns and rows, like CSS `display: grid`.
  Grid,
}

/// Which way a flex component lays its children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
  /// Left to right.
  #[default]
  Row,
  /// Top to bottom.
  Column,
}

/// Where a child with a fixed size sits across its line (flex) or in its cell (grid, up and down), like CSS `align-items`.
/// A child that fills always fills the whole line or cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
  /// At the start.
  #[default]
  Start,
  /// At the end.
  End,
  /// In the middle.
  Center,
  /// Same as `Start` for a fixed size; a child that fills is stretched whatever this is.
  Stretch,
}

/// How spare room along a flex line is shared out, like CSS `justify-content`. In a grid, `Start`, `End` and `Center`
/// place a fixed-size child left and right in its cell, and the other values act as `Start`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justify {
  /// Children packed at the start.
  #[default]
  Start,
  /// Children packed at the end.
  End,
  /// Children packed in the middle.
  Center,
  /// The first child at the start, the last at the end, equal gaps between.
  SpaceBetween,
  /// Equal room around every child.
  SpaceAround,
  /// Equal gaps between children and at both ends.
  SpaceEvenly,
}

/// The size of one grid column or row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Track {
  /// Exactly this many pixels.
  Px(u32),
  /// A share of the room that is left, like CSS `fr`: `Fr(2.0)` gets twice as much as `Fr(1.0)`.
  Fr(f32),
}

/// How much room a child wants along one axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Length {
  /// As much as is left over, shared with the other children that fill.
  Fill,
  /// Exactly this many pixels, or less if the component has less.
  Fixed(u32),
}

/// What a child asks of its parent's layout, set on the child by whoever uses it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hints {
  pub width: Length,
  pub height: Length,
  /// The share of the spare room along a flex line a child that fills gets, like CSS `flex-grow`.
  pub grow: f32,
  /// How many grid columns and rows the child covers.
  pub span: (u16, u16),
  /// The margin around the child, which is the child's own setting and is copied here by the parent.
  pub margin: Edges,
}

impl Default for Hints {
  fn default() -> Self {
    Self {
      width: Length::Fill,
      height: Length::Fill,
      grow: 1.0,
      span: (1, 1),
      margin: Edges::default(),
    }
  }
}

/// How a component places its children.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Container {
  pub display: Display,
  pub direction: Direction,
  pub wrap: bool,
  pub gap: u32,
  pub padding: Edges,
  pub margin: Edges,
  pub border_width: u32,
  pub border_color: Option<[u8; 4]>,
  pub radius: Radius,
  pub background: Background,
  pub justify: Justify,
  pub align: Align,
  pub columns: Vec<Track>,
  pub rows: Vec<Track>,
}

/// Where each child goes in a component of `p_size`.
pub(crate) fn layout(p_container: &Container, p_size: (u32, u32), p_hints: &[Hints]) -> Vec<Rect> {
  // The border and the padding are room kept clear all the way round; what is left is where children go.
  let border = p_container.border_width;
  let (left, top) = (p_container.padding.left + border, p_container.padding.top + border);
  let inner = Rect {
    x: left as i32,
    y: top as i32,
    width: p_size.0.saturating_sub(left + p_container.padding.right + border),
    height: p_size.1.saturating_sub(top + p_container.padding.bottom + border),
  };
  match p_container.display {
    Display::Stack => p_hints.iter().map(|hints| stack_child(inner, hints)).collect(),
    Display::Flex => flex(p_container, inner, p_hints),
    Display::Grid => grid(p_container, inner, p_hints),
  }
}

/// Splits `p_total` into parts in proportion to `p_weights`, so the parts add up to exactly `p_total` and no pixel is
/// lost to rounding. All-zero weights give all-zero parts.
fn shares(p_total: u32, p_weights: &[f32]) -> Vec<u32> {
  let sum: f64 = p_weights.iter().map(|weight| weight.max(0.0) as f64).sum();
  if sum <= 0.0 {
    return vec![0; p_weights.len()];
  }
  let (mut taken, mut running) = (0u32, 0f64);
  p_weights
    .iter()
    .map(|weight| {
      running += weight.max(0.0) as f64;
      let edge = ((p_total as f64 * running / sum).round() as u32).clamp(taken, p_total);
      let part = edge - taken;
      taken = edge;
      part
    })
    .collect()
}

/// Where a child of `p_length` goes inside `p_room` pixels starting at `p_start`, as (start, size).
fn place(p_length: Length, p_start: u32, p_room: u32, p_at: Align) -> (u32, u32) {
  match p_length {
    Length::Fill => (p_start, p_room),
    Length::Fixed(pixels) => {
      let size = pixels.min(p_room);
      let spare = p_room - size;
      let offset = match p_at {
        Align::Start | Align::Stretch => 0,
        Align::End => spare,
        Align::Center => spare / 2,
      };
      (p_start + offset, size)
    }
  }
}

fn stack_child(p_inner: Rect, p_hints: &Hints) -> Rect {
  let margin = p_hints.margin;
  let (x, width) = place(
    p_hints.width,
    p_inner.x as u32 + margin.left,
    p_inner.width.saturating_sub(margin.horizontal()),
    Align::Start,
  );
  let (y, height) = place(
    p_hints.height,
    p_inner.y as u32 + margin.top,
    p_inner.height.saturating_sub(margin.vertical()),
    Align::Start,
  );
  Rect {
    x: x as i32,
    y: y as i32,
    width,
    height,
  }
}

/// What flex needs to know about one child, in terms of "along the line" and "across it".
struct Item {
  along: Length,
  across: Length,
  grow: f32,
  /// The margins before and after it along the line.
  along_margin: (u32, u32),
  /// The margins before and after it across the line.
  across_margin: (u32, u32),
}

impl Item {
  /// The room it takes along the line before any spare room is shared: its margins and, if it has a fixed size, that.
  fn committed(&self) -> u32 {
    self.along_margin.0 + self.along_margin.1 + if let Length::Fixed(pixels) = self.along { pixels } else { 0 }
  }

  /// The room it needs across the line: its margins and, if it has a fixed size, that.
  fn across_needed(&self) -> u32 {
    self.across_margin.0 + self.across_margin.1 + if let Length::Fixed(pixels) = self.across { pixels } else { 0 }
  }
}

/// Children in lines. The code works in "along the line" and "across it" and turns that into x and y at the end, so a row
/// and a column are the same algorithm. A child's margins are part of the room it takes, and the rectangle it gets is
/// what is left inside them.
fn flex(p_container: &Container, p_inner: Rect, p_hints: &[Hints]) -> Vec<Rect> {
  let row = p_container.direction == Direction::Row;
  let gap = p_container.gap;
  let (along_start, across_start) = if row { (p_inner.x, p_inner.y) } else { (p_inner.y, p_inner.x) };
  let (along_room, across_room) = if row { (p_inner.width, p_inner.height) } else { (p_inner.height, p_inner.width) };
  let items: Vec<Item> = p_hints
    .iter()
    .map(|hints| {
      let margin = hints.margin;
      if row {
        Item {
          along: hints.width,
          across: hints.height,
          grow: hints.grow,
          along_margin: (margin.left, margin.right),
          across_margin: (margin.top, margin.bottom),
        }
      } else {
        Item {
          along: hints.height,
          across: hints.width,
          grow: hints.grow,
          along_margin: (margin.top, margin.bottom),
          across_margin: (margin.left, margin.right),
        }
      }
    })
    .collect();
  if items.is_empty() {
    return Vec::new();
  }

  // Break into lines: all in one, unless wrapping, when a child that would run past the end starts the next line. A child
  // that fills starts from nothing but its margins, so it hardly ever causes a break.
  let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
  let mut used = 0u32;
  for (index, item) in items.iter().enumerate() {
    let line_has_children = lines.last().is_some_and(|line| !line.is_empty());
    let needed = if line_has_children { used + gap + item.committed() } else { item.committed() };
    if p_container.wrap && line_has_children && needed > along_room {
      lines.push(vec![index]);
      used = item.committed();
    } else {
      lines.last_mut().expect("there is always a line").push(index);
      used = needed;
    }
  }

  // How tall each line is. One line takes all the room. Wrapped lines are as tall as their tallest fixed child, and lines
  // with no fixed child share what is left.
  let line_across: Vec<u32> = if p_container.wrap {
    let mut heights: Vec<u32> = lines
      .iter()
      .map(|line| line.iter().map(|&index| items[index].across_needed()).max().unwrap_or(0).min(across_room))
      .collect();
    let taken: u32 = heights.iter().sum::<u32>() + gap * (heights.len() as u32 - 1);
    let empty: Vec<usize> = (0..heights.len()).filter(|&line| heights[line] == 0).collect();
    for (line, extra) in empty.iter().zip(shares(across_room.saturating_sub(taken), &vec![1.0; empty.len()])) {
      heights[*line] = extra;
    }
    heights
  } else {
    vec![across_room]
  };

  let mut rects = vec![
    Rect {
      x: 0,
      y: 0,
      width: 0,
      height: 0
    };
    items.len()
  ];
  let mut across_at = across_start as u32;
  for (line, &line_height) in lines.iter().zip(&line_across) {
    let count = line.len();
    let room = along_room.saturating_sub(gap * (count as u32 - 1));
    let committed: Vec<u32> = line.iter().map(|&index| items[index].committed()).collect();
    let fixed: u32 = committed.iter().sum();

    // The room each child takes along the line, margins included: what it is committed to, shrunk together if it does not
    // all fit, or grown for the children that fill, which share what the others leave.
    let weights_committed: Vec<f32> = committed.iter().map(|room| *room as f32).collect();
    let weights_fill: Vec<f32> =
      line.iter().map(|&index| if items[index].along == Length::Fill { items[index].grow } else { 0.0 }).collect();
    let mut spare = 0;
    let outer: Vec<u32> = if fixed > room {
      shares(room, &weights_committed)
    } else if weights_fill.iter().sum::<f32>() > 0.0 {
      let grown = shares(room - fixed, &weights_fill);
      committed.iter().zip(grown).map(|(committed, grown)| committed + grown).collect()
    } else {
      spare = room - fixed;
      committed.clone()
    };

    // The room before each child (and after the last), from `justify`.
    let before: Vec<u32> = match p_container.justify {
      Justify::Start => vec![0; count + 1],
      Justify::End => std::iter::once(spare).chain(vec![0; count]).collect(),
      Justify::Center => std::iter::once(spare / 2).chain(vec![0; count]).collect(),
      Justify::SpaceBetween if count > 1 => {
        std::iter::once(0).chain(shares(spare, &vec![1.0; count - 1])).chain(std::iter::once(0)).collect()
      }
      Justify::SpaceBetween => vec![0; count + 1],
      Justify::SpaceAround => {
        let mut weights = vec![1.0; count + 1];
        weights[0] = 0.5;
        weights[count] = 0.5;
        shares(spare, &weights)
      }
      Justify::SpaceEvenly => shares(spare, &vec![1.0; count + 1]),
    };

    let mut along_at = along_start as u32 + before[0];
    for (position, &index) in line.iter().enumerate() {
      let item = &items[index];
      // Inside its margins.
      let along = along_at + item.along_margin.0;
      let along_size = outer[position].saturating_sub(item.along_margin.0 + item.along_margin.1);
      let across_room_inside = line_height.saturating_sub(item.across_margin.0 + item.across_margin.1);
      let (across, across_size) =
        place(item.across, across_at + item.across_margin.0, across_room_inside, p_container.align);
      rects[index] = if row {
        Rect {
          x: along as i32,
          y: across as i32,
          width: along_size,
          height: across_size,
        }
      } else {
        Rect {
          x: across as i32,
          y: along as i32,
          width: across_size,
          height: along_size,
        }
      };
      along_at += outer[position] + gap + before[position + 1];
    }
    across_at += line_height + gap;
  }
  rects
}

/// The sizes of a grid's columns or rows in `p_room` pixels: pixel tracks as asked, or shrunk together if they do not fit,
/// and `fr` tracks sharing what is left.
fn track_sizes(p_tracks: &[Track], p_room: u32, p_gap: u32) -> Vec<u32> {
  let room = p_room.saturating_sub(p_gap * (p_tracks.len() as u32).saturating_sub(1));
  let pixels = |p_track: &Track| if let Track::Px(pixels) = p_track { *pixels } else { 0 };
  let fixed: u32 = p_tracks.iter().map(pixels).sum();
  if fixed > room {
    return shares(room, &p_tracks.iter().map(|track| pixels(track) as f32).collect::<Vec<_>>());
  }
  let weights: Vec<f32> =
    p_tracks.iter().map(|track| if let Track::Fr(share) = track { *share } else { 0.0 }).collect();
  let grown = shares(room - fixed, &weights);
  p_tracks.iter().zip(grown).map(|(track, grown)| pixels(track) + grown).collect()
}

/// Children in cells. Each is put in the first free place, left to right and then down, covering as many cells as it spans.
/// Columns and rows that were not defined are made as needed and share the room equally.
fn grid(p_container: &Container, p_inner: Rect, p_hints: &[Hints]) -> Vec<Rect> {
  let columns = if p_container.columns.is_empty() { vec![Track::Fr(1.0)] } else { p_container.columns.clone() };

  let mut taken: HashSet<(usize, usize)> = HashSet::new();
  let (mut row, mut column) = (0usize, 0usize);
  let mut cells = Vec::with_capacity(p_hints.len());
  for hints in p_hints {
    let across = (hints.span.0.max(1) as usize).min(columns.len());
    let down = hints.span.1.max(1) as usize;
    let free = |row: usize, column: usize| {
      column + across <= columns.len()
        && (row..row + down).all(|r| (column..column + across).all(|c| !taken.contains(&(r, c))))
    };
    while !free(row, column) {
      column += 1;
      if column >= columns.len() {
        column = 0;
        row += 1;
      }
    }
    for r in row..row + down {
      for c in column..column + across {
        taken.insert((r, c));
      }
    }
    cells.push((column, row, across, down));
    column += across;
    if column >= columns.len() {
      column = 0;
      row += 1;
    }
  }

  let used_rows = cells.iter().map(|(_, row, _, down)| row + down).max().unwrap_or(0);
  let mut rows = p_container.rows.clone();
  rows.resize(rows.len().max(used_rows), Track::Fr(1.0));

  let gap = p_container.gap;
  let widths = track_sizes(&columns, p_inner.width, gap);
  let heights = track_sizes(&rows, p_inner.height, gap);
  let starts = |p_sizes: &[u32], p_origin: i32| -> Vec<u32> {
    let mut at = p_origin as u32;
    p_sizes
      .iter()
      .map(|size| {
        let start = at;
        at += size + gap;
        start
      })
      .collect()
  };
  let (left, top) = (starts(&widths, p_inner.x), starts(&heights, p_inner.y));
  let covering = |p_sizes: &[u32], p_first: usize, p_count: usize| -> u32 {
    p_sizes[p_first..p_first + p_count].iter().sum::<u32>() + gap * (p_count as u32 - 1)
  };
  let across_align = match p_container.justify {
    Justify::End => Align::End,
    Justify::Center => Align::Center,
    _ => Align::Start,
  };

  p_hints
    .iter()
    .zip(&cells)
    .map(|(hints, &(column, row, across, down))| {
      let margin = hints.margin;
      let (cell_width, cell_height) = (covering(&widths, column, across), covering(&heights, row, down));
      let (x, width) =
        place(hints.width, left[column] + margin.left, cell_width.saturating_sub(margin.horizontal()), across_align);
      let (y, height) =
        place(hints.height, top[row] + margin.top, cell_height.saturating_sub(margin.vertical()), p_container.align);
      Rect {
        x: x as i32,
        y: y as i32,
        width,
        height,
      }
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  const FILL: Hints = Hints {
    width: Length::Fill,
    height: Length::Fill,
    grow: 1.0,
    span: (1, 1),
    margin: Edges {
      top: 0,
      right: 0,
      bottom: 0,
      left: 0,
    },
  };

  fn fixed(p_width: u32, p_height: u32) -> Hints {
    Hints {
      width: Length::Fixed(p_width),
      height: Length::Fixed(p_height),
      ..FILL
    }
  }

  fn rect(p_x: i32, p_y: i32, p_width: u32, p_height: u32) -> Rect {
    Rect {
      x: p_x,
      y: p_y,
      width: p_width,
      height: p_height,
    }
  }

  fn flex(p_direction: Direction) -> Container {
    Container {
      display: Display::Flex,
      direction: p_direction,
      ..Container::default()
    }
  }

  fn grid(p_columns: Vec<Track>, p_rows: Vec<Track>) -> Container {
    Container {
      display: Display::Grid,
      columns: p_columns,
      rows: p_rows,
      ..Container::default()
    }
  }

  // --- stack

  #[test]
  fn a_stack_puts_every_child_at_the_top_left() {
    let rects = layout(&Container::default(), (100, 50), &[FILL, fixed(20, 10)]);
    assert_eq!(rects, vec![rect(0, 0, 100, 50), rect(0, 0, 20, 10)]);
  }

  #[test]
  fn a_stack_leaves_room_for_its_padding() {
    let container = Container {
      padding: Edges::all(5),
      ..Container::default()
    };
    assert_eq!(layout(&container, (100, 50), &[FILL]), vec![rect(5, 5, 90, 40)]);
  }

  // --- flex

  #[test]
  fn a_flex_row_splits_its_width_equally_among_children_that_fill() {
    let rects = layout(&flex(Direction::Row), (100, 40), &[FILL, FILL]);
    assert_eq!(rects, vec![rect(0, 0, 50, 40), rect(50, 0, 50, 40)]);
  }

  #[test]
  fn a_flex_row_gives_fixed_widths_first_and_shares_the_rest() {
    let rects = layout(&flex(Direction::Row), (100, 40), &[fixed(30, 10), FILL, FILL]);
    assert_eq!(rects, vec![rect(0, 0, 30, 10), rect(30, 0, 35, 40), rect(65, 0, 35, 40)]);
  }

  #[test]
  fn the_shares_of_the_room_that_is_left_follow_grow() {
    let wide = Hints { grow: 3.0, ..FILL };
    let rects = layout(&flex(Direction::Row), (100, 10), &[FILL, wide]);
    assert_eq!(rects.iter().map(|rect| rect.width).collect::<Vec<_>>(), vec![25, 75]);
  }

  #[test]
  fn pixels_that_do_not_divide_evenly_are_still_all_used() {
    let rects = layout(&flex(Direction::Row), (101, 10), &[FILL, FILL, FILL]);
    assert_eq!(rects.iter().map(|rect| rect.width).sum::<u32>(), 101);
    assert_eq!(rects[0].x, 0);
    assert_eq!(rects[2].x as u32 + rects[2].width, 101);
  }

  #[test]
  fn a_flex_column_is_a_row_turned_on_its_side() {
    let rects = layout(&flex(Direction::Column), (40, 100), &[fixed(10, 30), FILL]);
    assert_eq!(rects, vec![rect(0, 0, 10, 30), rect(0, 30, 40, 70)]);
  }

  #[test]
  fn fixed_sizes_larger_than_the_room_shrink_to_fit() {
    let rects = layout(&flex(Direction::Row), (50, 20), &[fixed(40, 5), fixed(40, 5)]);
    assert_eq!(rects, vec![rect(0, 0, 25, 5), rect(25, 0, 25, 5)]);
  }

  #[test]
  fn gap_and_padding_make_room_between_and_around() {
    let container = Container {
      gap: 10,
      padding: Edges::all(5),
      ..flex(Direction::Row)
    };
    let rects = layout(&container, (110, 30), &[FILL, FILL]);
    assert_eq!(rects, vec![rect(5, 5, 45, 20), rect(60, 5, 45, 20)]);
  }

  #[test]
  fn justify_shares_out_spare_room_along_the_line() {
    let hints = [fixed(10, 10), fixed(10, 10)];
    let positions = |p_justify| {
      let container = Container {
        justify: p_justify,
        ..flex(Direction::Row)
      };
      layout(&container, (100, 10), &hints).iter().map(|rect| rect.x).collect::<Vec<_>>()
    };
    assert_eq!(positions(Justify::Start), vec![0, 10]);
    assert_eq!(positions(Justify::End), vec![80, 90]);
    assert_eq!(positions(Justify::Center), vec![40, 50]);
    assert_eq!(positions(Justify::SpaceBetween), vec![0, 90]);
  }

  #[test]
  fn align_places_children_across_the_line_but_a_child_that_fills_still_stretches() {
    let container = Container {
      align: Align::Center,
      ..flex(Direction::Row)
    };
    let fills_across = Hints {
      width: Length::Fixed(10),
      ..FILL
    };
    let rects = layout(&container, (100, 40), &[fixed(10, 10), fills_across]);
    assert_eq!(rects, vec![rect(0, 15, 10, 10), rect(10, 0, 10, 40)]);
  }

  #[test]
  fn a_wrapping_row_moves_children_that_do_not_fit_to_the_next_line() {
    let container = Container {
      wrap: true,
      ..flex(Direction::Row)
    };
    let rects = layout(&container, (50, 40), &[fixed(30, 10), fixed(30, 10)]);
    assert_eq!((rects[0].x, rects[0].y), (0, 0));
    assert_eq!(rects[1].x, 0);
    assert!(rects[1].y >= 10, "the second child is on the next line");
  }

  // --- margins

  fn margin(p_hints: Hints, p_margin: Edges) -> Hints {
    Hints {
      margin: p_margin,
      ..p_hints
    }
  }

  #[test]
  fn a_fixed_child_keeps_its_margins_clear_and_the_next_child_starts_after_them() {
    let rects = layout(&flex(Direction::Row), (100, 40), &[margin(fixed(30, 10), Edges::all(5)), FILL]);
    assert_eq!(rects, vec![rect(5, 5, 30, 10), rect(40, 0, 60, 40)]);
  }

  #[test]
  fn children_that_fill_share_the_room_left_after_their_margins() {
    let sides = Edges::new(0, 5, 0, 5);
    let rects = layout(&flex(Direction::Row), (100, 40), &[margin(FILL, sides), margin(FILL, sides)]);
    assert_eq!(rects, vec![rect(5, 0, 40, 40), rect(55, 0, 40, 40)]);
  }

  #[test]
  fn margins_work_the_same_down_a_column_and_across_it() {
    let rects = layout(&flex(Direction::Column), (40, 100), &[margin(FILL, Edges::new(10, 4, 6, 2)), FILL]);
    assert_eq!(
      rects[0],
      rect(2, 10, 34, 42),
      "the 84 pixels left after its margins are shared, and its margins come on top"
    );
    assert_eq!(rects[1].y, 58, "so the second starts after the first one's outer box of 16 + 42");
  }

  #[test]
  fn a_stack_child_sits_inside_its_margin() {
    let rects =
      layout(&Container::default(), (100, 50), &[margin(FILL, Edges::all(4)), margin(fixed(20, 10), [2, 3].into())]);
    assert_eq!(rects, vec![rect(4, 4, 92, 42), rect(3, 2, 20, 10)]);
  }

  #[test]
  fn a_grid_child_sits_inside_its_margin_in_its_cell() {
    let container = grid(vec![Track::Px(50), Track::Px(50)], vec![Track::Px(20)]);
    let rects = layout(&container, (100, 20), &[FILL, margin(FILL, Edges::all(5))]);
    assert_eq!(rects, vec![rect(0, 0, 50, 20), rect(55, 5, 40, 10)]);
  }

  #[test]
  fn padding_and_border_leave_room_on_each_side_for_the_children() {
    let container = Container {
      padding: Edges::new(1, 2, 3, 4),
      border_width: 2,
      ..Container::default()
    };
    assert_eq!(layout(&container, (50, 40), &[FILL]), vec![rect(6, 3, 40, 32)]);
  }

  #[test]
  fn edges_come_from_a_number_or_css_ordered_arrays() {
    assert_eq!(Edges::from(3), Edges::new(3, 3, 3, 3));
    assert_eq!(Edges::from([1, 2]), Edges::new(1, 2, 1, 2));
    assert_eq!(Edges::from([1, 2, 3, 4]), Edges::new(1, 2, 3, 4));
  }

  // --- grid

  #[test]
  fn a_grid_places_children_in_cells_by_its_tracks() {
    let container = grid(vec![Track::Px(20), Track::Fr(1.0), Track::Fr(3.0)], vec![Track::Px(10), Track::Fr(1.0)]);
    let rects = layout(&container, (100, 40), &[FILL, FILL, FILL, FILL, FILL, FILL]);
    assert_eq!(rects[0], rect(0, 0, 20, 10));
    assert_eq!(rects[1], rect(20, 0, 20, 10));
    assert_eq!(rects[2], rect(40, 0, 60, 10));
    assert_eq!(rects[3], rect(0, 10, 20, 30));
    assert_eq!(rects[5], rect(40, 10, 60, 30));
  }

  #[test]
  fn a_child_can_span_several_cells() {
    let container = grid(vec![Track::Fr(1.0), Track::Fr(1.0), Track::Fr(1.0)], vec![Track::Px(10), Track::Px(10)]);
    let wide = Hints { span: (2, 1), ..FILL };
    let rects = layout(&container, (90, 20), &[wide, FILL, FILL]);
    assert_eq!(rects[0], rect(0, 0, 60, 10));
    assert_eq!(rects[1], rect(60, 0, 30, 10));
    assert_eq!(rects[2], rect(0, 10, 30, 10));
  }

  #[test]
  fn a_grid_with_no_tracks_shares_its_room_equally() {
    let rects = layout(&grid(vec![], vec![]), (100, 40), &[FILL, FILL]);
    assert_eq!(rects, vec![rect(0, 0, 100, 20), rect(0, 20, 100, 20)]);
  }

  #[test]
  fn no_children_means_no_rectangles() {
    assert!(layout(&flex(Direction::Row), (10, 10), &[]).is_empty());
    assert!(layout(&grid(vec![], vec![]), (10, 10), &[]).is_empty());
  }

  #[test]
  fn a_rect_contains_its_top_left_edge_but_not_its_bottom_right_edge() {
    let rect = rect(10, 20, 5, 5);
    assert!(rect.contains(10.0, 20.0));
    assert!(rect.contains(14.9, 24.9));
    assert!(!rect.contains(15.0, 22.0));
    assert!(!rect.contains(12.0, 25.0));
    assert!(!rect.contains(9.9, 22.0));
  }
}
