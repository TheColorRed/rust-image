use std::sync::{Arc, Mutex};

use abra::abra_core::{AlphaStop, Area, Channels, Color, ColorStop, Fill, Gradient, Image as Pixels};
use abra::drawing::prelude::fill;
use vessel::prelude::*;

use crate::components::{color_of, layer};

/// Room left on each side of the bar, so a stop at its end can be seen and taken.
const SIDE: f32 = 12.0;
/// The height of the strip of stops above the bar and of the one below it.
const STRIP: f32 = 30.0;
/// How close a finger has to land to a stop to take it instead of making a new one.
const GRAB: f32 = 18.0;
const MARKER_WIDTH: f32 = 12.0;
const MARKER_HEIGHT: f32 = 18.0;
/// The squares of the checkerboard under the bar, which shows how solid the gradient is.
const CELL: f32 = 8.0;

/// Which list of stops a stop is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
  Color,
  Alpha,
}

/// A color stop: where it is along the gradient (0 to 1) and its color as red, green and blue.
type ColorPlace = (f32, [u8; 3]);

/// The gradient as the colors it passes through and, apart from them, how solid it is along its length, each in the order they were made.
#[derive(Clone)]
struct Stops {
  colors: Vec<ColorPlace>,
  alphas: Vec<AlphaStop>,
}

impl Stops {
  /// abra's gradient through the color stops, which mixes the colors.
  fn color_gradient(&self) -> Gradient {
    Gradient::new(self.colors.iter().map(|(at, [red, green, blue])| ColorStop::new(Color::from_rgb(*red, *green, *blue), *at)).collect())
  }

  /// The color of the gradient at `p_at`, 0 to 1.
  fn color_at(&self, p_at: f32) -> [u8; 3] {
    let color = self.color_gradient().color_at(p_at);
    [color.r, color.g, color.b]
  }

  /// How solid the gradient is at `p_at`, 0 to 255.
  fn alpha_at(&self, p_at: f32) -> u8 {
    AlphaStop::alpha_at(&self.alphas, p_at)
  }

  /// The gradient as abra draws it: the colors, with the strength the strength stops give.
  fn gradient(&self) -> Gradient {
    self.color_gradient().with_alpha_stops(&self.alphas)
  }

  fn places(&self, p_kind: Kind) -> Vec<f32> {
    match p_kind {
      Kind::Color => self.colors.iter().map(|stop| stop.0).collect(),
      Kind::Alpha => self.alphas.iter().map(|stop| stop.time).collect(),
    }
  }

  fn move_stop(&mut self, p_kind: Kind, p_index: usize, p_at: f32) {
    match p_kind {
      Kind::Color => self.colors[p_index].0 = p_at,
      Kind::Alpha => self.alphas[p_index].time = p_at,
    }
  }
}

/// Where the bar and its strips are on a surface.
#[derive(Clone, Copy)]
struct Layout {
  left: f32,
  width: f32,
  bar_top: f32,
  bar_height: f32,
  /// The height of the surface, which the halves for the two strips are made from.
  height: f32,
}

impl Layout {
  fn new(p_size: (u32, u32), p_density: f32) -> Self {
    let height = p_size.1 as f32;
    let side = SIDE * p_density;
    let strip = STRIP * p_density;
    Self { left: side, width: (p_size.0 as f32 - side * 2.0).max(1.0), bar_top: strip, bar_height: (height - strip * 2.0).max(1.0), height }
  }

  fn x_of(&self, p_at: f32) -> f32 {
    self.left + p_at * self.width
  }

  fn at_of(&self, p_x: f32) -> f32 {
    ((p_x - self.left) / self.width).clamp(0.0, 1.0)
  }

  /// Which strip a place on the surface is for: the strength stops are over the bar and the color stops under it, and the bar is shared between them.
  fn kind_of(&self, p_y: f32) -> Kind {
    if p_y < self.height / 2.0 { Kind::Alpha } else { Kind::Color }
  }
}

/// The outline of the bar and a marker for each stop, on a picture of the whole surface.
fn paint_markers(p_stops: &Stops, p_selected: Option<(Kind, usize)>, p_accent: Color, p_outline: Color, p_density: f32, p_size: (u32, u32)) -> Pixels {
  let scale = p_density;
  let layout = Layout::new(p_size, scale);
  let mut image = Pixels::new(p_size.0, p_size.1);
  let bar_origin = (layout.left, layout.bar_top);
  // The outline of the bar is four thin rectangles.
  for (origin, size) in [
    (bar_origin, (layout.width, 1.0)),
    ((layout.left, layout.bar_top + layout.bar_height - 1.0), (layout.width, 1.0)),
    (bar_origin, (1.0, layout.bar_height)),
    ((layout.left + layout.width - 1.0, layout.bar_top), (1.0, layout.bar_height)),
  ] {
    fill(Area::rect(origin, size), Fill::solid(p_outline)).apply(&mut image);
  }

  let marker = |image: &mut Pixels, p_x: f32, p_y: f32, p_fill: Color, p_on: bool, p_points_down: bool| {
    let marker_width = MARKER_WIDTH * scale;
    let marker_height = MARKER_HEIGHT * scale;
    let edge = if p_on { p_accent } else { Color::white() };
    // A flat-topped shield: broad shoulders, straight sides and a centered point.
    let draw_shape = |image: &mut Pixels, inset: f32, color: Color| {
      let inset = inset * scale;
      let left = p_x - marker_width / 2.0 + inset;
      let right = p_x + marker_width / 2.0 - inset;
      let top = p_y + inset;
      let bottom = p_y + marker_height - inset;
      let shoulder = if p_points_down { bottom - 5.0 * scale } else { top + 5.0 * scale };
      let mut shape = Area::new();
      if p_points_down {
        shape.move_to((left, top)).line_to((right, top)).line_to((right, shoulder)).line_to((p_x, bottom)).line_to((left, shoulder)).line_to((left, top));
      } else {
        shape.move_to((p_x, top)).line_to((right, shoulder)).line_to((right, bottom)).line_to((left, bottom)).line_to((left, shoulder)).line_to((p_x, top));
      }
      fill(shape, Fill::solid(color)).apply(image);
    };
    draw_shape(image, 0.0, edge);
    draw_shape(image, 2.0, p_fill);
  };
  for (index, AlphaStop { alpha, time: at }) in p_stops.alphas.iter().enumerate() {
    marker(&mut image, layout.x_of(*at), (STRIP - MARKER_HEIGHT - 3.0) * scale, Color::from_rgb(*alpha, *alpha, *alpha), p_selected == Some((Kind::Alpha, index)), true);
  }
  for (index, (at, [red, green, blue])) in p_stops.colors.iter().enumerate() {
    marker(
      &mut image,
      layout.x_of(*at),
      layout.bar_top + layout.bar_height + 3.0 * scale,
      Color::from_rgb(*red, *green, *blue),
      p_selected == Some((Kind::Color, index)),
      false,
    );
  }
  image
}

/// A stop the finger has hold of.
#[derive(Clone, Copy)]
struct Held {
  kind: Kind,
  index: usize,
  /// Where the stop and the finger were when it was taken, so the stop moves by how far the finger has gone and does not jump to it.
  at: f32,
  finger_x: f32,
}

/// The gradient editor's component, as in Unity's gradient editor: the gradient drawn as a bar over a checkerboard, a strip of strength stops above it and a
/// strip of color stops below it. A finger on the top half takes a strength stop and one on the bottom half a color stop: it takes the nearest stop if one
/// is close, and otherwise makes a new stop there with the color or strength the gradient has at that place, and then moves it along the bar.
///
/// It is made of Vessel children: the checkerboard, which is drawn again only when the size changes, the gradient, which is drawn again when a stop
/// changes, and the outline with the markers. abra draws the pictures; the buttons and fields (a stop's color, strength and location, remove, OK) are the app's.
///
/// Read `colors`, `alphas` and the selection while the finger moves and when it lifts; the app changes them with `set_colors`, `set_alphas` and `set_selected`.
#[derive(uniffi::Object, Component)]
pub struct GradientEditor {
  root: Container,
  stops: BehaviorSubject<Stops>,
  selected: BehaviorSubject<Option<(Kind, usize)>>,
}

#[uniffi::export]
impl GradientEditor {
  /// An editor of the gradient with these stops, with the first color stop selected. `density` converts the editor's dp measurements to surface pixels.
  /// `colors` is `place, red, green, blue` for each color stop in one
  /// list (the place from 0 to 1 along the gradient, the channels 0 to 255) and `alphas` is `place, alpha` for each strength stop. `accent` and `outline` are
  /// colors, each as red, green, blue and alpha from 0 to 255: the ring round the selected stop, and the outline of the bar.
  ///
  /// # Panics
  /// When either list is empty or not whole stops, or a color has fewer than four numbers.
  #[uniffi::constructor]
  pub fn new(colors: Vec<f64>, alphas: Vec<f64>, accent: Vec<u8>, outline: Vec<u8>, density: f64) -> Arc<Self> {
    let (accent, outline) = (color_of(&accent), color_of(&outline));
    let density = (density as f32).max(0.1);
    let stops = BehaviorSubject::new(Stops { colors: color_stops(&colors), alphas: alpha_stops(&alphas) });
    let selected = BehaviorSubject::new(Some((Kind::Color, 0)));

    // The checkerboard under the bar, which depends on the size alone, and the gradient over it.
    let checkers = layer("gradient-checkerboard", move |size| {
      let mut image = Pixels::new(size.0, size.1);
      fill(Area::rect((0.0, 0.0), (size.0 as f32, size.1 as f32)), Fill::checkerboard((CELL * density).round() as u32, (255, 255, 255), (200, 200, 200))).apply(&mut image);
      image
    });
    let bar = layer("gradient-bar", {
      let stops = stops.clone();
      move |size| {
        // Every row is the same, so one row is drawn and copied down: drawing the whole bar is much slower, and this is drawn again for each
        // move of a stop.
        let mut row = Pixels::new(size.0, 1);
        fill(Area::rect((0.0, 0.0), (size.0 as f32, 1.0)), &stops.value().gradient()).apply(&mut row);
        Pixels::new_from_pixels(size.0, size.1, row.rgba().repeat(size.1 as usize), Channels::RGBA)
      }
    });
    // The outline and the markers fill the whole surface, over the bar.
    let markers = layer("gradient-markers", {
      let (stops, selected) = (stops.clone(), selected.clone());
      move |size| paint_markers(&stops.value(), selected.value(), accent, outline, density, size)
    });

    let root = Container::new();
    root.add(&checkers).add(&bar).add(&markers);
    let arrange = {
      let size = root.size().clone();
      move || {
        let layout = Layout::new(size.peek(), density);
        for spot in [&checkers, &bar] {
          spot.place((layout.left).round() as u32, (layout.bar_top).round() as u32, (layout.width).round() as u32, (layout.bar_height).round() as u32);
        }
      }
    };
    arrange();
    root.keep(root.size().subscribe(move |_| arrange()));

    // The stop a drag has hold of.
    let held = Mutex::new(None::<Held>);
    let this = root.downgrade();
    root.subject::<Drag>().subscribe({
      let (stops, selected) = (stops.clone(), selected.clone());
      move |drag| {
        let Some(root) = this.upgrade() else { return };
        let layout = Layout::new(root.size().peek(), density);
        match drag {
          Drag::Start { x, y } => {
            let (x, y) = (*x, *y);
            let kind = layout.kind_of(y);
            let mut now = stops.peek();
            let near = now
              .places(kind)
              .iter()
              .enumerate()
              .map(|(index, at)| (index, (layout.x_of(*at) - x).abs()))
              .filter(|(_, distance)| *distance <= GRAB * density)
              .min_by(|a, b| a.1.total_cmp(&b.1))
              .map(|(index, _)| index);
            let index = near.unwrap_or_else(|| {
              // A new stop starts as the gradient is at that place, so it changes nothing until it is moved.
              let at = layout.at_of(x);
              match kind {
                Kind::Color => {
                  let color = now.color_at(at);
                  now.colors.push((at, color));
                  now.colors.len() - 1
                }
                Kind::Alpha => {
                  let alpha = now.alpha_at(at);
                  now.alphas.push(AlphaStop::new(alpha, at));
                  now.alphas.len() - 1
                }
              }
            });
            let taken = Held { kind, index, at: now.places(kind)[index], finger_x: x };
            stops.next(now);
            selected.next(Some((kind, index)));
            *held.lock().unwrap() = Some(taken);
          }
          Drag::Move { x, .. } => {
            let taken = *held.lock().unwrap();
            if let Some(taken) = taken {
              let at = (taken.at + (*x - taken.finger_x) / layout.width).clamp(0.0, 1.0);
              let mut now = stops.peek();
              now.move_stop(taken.kind, taken.index, at);
              stops.next(now);
            }
          }
          Drag::End { .. } => *held.lock().unwrap() = None,
        }
      }
    });

    Arc::new(Self { root, stops, selected })
  }

  /// The color stops now: `place, red, green, blue` for each, in one list, in the order they were made.
  pub fn colors(&self) -> Vec<f64> {
    self.stops.peek().colors.iter().flat_map(|(at, [red, green, blue])| [*at as f64, *red as f64, *green as f64, *blue as f64]).collect()
  }

  /// The strength stops now: `place, alpha` for each, in one list, in the order they were made.
  pub fn alphas(&self) -> Vec<f64> {
    self.stops.peek().alphas.iter().flat_map(|stop| [stop.time as f64, stop.alpha as f64]).collect()
  }

  /// Which list the selected stop is in: 0 for the color stops, 1 for the strength stops, or -1 when none is selected.
  pub fn selected_kind(&self) -> i32 {
    match self.selected.peek() {
      Some((Kind::Color, _)) => 0,
      Some((Kind::Alpha, _)) => 1,
      None => -1,
    }
  }

  /// The place of the selected stop in its list in the order they were made, or -1 when none is selected.
  pub fn selected_index(&self) -> i32 {
    self.selected.peek().map_or(-1, |(_, index)| index as i32)
  }

  /// Replaces the color stops, as when a color was picked or a stop was removed. The selection stays if there is still a stop at its place.
  ///
  /// # Panics
  /// When there is no stop or one is not whole.
  pub fn set_colors(&self, colors: Vec<f64>) {
    let colors = color_stops(&colors);
    self.selected.next(self.selected.peek().filter(|(kind, index)| *kind != Kind::Color || *index < colors.len()));
    let mut now = self.stops.peek();
    now.colors = colors;
    self.stops.next(now);
  }

  /// Replaces the strength stops. The selection stays if there is still a stop at its place.
  ///
  /// # Panics
  /// When there is no stop or one is not whole.
  pub fn set_alphas(&self, alphas: Vec<f64>) {
    let alphas = alpha_stops(&alphas);
    self.selected.next(self.selected.peek().filter(|(kind, index)| *kind != Kind::Alpha || *index < alphas.len()));
    let mut now = self.stops.peek();
    now.alphas = alphas;
    self.stops.next(now);
  }

  /// Selects the stop at `index` in the list `kind` (0 for the colors, 1 for the strengths), or none when `kind` is -1 or there is no such stop.
  pub fn set_selected(&self, kind: i32, index: i32) {
    let now = self.stops.peek();
    let kind = match kind {
      0 => Some(Kind::Color),
      1 => Some(Kind::Alpha),
      _ => None,
    };
    self.selected.next(kind.and_then(|kind| {
      let count = if kind == Kind::Color { now.colors.len() } else { now.alphas.len() };
      usize::try_from(index).ok().filter(|index| *index < count).map(|index| (kind, index))
    }));
  }
}

/// Color stops from `place, red, green, blue` in one list.
///
/// # Panics
/// When the list is empty or does not hold whole stops.
fn color_stops(p_list: &[f64]) -> Vec<ColorPlace> {
  assert!(!p_list.is_empty() && p_list.len() % 4 == 0, "a gradient needs color stops, each with a place, red, green and blue");
  p_list.chunks_exact(4).map(|stop| (stop[0] as f32, [stop[1], stop[2], stop[3]].map(|channel| channel.round().clamp(0.0, 255.0) as u8))).collect()
}

/// Strength stops from `place, alpha` in one list.
///
/// # Panics
/// When the list is empty or does not hold whole stops.
fn alpha_stops(p_list: &[f64]) -> Vec<AlphaStop> {
  assert!(!p_list.is_empty() && p_list.len() % 2 == 0, "a gradient needs strength stops, each with a place and an alpha");
  p_list.chunks_exact(2).map(|stop| AlphaStop::new(stop[1].round().clamp(0.0, 255.0) as u8, stop[0] as f32)).collect()
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;
  use crate::components::pixel;

  fn editor() -> Arc<GradientEditor> {
    // Red to blue, solid at both ends and see-through in the middle.
    let editor = GradientEditor::new(
      vec![0.0, 255.0, 0.0, 0.0, 1.0, 0.0, 0.0, 255.0],
      vec![0.0, 255.0, 0.5, 0.0, 1.0, 255.0],
      vec![255, 140, 0, 255],
      vec![90, 90, 90, 255],
      1.0,
    );
    // 224 wide: a bar 200 across from x = 12. 104 high: strips of 30 and a bar of 44.
    editor.size().next((224, 104));
    editor
  }

  fn render(p_editor: &GradientEditor) -> Frame {
    let mut component = p_editor.component().clone();
    component.render(Duration::ZERO)
  }

  fn touch(p_editor: &GradientEditor, p_x: f32, p_y: f32, p_pressed: Option<bool>) {
    p_editor.next(PointerEvent::Moved { x: p_x, y: p_y });
    if let Some(pressed) = p_pressed {
      p_editor.next(PointerEvent::Button { button: PointerButton::Left, pressed });
    }
  }

  #[test]
  fn colors_and_strengths_are_mixed_from_their_own_stops() {
    let editor = editor();
    let stops = editor.stops.peek();
    assert_eq!(stops.color_at(0.5), [128, 0, 128], "halfway from red to blue");
    assert_eq!(stops.alpha_at(0.5), 0);
    assert_eq!(stops.alpha_at(0.25), 128, "halfway to nothing");
    assert_eq!(stops.color_at(-1.0), [255, 0, 0], "before the first stop it is that stop's color");
    assert_eq!(stops.alpha_at(5.0), 255);
  }

  #[test]
  fn a_finger_on_a_stop_takes_it_and_moves_it_along_the_bar() {
    let editor = editor();
    // The top half is the strength stops, and the one in the middle is at x = 112.
    touch(&editor, 114.0, 10.0, Some(true));
    assert_eq!((editor.selected_kind(), editor.selected_index()), (1, 1));
    touch(&editor, 114.0 + 50.0, 10.0, None);
    let alphas = editor.alphas();
    assert!((alphas[2] - 0.75).abs() < 0.01, "{alphas:?}");
    touch(&editor, 900.0, 10.0, None);
    assert_eq!(editor.alphas()[2], 1.0, "held at the end of the bar");
    touch(&editor, 900.0, 10.0, Some(false));
    touch(&editor, 20.0, 10.0, None);
    assert_eq!(editor.alphas()[2], 1.0, "let go");
  }

  #[test]
  fn a_finger_on_an_empty_place_makes_a_stop_that_changes_nothing_until_it_is_moved() {
    let editor = editor();
    // The bottom half is the color stops: x = 62 is a quarter of the way along the bar, away from both stops.
    touch(&editor, 62.0, 90.0, Some(true));
    assert_eq!((editor.selected_kind(), editor.selected_index()), (0, 2));
    let colors = editor.colors();
    assert_eq!(colors.len(), 12);
    assert!((colors[8] - 0.25).abs() < 0.01);
    assert_eq!(&colors[9..12], &[191.0, 0.0, 64.0], "red going to blue, a quarter of the way");
  }

  #[test]
  fn the_app_can_set_the_stops_and_the_selection() {
    let editor = editor();
    editor.set_colors(vec![0.0, 10.0, 20.0, 30.0]);
    assert_eq!(editor.colors(), vec![0.0, 10.0, 20.0, 30.0]);
    editor.set_selected(1, 2);
    assert_eq!((editor.selected_kind(), editor.selected_index()), (1, 2));
    editor.set_alphas(vec![0.0, 40.0]);
    assert_eq!(editor.selected_kind(), -1, "the stop it had is gone");
    editor.set_selected(0, 0);
    assert_eq!((editor.selected_kind(), editor.selected_index()), (0, 0));
    editor.set_selected(0, 5);
    assert_eq!(editor.selected_kind(), -1);
  }

  #[test]
  fn abra_draws_the_gradient_over_a_checkerboard() {
    let frame = render(&editor());
    // The start is solid red and the end solid blue; in the middle it is see-through, so the checkerboard's grays show.
    let start = pixel(&frame, 14, 40);
    assert!(start[0] > 230 && start[2] < 30, "{start:?}");
    let end = pixel(&frame, 210, 40);
    assert!(end[2] > 230 && end[0] < 30, "{end:?}");
    let middle = pixel(&frame, 112, 40);
    let (low, high) = (middle[..3].iter().min().unwrap(), middle[..3].iter().max().unwrap());
    assert!(high - low <= 6, "a gray: {middle:?}");
  }

  #[test]
  fn moving_a_stop_changes_the_gradient_and_leaves_the_checkerboard_alone() {
    let editor = editor();
    let before = render(&editor);
    // Make the end of the bar red as well: the whole bar is red.
    editor.set_colors(vec![0.0, 255.0, 0.0, 0.0, 1.0, 255.0, 0.0, 0.0]);
    let after = render(&editor);
    let end = pixel(&after, 210, 40);
    assert!(end[0] > 230 && end[2] < 30, "the end of the bar is red now: {end:?}");
    assert_eq!(pixel(&before, 112, 40), pixel(&after, 112, 40), "the see-through middle shows the same checkerboard");
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
        touch(&subject, 114.0, 10.0, Some(true));
        for step in 0..400u32 {
          touch(&subject, 20.0 + (step % 180) as f32, 10.0, None);
          subject.set_selected(0, (step % 2) as i32);
        }
        touch(&subject, 20.0, 10.0, Some(false));
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
    touch(&subject, 114.0, 10.0, Some(true));
    drop(subject);
    assert!(weak.upgrade().is_none(), "the gradient editor is still alive after it was dropped: something in it holds itself");
  }

  /// How long a frame takes while a stop is dragged on a phone-sized editor. Run with
  /// `cargo test -p alakazam-mobile --lib timing_of_a_frame_while_dragging -- --ignored --nocapture`.
  #[test]
  #[ignore]
  fn timing_of_a_frame_while_dragging_a_stop() {
    let editor = GradientEditor::new(
      vec![0.0, 255.0, 0.0, 0.0, 0.5, 0.0, 255.0, 0.0, 1.0, 0.0, 0.0, 255.0],
      vec![0.0, 255.0, 1.0, 255.0],
      vec![255, 140, 0, 255],
      vec![90, 90, 90, 255],
      3.0,
    );
    editor.size().next((1000, 330));
    render(&editor);
    // The middle color stop is below the bar at x = 36 + 0.5 * 928.
    touch(&editor, 500.0, 300.0, Some(true));
    let mut frames = Vec::new();
    for step in 0..200u32 {
      touch(&editor, 400.0 + (step % 100) as f32, 300.0, None);
      let started = std::time::Instant::now();
      render(&editor);
      frames.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    println!("a frame while a stop is dragged: {:.3} ms, worst {:.3} ms", frames.iter().sum::<f64>() / frames.len() as f64, frames.iter().cloned().fold(0.0, f64::max));
  }
}
