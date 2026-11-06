use std::sync::{Arc, Mutex};

use abra::abra_core::{Area, Color, Curve, Fill, Image as Pixels, LineCap, Path};
use abra::drawing::prelude::fill;
use vessel::prelude::*;

use crate::components::{color_of, layer};

/// Room left round the plot, so a point on its edge can be seen and taken. A handle is wider than this on a dense screen, and then the room is
/// as wide as a handle's radius.
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

/// How far from the middle of a point its handle reaches: the radius of the disc and the outside of the ring round it, which is a line
/// centered on the disc's edge, and a pixel for the smoothing at the edge.
fn handle_reach(p_density: f32) -> f32 {
  ((POINT_RADIUS + 1.0) * p_density).ceil() + 1.0
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
  /// The plot on a surface of `p_size`, which leaves room all round for a handle at its edge (`INSET`, or a handle's radius if that is more).
  fn new(p_size: (u32, u32), p_min: f32, p_max: f32, p_density: f32) -> Self {
    let inset = INSET.max(handle_reach(p_density));
    Self {
      left: inset,
      top: inset,
      width: (p_size.0 as f32 - inset * 2.0).max(1.0),
      height: (p_size.1 as f32 - inset * 2.0).max(1.0),
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

/// Draws a line through each list of points, `p_width` wide, with round ends. Each line is its own shape, so the picture it covers is the size of
/// that line, not of all of them together.
fn lines(p_image: &mut Pixels, p_lines: &[Vec<(f32, f32)>], p_width: f32, p_color: Color) {
  for line in p_lines {
    let mut path = Path::new();
    for (index, point) in line.iter().enumerate() {
      if index == 0 {
        path.move_to(*point);
      } else {
        path.line_to(*point);
      }
    }
    fill(path.stroke(p_width).with_cap(LineCap::Round), Fill::solid(p_color)).apply(p_image);
  }
}

/// A point the finger has hold of.
#[derive(Clone, Copy)]
struct Held {
  index: usize,
  /// Where the point was, and where the finger was, when it was taken: the point moves by how far the finger has gone since, so it does not jump to the finger.
  point: (f32, f32),
  finger: (f32, f32),
}

/// The curve, its points, and the child for each point's handle. A handle is a small child that is drawn once for how it looks (selected or not)
/// and moved to its point, so dragging a point draws the curve again and moves a child, and never draws a handle again.
struct Editor {
  points: BehaviorSubject<Vec<(f32, f32)>>,
  selected: BehaviorSubject<Option<usize>>,
  size: BehaviorSubject<(u32, u32)>,
  root: WeakComponent,
  look: Look,
  min: f32,
  max: f32,
  density: f32,
  /// One child for each point there has been, in the order of the points. When there are fewer points the extra ones are empty (size nothing)
  /// and wait for the next point, so the list is as long as the most points there have been at once.
  handles: Mutex<Vec<Component>>,
  /// Held while the handles are put in place, so two changes at once cannot leave a handle where the older one put it. Nothing else takes it,
  /// and putting a handle in place never comes back here, so it cannot be waited for in a circle.
  arranging: Mutex<()>,
}

impl Editor {
  /// The plot for a surface of the size it has now.
  fn plot(&self) -> Plot {
    Plot::new(self.size.peek(), self.min, self.max, self.density)
  }

  /// The child that draws the handle of the point at `p_index`: a disc with a ring, filled in the selected color when its point is selected.
  fn handle(&self, p_index: usize) -> Component {
    let (selected, look, density) = (self.selected.clone(), self.look, self.density);
    layer("curve-handle", move |size| {
      let mut image = Pixels::new(size.0, size.1);
      let middle = (size.0 as f32 / 2.0, size.1 as f32 / 2.0);
      if selected.value() == Some(p_index) {
        fill(Area::circle(middle, POINT_RADIUS * density), Fill::solid(look.selected)).apply(&mut image);
        fill(Area::circle(middle, POINT_RADIUS * density).stroke(2.0 * density), Fill::solid(Color::white())).apply(&mut image);
      } else {
        fill(Area::circle(middle, POINT_RADIUS * density), Fill::solid(Color::white())).apply(&mut image);
        fill(Area::circle(middle, (POINT_RADIUS - 1.0) * density).stroke(2.0 * density), Fill::solid(look.point)).apply(&mut image);
      }
      image
    })
  }

  /// Puts a handle on every point, making the children it needs.
  fn arrange(&self) {
    let Some(root) = self.root.upgrade() else { return };
    let _arranging = self.arranging.lock().unwrap();
    let points = self.points.peek();
    let plot = self.plot();
    let (handles, added) = {
      let mut handles = self.handles.lock().unwrap();
      let before = handles.len();
      while handles.len() < points.len() {
        let handle = self.handle(handles.len());
        handles.push(handle);
      }
      (handles.clone(), handles[before..].to_vec())
    };
    for handle in &added {
      root.add(handle);
    }
    let reach = handle_reach(self.density);
    for (index, handle) in handles.iter().enumerate() {
      match points.get(index) {
        Some(point) => {
          let (x, y) = plot.to_pixel(*point);
          handle.place((x - reach).round() as u32, (y - reach).round() as u32, (reach * 2.0) as u32, (reach * 2.0) as u32);
        }
        None => handle.place(0, 0, 0, 0),
      }
    }
  }
}

/// The curve editor's component: a curve drawn on a grid, with its points to drag. A finger that lands on a point takes it and moves it, and one that
/// lands on an empty place makes a point there and moves that. The line is the library's own curve through the points, so it is the line a plugin makes
/// from the same points. It is made of Vessel children: the background and grid, which are drawn again only when the size changes, the curve, which
/// is drawn again when a point moves, and a small child for each point's handle, which is only moved. abra draws the pictures; the buttons and the
/// fields (a point's time and value, remove, OK) are the app's.
///
/// Read `points` and `selected` while the finger moves and when it lifts, to show the fields and to tell the plugin; the app changes them with
/// `set_points` and `set_selected`.
#[derive(uniffi::Object, Component)]
pub struct CurveEditor {
  root: Container,
  editor: Arc<Editor>,
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
    let density = (density as f32).max(0.1);
    let look = Look {
      background: color_of(&colors[0]),
      grid: color_of(&colors[1]),
      line: color_of(&colors[2]),
      point: color_of(&colors[3]),
      selected: color_of(&colors[4]),
    };
    let points = BehaviorSubject::new(pairs(&points));
    let selected = BehaviorSubject::new(Some(0));

    // The background and the grid: they depend on the size alone.
    let grid = layer("curve-grid", move |size| {
      let plot = Plot::new(size, min, max, density);
      let mut image = Pixels::new_from_color(size.0, size.1, look.background);
      let mut grid_lines = Vec::new();
      for step in 0..=GRID {
        let share = step as f32 / GRID as f32;
        let (across, down) = (plot.left + plot.width * share, plot.top + plot.height * share);
        grid_lines.push(vec![(across, plot.top), (across, plot.top + plot.height)]);
        grid_lines.push(vec![(plot.left, down), (plot.left + plot.width, down)]);
      }
      lines(&mut image, &grid_lines, 1.0, look.grid);
      image
    });

    // The curve: abra's curve through the points, made in the pixels of the plot (it has the same shape there) and flattened into the line abra
    // strokes. It is flat before its first point and after its last, which is drawn as two straight lines.
    let curve = layer("curve-line", {
      let points = points.clone();
      move |size| {
        let points = points.value();
        let plot = Plot::new(size, min, max, density);
        let mut image = Pixels::new(size.0, size.1);
        let pixels: Vec<(f32, f32)> = points.iter().map(|point| plot.to_pixel(*point)).collect();
        let curve = Curve::new(pixels.iter().copied());
        let mut strokes = vec![curve.flatten(0.25).iter().map(|point| (point.x, point.y)).collect::<Vec<_>>()];
        if let (Some(first), Some(last)) = (pixels.iter().min_by(|a, b| a.0.total_cmp(&b.0)), pixels.iter().max_by(|a, b| a.0.total_cmp(&b.0))) {
          strokes.push(vec![(plot.left, first.1), *first]);
          strokes.push(vec![*last, (plot.left + plot.width, last.1)]);
        }
        lines(&mut image, &strokes, 2.5, look.line);
        image
      }
    });

    let root = Container::new();
    root.add(&grid).add(&curve);
    let editor = Arc::new(Editor {
      points: points.clone(),
      selected: selected.clone(),
      size: root.size().clone(),
      root: root.downgrade(),
      look,
      min,
      max,
      density,
      handles: Mutex::new(Vec::new()),
      arranging: Mutex::new(()),
    });
    editor.arrange();
    root.keep(root.size().subscribe({
      let editor = Arc::clone(&editor);
      move |_| editor.arrange()
    }));

    // The point a drag has hold of.
    let held = Mutex::new(None::<Held>);
    root.subject::<Drag>().subscribe({
      let editor = Arc::clone(&editor);
      move |drag| {
        let plot = editor.plot();
        match drag {
          Drag::Start { x, y } => {
            let at = (*x, *y);
            let mut now = editor.points.peek();
            let index = nearest(&now, &plot, at, GRAB * density).unwrap_or_else(|| {
              now.push(plot.to_point(at));
              now.len() - 1
            });
            let taken = Held { index, point: now[index], finger: at };
            editor.points.next(now);
            editor.selected.next(Some(index));
            editor.arrange();
            *held.lock().unwrap() = Some(taken);
          }
          Drag::Move { x, y } => {
            let taken = *held.lock().unwrap();
            if let Some(taken) = taken {
              let start = plot.to_pixel(taken.point);
              let moved = plot.to_point((start.0 + (*x - taken.finger.0), start.1 + (*y - taken.finger.1)));
              let mut now = editor.points.peek();
              if let Some(point) = now.get_mut(taken.index) {
                *point = moved;
                editor.points.next(now);
                editor.arrange();
              }
            }
          }
          Drag::End { .. } => *held.lock().unwrap() = None,
        }
      }
    });

    Arc::new(Self { root, editor })
  }

  /// The points now, as `x, y` pairs in one list, in the order they were made.
  pub fn points(&self) -> Vec<f64> {
    self.editor.points.peek().iter().flat_map(|(x, y)| [*x as f64, *y as f64]).collect()
  }

  /// The place of the selected point in the order they were made, or -1 when none is selected.
  pub fn selected(&self) -> i32 {
    self.editor.selected.peek().map_or(-1, |index| index as i32)
  }

  /// Replaces the points, as when a field was typed in or a point was removed. The selected point stays selected if there is still one at its place.
  ///
  /// # Panics
  /// When there are no points or one has no `y`.
  pub fn set_points(&self, points: Vec<f64>) {
    let points = pairs(&points);
    self.editor.selected.next(self.editor.selected.peek().filter(|index| *index < points.len()));
    self.editor.points.next(points);
    self.editor.arrange();
  }

  /// Selects the point at `index`, or none when it is -1 or past the last.
  pub fn set_selected(&self, index: i32) {
    self.editor.selected.next(usize::try_from(index).ok().filter(|index| *index < self.editor.points.peek().len()));
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
  use std::time::Duration;

  use super::*;
  use crate::components::pixel;

  fn colors() -> Vec<Vec<u8>> {
    vec![vec![30, 30, 30, 255], vec![70, 70, 70, 255], vec![40, 120, 255, 255], vec![40, 120, 255, 255], vec![255, 140, 0, 255]]
  }

  /// An editor 214 x 214 pixels, so the plot is 186 pixels across and down from (14, 14), of the line from (0, 0) to (1, 1) with `min` 0 and `max` 1.
  fn editor() -> Arc<CurveEditor> {
    let editor = CurveEditor::new(vec![0.0, 0.0, 1.0, 1.0], 0.0, 1.0, colors(), 1.0);
    editor.size().next((214, 214));
    editor
  }

  fn render(p_editor: &CurveEditor) -> Frame {
    let mut component = p_editor.component().clone();
    component.render(Duration::ZERO)
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
    let frame = render(&editor());
    // The line from the bottom left to the top right of the plot passes through the middle of it, which is lit in the line's blue.
    let middle = pixel(&frame, 107, 107);
    assert!(middle[2] > middle[0] + 60, "the middle is on the blue line: {middle:?}");
    // Away from the line and the grid it is the background.
    assert_eq!(&pixel(&frame, 80, 50)[..3], &[30, 30, 30]);
    // The selected point (0, 0) is marked at the bottom left of the plot in the selected color.
    let mark = pixel(&frame, 14, 200);
    assert!(mark[0] > 200, "{mark:?}");
  }

  #[test]
  fn dragging_a_point_moves_its_mark_and_leaves_none_behind() {
    let editor = editor();
    render(&editor);
    touch(&editor, 205.0, 20.0, Some(true));
    touch(&editor, 112.0, 113.0, None);
    let frame = render(&editor);
    assert_ne!(&pixel(&frame, 200, 14)[..3], &[255, 140, 0], "the selected point left the corner");
    assert_eq!(&pixel(&frame, 112, 113)[..3], &[255, 140, 0], "and is under the finger's new place");
    assert_eq!(&pixel(&frame, 80, 50)[..3], &[30, 30, 30], "the background is still there");
  }

  /// Dragging on one thread while another draws must not leave either waiting for the other. A hang fails the test at the timeout.
  #[test]
  fn dragging_while_another_thread_draws_does_not_wait_forever() {
    let subject = editor();
    let (done, finished) = std::sync::mpsc::channel();
    let drawing = {
      let (subject, done) = (Arc::clone(&subject), done.clone());
      std::thread::spawn(move || {
        for _ in 0..200 {
          render(&subject);
        }
        done.send(()).unwrap();
      })
    };
    let dragging = {
      let subject = Arc::clone(&subject);
      std::thread::spawn(move || {
        touch(&subject, 205.0, 20.0, Some(true));
        for step in 0..400u32 {
          touch(&subject, 20.0 + (step % 150) as f32, 20.0 + (step % 150) as f32, None);
          subject.set_selected((step % 2) as i32);
        }
        touch(&subject, 20.0, 20.0, Some(false));
        done.send(()).unwrap();
      })
    };
    for _ in 0..2 {
      finished.recv_timeout(Duration::from_secs(20)).expect("drawing or dragging is waiting for the other");
    }
    drawing.join().unwrap();
    dragging.join().unwrap();
  }

  #[test]
  fn dropping_it_frees_its_parts() {
    let subject = editor();
    let weak = subject.component().downgrade();
    render(&subject);
    touch(&subject, 205.0, 20.0, Some(true));
    drop(subject);
    assert!(weak.upgrade().is_none(), "the curve editor is still alive after it was dropped: something in it holds itself");
  }

  #[test]
  fn a_handle_is_not_cut_off_at_the_edge_of_the_child_that_draws_it() {
    for density in [1.0, 2.0, 2.75, 3.0] {
      let editor = CurveEditor::new(vec![0.0, 0.0, 1.0, 1.0], 0.0, 1.0, colors(), density);
      editor.size().next((400, 300));
      for index in 0..2 {
        let mut handle = editor.editor.handle(index);
        let side = (handle_reach(density as f32) * 2.0) as u32;
        handle.size().next((side, side));
        let frame = handle.render(Duration::ZERO);
        let edge: Vec<[u8; 4]> = (0..side).flat_map(|at| [(0, at), (side - 1, at), (at, 0), (at, side - 1)]).map(|(x, y)| pixel(&frame, x, y)).collect();
        assert!(edge.iter().all(|pixel| pixel[3] == 0), "density {density}, handle {index}: the ring reaches the edge of its child");
        assert!(frame.pixels.chunks(4).any(|pixel| pixel[3] == 255), "the handle is drawn");
      }
    }
  }

  /// How long a frame takes while a handle is dragged on a phone-sized editor. Run with
  /// `cargo test -p alakazam-mobile --lib timing_of_a_frame_while_dragging_a_handle -- --ignored --nocapture`.
  #[test]
  #[ignore]
  fn timing_of_a_frame_while_dragging_a_handle() {
    let editor = CurveEditor::new(vec![0.0, 0.0, 0.25, 0.4, 0.5, 0.5, 0.75, 0.8, 1.0, 1.0], 0.0, 1.0, colors(), 3.0);
    editor.size().next((1000, 700));
    render(&editor);
    // Take the middle point, which is at (14 + 0.5 * 972, 14 + 0.5 * 672).
    touch(&editor, 500.0, 350.0, Some(true));
    let mut moves = Vec::new();
    let mut frames = Vec::new();
    for step in 0..300u32 {
      let (x, y) = (480.0 + (step % 40) as f32, 330.0 + (step % 60) as f32);
      let started = std::time::Instant::now();
      touch(&editor, x, y, None);
      moves.push(started.elapsed().as_secs_f64() * 1000.0);
      let started = std::time::Instant::now();
      render(&editor);
      frames.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let average = |times: &[f64]| times.iter().sum::<f64>() / times.len() as f64;
    println!("moving the point: {:.3} ms, drawing the frame: {:.3} ms on average, worst frame {:.3} ms", average(&moves), average(&frames), frames.iter().cloned().fold(0.0, f64::max));
  }
}
