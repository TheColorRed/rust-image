use std::sync::{Arc, Mutex};

use abra::abra_core::{Color, Curve, Image as Pixels};
use vessel::prelude::*;

use crate::components::surface::{PictureSource, Shared, blank, color_of, disc, line, rect, ring};

/// Room left round the plot, so a point on its edge can be seen and taken.
const INSET: f32 = 14.0;
/// How close a finger has to land to a point to take it instead of making a new one.
const GRAB: f32 = 24.0;
const POINT_RADIUS: f32 = 12.0;
/// How many grid lines there are across and down.
const GRID: u32 = 4;

/// The colors a plot is drawn in.
#[derive(Clone, Copy)]
struct Look {
  background: Color,
  grid: Color,
  line: Color,
  point: Color,
  selected: Color,
}

/// What the plot shows: the points of the curve, each `(x, y)` with `x` from 0 (the start) to 1 (the end) and `y` from `min` to `max`, in the order
/// they were made, and the one that is selected.
#[derive(Clone)]
struct State {
  points: Vec<(f32, f32)>,
  selected: Option<usize>,
  min: f32,
  max: f32,
  look: Look,
  density: f32,
}

/// Where the plot is on the surface, and what its sides mean.
#[derive(Clone, Copy)]
struct Plot {
  left: f32,
  top: f32,
  width: f32,
  height: f32,
  min: f32,
  max: f32,
}

impl Plot {
  /// The plot on a surface of `p_size`, which leaves `INSET` clear all round.
  fn new(p_size: (u32, u32), p_min: f32, p_max: f32) -> Self {
    Self {
      left: INSET,
      top: INSET,
      width: (p_size.0 as f32 - INSET * 2.0).max(1.0),
      height: (p_size.1 as f32 - INSET * 2.0).max(1.0),
      min: p_min,
      max: p_max,
    }
  }

  /// The place on the surface of a point.
  fn to_pixel(&self, p_point: (f32, f32)) -> (f32, f32) {
    (self.left + p_point.0 * self.width, self.top + (1.0 - (p_point.1 - self.min) / (self.max - self.min)) * self.height)
  }

  /// The point at a place on the surface, held inside the plot.
  fn to_point(&self, p_pixel: (f32, f32)) -> (f32, f32) {
    (
      ((p_pixel.0 - self.left) / self.width).clamp(0.0, 1.0),
      (self.min + (1.0 - (p_pixel.1 - self.top) / self.height) * (self.max - self.min)).clamp(self.min, self.max),
    )
  }
}

/// The picture of the plot: abra's curve through the points, stroked on a grid, with a mark for each point.
fn paint(p_state: &State, p_size: (u32, u32)) -> Pixels {
  let State { points, selected, min, max, look, density } = p_state;
  let plot = Plot::new(p_size, *min, *max);
  let mut image = blank(p_size);
  rect(&mut image, (0.0, 0.0), (p_size.0 as f32, p_size.1 as f32), look.background);
  for step in 0..=GRID {
    let share = step as f32 / GRID as f32;
    let (across, down) = (plot.left + plot.width * share, plot.top + plot.height * share);
    line(&mut image, &[(across, plot.top), (across, plot.top + plot.height)], 1.0, look.grid);
    line(&mut image, &[(plot.left, down), (plot.left + plot.width, down)], 1.0, look.grid);
  }

  // The library's curve, made in the pixels of the plot (it has the same shape there) and flattened into the line abra strokes. It is flat before its
  // first point and after its last, which is drawn as two straight lines.
  let pixels: Vec<(f32, f32)> = points.iter().map(|point| plot.to_pixel(*point)).collect();
  let curve = Curve::new(pixels.iter().copied());
  line(&mut image, &curve.flatten(0.25).iter().map(|point| (point.x, point.y)).collect::<Vec<_>>(), 2.5, look.line);
  if let (Some(first), Some(last)) = (pixels.iter().min_by(|a, b| a.0.total_cmp(&b.0)), pixels.iter().max_by(|a, b| a.0.total_cmp(&b.0))) {
    line(&mut image, &[(plot.left, first.1), *first], 2.5, look.line);
    line(&mut image, &[*last, (plot.left + plot.width, last.1)], 2.5, look.line);
  }

  for (index, point) in pixels.iter().enumerate() {
    if Some(index) == *selected {
      disc(&mut image, *point, POINT_RADIUS * density, look.selected);
      ring(&mut image, *point, POINT_RADIUS * density, 2.0 * density, Color::white());
    } else {
      disc(&mut image, *point, POINT_RADIUS * density, Color::white());
      ring(&mut image, *point, (POINT_RADIUS - 1.0) * density, 2.0 * density, look.point);
    }
  }
  image
}

/// A point the finger has hold of.
#[derive(Clone, Copy)]
struct Held {
  index: usize,
  /// Where the point was, and where the finger was, when it was taken: the point moves by how far the finger has gone since, so it does not jump to the finger.
  point: (f32, f32),
  finger: (f32, f32),
}

/// The curve editor's surface: a curve drawn on a grid, with its points to drag. A finger that lands on a point takes it and moves it, and one that
/// lands on an empty place makes a point there and moves that. The line is the library's own curve through the points, so it is the line a plugin makes
/// from the same points. abra draws the picture and Vessel renders it and brings the touches; the buttons and the fields (a point's time and value,
/// remove, OK) are the app's.
///
/// Read `points` and `selected` while the finger moves and when it lifts, to show the fields and to tell the plugin; the app changes them with
/// `set_points` and `set_selected`.
#[derive(uniffi::Object, Component)]
pub struct CurveEditor {
  image: Image,
  shared: Arc<Shared<State>>,
}

#[uniffi::export]
impl CurveEditor {
  /// An editor of the curve through `points`, which are `x, y` pairs in one list (`x` from 0 to 1 along the curve, `y` from `min` to `max`), with the
  /// first one selected. `colors` are five colors, each as red, green, blue and alpha from 0 to 255: the background, the grid, the line, a point and the selected point. `density` converts the handle's dp measurements to surface pixels.
  ///
  /// # Panics
  /// When there are no points or one has no `y`, when `max` is not above `min`, or when there are fewer than five colors.
  #[uniffi::constructor]
  pub fn new(points: Vec<f64>, min: f64, max: f64, colors: Vec<Vec<u8>>, density: f64) -> Arc<Self> {
    assert!(max > min, "the highest value is above the lowest");
    let (min, max) = (min as f32, max as f32);
    let look = Look {
      background: color_of(&colors[0]),
      grid: color_of(&colors[1]),
      line: color_of(&colors[2]),
      point: color_of(&colors[3]),
      selected: color_of(&colors[4]),
    };
    let shared = Shared::new(State { points: pairs(&points), selected: Some(0), min, max, look, density: (density as f32).max(0.1) });
    let image = Image::new().track(&PictureSource::new(&shared, paint));

    // The finger: where it is now, and the point it has hold of.
    let finger = Mutex::new(((0.0f32, 0.0f32), None::<Held>));
    let this = image.downgrade();
    image.subject::<PointerEvent>().subscribe({
      let shared = Arc::clone(&shared);
      move |event| {
        let Some(image) = this.upgrade() else { return };
        let plot = Plot::new(image.size().value(), min, max);
        let mut finger = finger.lock().unwrap();
        match event {
          PointerEvent::Moved { x, y } => {
            finger.0 = (*x, *y);
            if let Some(held) = finger.1 {
              let start = plot.to_pixel(held.point);
              let moved = plot.to_point((start.0 + (*x - held.finger.0), start.1 + (*y - held.finger.1)));
              shared.update(|state| {
                if let Some(point) = state.points.get_mut(held.index) {
                  *point = moved;
                }
              });
            }
          }
          PointerEvent::Button { button: PointerButton::Left, pressed: true } => {
            let mut taken = None;
            shared.update(|state| {
              let index = nearest(&state.points, &plot, finger.0, GRAB * state.density).unwrap_or_else(|| {
                state.points.push(plot.to_point(finger.0));
                state.points.len() - 1
              });
              state.selected = Some(index);
              taken = Some(Held { index, point: state.points[index], finger: finger.0 });
            });
            finger.1 = taken;
          }
          PointerEvent::Button { button: PointerButton::Left, pressed: false } => finger.1 = None,
          PointerEvent::Button { .. } => {}
        }
      }
    });

    Arc::new(Self { image, shared })
  }

  /// The points now, as `x, y` pairs in one list, in the order they were made.
  pub fn points(&self) -> Vec<f64> {
    self.shared.get().points.iter().flat_map(|(x, y)| [*x as f64, *y as f64]).collect()
  }

  /// The place of the selected point in the order they were made, or -1 when none is selected.
  pub fn selected(&self) -> i32 {
    self.shared.get().selected.map_or(-1, |index| index as i32)
  }

  /// Replaces the points, as when a field was typed in or a point was removed. The selected point stays selected if there is still one at its place.
  ///
  /// # Panics
  /// When there are no points or one has no `y`.
  pub fn set_points(&self, points: Vec<f64>) {
    let points = pairs(&points);
    self.shared.update(|state| {
      state.selected = state.selected.filter(|index| *index < points.len());
      state.points = points;
    });
  }

  /// Selects the point at `index`, or none when it is -1 or past the last.
  pub fn set_selected(&self, index: i32) {
    self.shared.update(|state| state.selected = usize::try_from(index).ok().filter(|index| *index < state.points.len()));
  }
}

/// `x, y` pairs from one list.
///
/// # Panics
/// When the list is empty or has an `x` without a `y`.
fn pairs(p_list: &[f64]) -> Vec<(f32, f32)> {
  assert!(!p_list.is_empty() && p_list.len() % 2 == 0, "a curve needs points, each with an x and a y");
  p_list.chunks_exact(2).map(|pair| (pair[0] as f32, pair[1] as f32)).collect()
}

/// The point closest to a place on the surface, when it is near enough to be taken.
fn nearest(p_points: &[(f32, f32)], p_plot: &Plot, p_at: (f32, f32), p_grab: f32) -> Option<usize> {
  p_points
    .iter()
    .enumerate()
    .map(|(index, point)| {
      let center = p_plot.to_pixel(*point);
      (index, (center.0 - p_at.0).hypot(center.1 - p_at.1))
    })
    .filter(|(_, distance)| *distance <= p_grab)
    .min_by(|a, b| a.1.total_cmp(&b.1))
    .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::components::surface::pixel;

  fn colors() -> Vec<Vec<u8>> {
    vec![vec![30, 30, 30, 255], vec![70, 70, 70, 255], vec![40, 120, 255, 255], vec![40, 120, 255, 255], vec![255, 140, 0, 255]]
  }

  /// An editor 214 x 214 pixels, so the plot is 186 pixels across and down from (14, 14), of the line from (0, 0) to (1, 1) with `min` 0 and `max` 1.
  fn editor() -> Arc<CurveEditor> {
    let editor = CurveEditor::new(vec![0.0, 0.0, 1.0, 1.0], 0.0, 1.0, colors(), 1.0);
    editor.size().next((214, 214));
    editor
  }

  fn touch(p_editor: &CurveEditor, p_x: f32, p_y: f32, p_pressed: Option<bool>) {
    p_editor.next(PointerEvent::Moved { x: p_x, y: p_y });
    if let Some(pressed) = p_pressed {
      p_editor.next(PointerEvent::Button { button: PointerButton::Left, pressed });
    }
  }

  #[test]
  fn a_finger_on_a_point_takes_it_and_moves_it_by_how_far_it_goes() {
    let editor = editor();
    // The point (1, 1) is at the top right of the plot: (200, 14).
    touch(&editor, 205.0, 20.0, Some(true));
    assert_eq!(editor.selected(), 1);
    touch(&editor, 205.0 - 93.0, 20.0 + 93.0, None);
    let points = editor.points();
    assert!((points[2] - 0.5).abs() < 0.01 && (points[3] - 0.5).abs() < 0.01, "{points:?}");
    touch(&editor, 205.0 - 93.0, 20.0 + 93.0, Some(false));
    // Let go: moving the finger now changes nothing.
    touch(&editor, 100.0, 100.0, None);
    assert_eq!(editor.points(), points);
  }

  #[test]
  fn a_finger_on_an_empty_place_makes_a_point_there() {
    let editor = editor();
    // The middle of the plot, which is away from both points.
    touch(&editor, 107.0, 107.0, Some(true));
    let points = editor.points();
    assert_eq!(points.len(), 6);
    assert_eq!(editor.selected(), 2);
    assert!((points[4] - 0.5).abs() < 0.01 && (points[5] - 0.5).abs() < 0.01);
  }

  #[test]
  fn a_point_stays_inside_the_plot_and_the_app_can_set_the_points_and_the_selection() {
    let editor = editor();
    touch(&editor, 200.0, 14.0, Some(true));
    touch(&editor, 900.0, -900.0, None);
    let points = editor.points();
    assert_eq!((points[2], points[3]), (1.0, 1.0), "held at the corner");
    editor.set_points(vec![0.0, 0.2, 0.5, 0.9, 1.0, 0.4]);
    assert_eq!(editor.points().len(), 6);
    editor.set_selected(2);
    assert_eq!(editor.selected(), 2);
    editor.set_selected(-1);
    assert_eq!(editor.selected(), -1);
    editor.set_selected(7);
    assert_eq!(editor.selected(), -1, "past the last point selects none");
  }

  #[test]
  fn abra_draws_the_line_through_the_points_on_a_grid() {
    let editor = editor();
    let image = paint(&editor.shared.get(), (214, 214));
    // The line from the bottom left to the top right of the plot passes through the middle of it, which is lit in the line's blue.
    let middle = pixel(&image, 107, 107);
    assert!(middle[2] > middle[0] + 60, "the middle is on the blue line: {middle:?}");
    // Away from the line and the grid it is the background.
    assert_eq!(&pixel(&image, 80, 50)[..3], &[30, 30, 30]);
    // The selected point (0, 0) is marked at the bottom left of the plot in the selected color.
    let mark = pixel(&image, 14, 200);
    assert!(mark[0] > 200, "{mark:?}");
  }
}
