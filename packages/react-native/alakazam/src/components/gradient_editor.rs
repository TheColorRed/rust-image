use std::sync::{Arc, Mutex};

use abra::abra_core::{Area, Color, ColorStop, Fill, Gradient, Image as Pixels};
use abra::drawing::prelude::fill;
use vessel::prelude::*;

use crate::components::surface::{PictureSource, Shared, blank, checkerboard, color_of, rect, shade};

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
/// A strength stop: where it is along the gradient (0 to 1) and how solid it is, 0 to 255.
type AlphaPlace = (f32, u8);

/// What the editor shows: the gradient as the colors it passes through and, apart from them, how solid it is along its length, each in the order they
/// were made, and the stop that is selected.
#[derive(Clone)]
struct State {
  colors: Vec<ColorPlace>,
  alphas: Vec<AlphaPlace>,
  selected: Option<(Kind, usize)>,
  /// The ring round the selected stop, and the outline of the bar.
  accent: Color,
  outline: Color,
  /// React Native's screen density, used to turn the editor's dp measurements into surface pixels.
  density: f32,
}

impl State {
  /// abra's gradient through the color stops, which mixes the colors.
  fn color_gradient(&self) -> Gradient {
    Gradient::new(self.colors.iter().map(|(at, [red, green, blue])| ColorStop::new(Color::from_rgb(*red, *green, *blue), *at)).collect())
  }

  /// The color of the gradient at `p_at`, 0 to 1.
  fn color_at(&self, p_at: f32) -> [u8; 3] {
    let color = self.color_gradient().color_at(p_at);
    [color.r, color.g, color.b]
  }

  /// How solid the gradient is at `p_at`, 0 to 255, mixed in a straight line between the strength stops around it and held at the first and last beyond them.
  fn alpha_at(&self, p_at: f32) -> u8 {
    let mut stops = self.alphas.clone();
    stops.sort_by(|a, b| a.0.total_cmp(&b.0));
    let Some(after) = stops.iter().position(|stop| stop.0 >= p_at) else { return stops[stops.len() - 1].1 };
    if after == 0 {
      return stops[0].1;
    }
    let (before, after) = (stops[after - 1], stops[after]);
    let along = if after.0 > before.0 { (p_at - before.0) / (after.0 - before.0) } else { 0.0 };
    (before.1 as f32 + (after.1 as f32 - before.1 as f32) * along).round() as u8
  }

  /// The gradient as abra draws it: a stop at every place either list has one, with the color and the strength the lists have there.
  fn gradient(&self) -> Gradient {
    let mut places: Vec<f32> = self.colors.iter().map(|stop| stop.0).chain(self.alphas.iter().map(|stop| stop.0)).collect();
    places.sort_by(|a, b| a.total_cmp(b));
    places.dedup();
    Gradient::new(
      places
        .into_iter()
        .map(|at| {
          let [red, green, blue] = self.color_at(at);
          ColorStop::new(Color::from_rgba(red, green, blue, self.alpha_at(at)), at)
        })
        .collect(),
    )
  }

  fn places(&self, p_kind: Kind) -> Vec<f32> {
    match p_kind {
      Kind::Color => self.colors.iter().map(|stop| stop.0).collect(),
      Kind::Alpha => self.alphas.iter().map(|stop| stop.0).collect(),
    }
  }

  fn move_stop(&mut self, p_kind: Kind, p_index: usize, p_at: f32) {
    match p_kind {
      Kind::Color => self.colors[p_index].0 = p_at,
      Kind::Alpha => self.alphas[p_index].0 = p_at,
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

/// The picture of the editor: abra's gradient over a checkerboard, and a marker for each stop.
fn paint(p_state: &State, p_size: (u32, u32)) -> Pixels {
  let scale = p_state.density;
  let layout = Layout::new(p_size, scale);
  let mut image = blank(p_size);
  let (bar_origin, bar_size) = ((layout.left, layout.bar_top), (layout.width, layout.bar_height));
  checkerboard(&mut image, bar_origin, bar_size, CELL * scale);
  shade(&mut image, bar_origin, bar_size, &p_state.gradient());
  // The outline of the bar is four thin rectangles.
  for (origin, size) in [
    (bar_origin, (layout.width, 1.0)),
    ((layout.left, layout.bar_top + layout.bar_height - 1.0), (layout.width, 1.0)),
    (bar_origin, (1.0, layout.bar_height)),
    ((layout.left + layout.width - 1.0, layout.bar_top), (1.0, layout.bar_height)),
  ] {
    rect(&mut image, origin, size, p_state.outline);
  }

  let marker = |image: &mut Pixels, p_x: f32, p_y: f32, p_fill: Color, p_on: bool, p_points_down: bool| {
    let marker_width = MARKER_WIDTH * scale;
    let marker_height = MARKER_HEIGHT * scale;
    let edge = if p_on { p_state.accent } else { Color::white() };
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
  for (index, (at, alpha)) in p_state.alphas.iter().enumerate() {
    marker(&mut image, layout.x_of(*at), (STRIP - MARKER_HEIGHT - 3.0) * scale, Color::from_rgb(*alpha, *alpha, *alpha), p_state.selected == Some((Kind::Alpha, index)), true);
  }
  for (index, (at, [red, green, blue])) in p_state.colors.iter().enumerate() {
    marker(
      &mut image,
      layout.x_of(*at),
      layout.bar_top + layout.bar_height + 3.0 * scale,
      Color::from_rgb(*red, *green, *blue),
      p_state.selected == Some((Kind::Color, index)),
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

/// The gradient editor's surface, as in Unity's gradient editor: the gradient drawn as a bar over a checkerboard, a strip of strength stops above it and a
/// strip of color stops below it. A finger on the top half takes a strength stop and one on the bottom half a color stop: it takes the nearest stop if one
/// is close, and otherwise makes a new stop there with the color or strength the gradient has at that place, and then moves it along the bar. abra draws
/// the picture and Vessel renders it and brings the touches; the buttons and fields (a stop's color, strength and location, remove, OK) are the app's.
///
/// Read `colors`, `alphas` and the selection while the finger moves and when it lifts; the app changes them with `set_colors`, `set_alphas` and `set_selected`.
#[derive(uniffi::Object, Component)]
pub struct GradientEditor {
  image: Image,
  shared: Arc<Shared<State>>,
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
    let shared = Shared::new(State {
      colors: color_stops(&colors),
      alphas: alpha_stops(&alphas),
      selected: Some((Kind::Color, 0)),
      accent: color_of(&accent),
      outline: color_of(&outline),
      density: (density as f32).max(0.1),
    });
    let image = Image::new().track(&PictureSource::new(&shared, paint));

    let finger = Mutex::new(((0.0f32, 0.0f32), None::<Held>));
    let this = image.downgrade();
    image.subject::<PointerEvent>().subscribe({
      let shared = Arc::clone(&shared);
      move |event| {
        let Some(image) = this.upgrade() else { return };
        let density = shared.get().density;
        let layout = Layout::new(image.size().value(), density);
        let mut finger = finger.lock().unwrap();
        match event {
          PointerEvent::Moved { x, y } => {
            finger.0 = (*x, *y);
            if let Some(held) = finger.1 {
              let at = (held.at + (*x - held.finger_x) / layout.width).clamp(0.0, 1.0);
              shared.update(|state| state.move_stop(held.kind, held.index, at));
            }
          }
          PointerEvent::Button { button: PointerButton::Left, pressed: true } => {
            let (x, y) = finger.0;
            let kind = layout.kind_of(y);
            let mut taken = None;
            shared.update(|state| {
              let near = state
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
                    let color = state.color_at(at);
                    state.colors.push((at, color));
                    state.colors.len() - 1
                  }
                  Kind::Alpha => {
                    let alpha = state.alpha_at(at);
                    state.alphas.push((at, alpha));
                    state.alphas.len() - 1
                  }
                }
              });
              state.selected = Some((kind, index));
              taken = Some(Held { kind, index, at: state.places(kind)[index], finger_x: x });
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

  /// The color stops now: `place, red, green, blue` for each, in one list, in the order they were made.
  pub fn colors(&self) -> Vec<f64> {
    self.shared.get().colors.iter().flat_map(|(at, [red, green, blue])| [*at as f64, *red as f64, *green as f64, *blue as f64]).collect()
  }

  /// The strength stops now: `place, alpha` for each, in one list, in the order they were made.
  pub fn alphas(&self) -> Vec<f64> {
    self.shared.get().alphas.iter().flat_map(|(at, alpha)| [*at as f64, *alpha as f64]).collect()
  }

  /// Which list the selected stop is in: 0 for the color stops, 1 for the strength stops, or -1 when none is selected.
  pub fn selected_kind(&self) -> i32 {
    match self.shared.get().selected {
      Some((Kind::Color, _)) => 0,
      Some((Kind::Alpha, _)) => 1,
      None => -1,
    }
  }

  /// The place of the selected stop in its list in the order they were made, or -1 when none is selected.
  pub fn selected_index(&self) -> i32 {
    self.shared.get().selected.map_or(-1, |(_, index)| index as i32)
  }

  /// Replaces the color stops, as when a color was picked or a stop was removed. The selection stays if there is still a stop at its place.
  ///
  /// # Panics
  /// When there is no stop or one is not whole.
  pub fn set_colors(&self, colors: Vec<f64>) {
    let colors = color_stops(&colors);
    self.shared.update(|state| {
      state.selected = state.selected.filter(|(kind, index)| *kind != Kind::Color || *index < colors.len());
      state.colors = colors;
    });
  }

  /// Replaces the strength stops. The selection stays if there is still a stop at its place.
  ///
  /// # Panics
  /// When there is no stop or one is not whole.
  pub fn set_alphas(&self, alphas: Vec<f64>) {
    let alphas = alpha_stops(&alphas);
    self.shared.update(|state| {
      state.selected = state.selected.filter(|(kind, index)| *kind != Kind::Alpha || *index < alphas.len());
      state.alphas = alphas;
    });
  }

  /// Selects the stop at `index` in the list `kind` (0 for the colors, 1 for the strengths), or none when `kind` is -1 or there is no such stop.
  pub fn set_selected(&self, kind: i32, index: i32) {
    self.shared.update(|state| {
      let kind = match kind {
        0 => Some(Kind::Color),
        1 => Some(Kind::Alpha),
        _ => None,
      };
      state.selected = kind.and_then(|kind| {
        let count = if kind == Kind::Color { state.colors.len() } else { state.alphas.len() };
        usize::try_from(index).ok().filter(|index| *index < count).map(|index| (kind, index))
      });
    });
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
fn alpha_stops(p_list: &[f64]) -> Vec<AlphaPlace> {
  assert!(!p_list.is_empty() && p_list.len() % 2 == 0, "a gradient needs strength stops, each with a place and an alpha");
  p_list.chunks_exact(2).map(|stop| (stop[0] as f32, stop[1].round().clamp(0.0, 255.0) as u8)).collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::components::surface::pixel;

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

  fn touch(p_editor: &GradientEditor, p_x: f32, p_y: f32, p_pressed: Option<bool>) {
    p_editor.next(PointerEvent::Moved { x: p_x, y: p_y });
    if let Some(pressed) = p_pressed {
      p_editor.next(PointerEvent::Button { button: PointerButton::Left, pressed });
    }
  }

  #[test]
  fn colors_and_strengths_are_mixed_from_their_own_stops() {
    let editor = editor();
    let state = editor.shared.get();
    assert_eq!(state.color_at(0.5), [128, 0, 128], "halfway from red to blue");
    assert_eq!(state.alpha_at(0.5), 0);
    assert_eq!(state.alpha_at(0.25), 128, "halfway to nothing");
    assert_eq!(state.color_at(-1.0), [255, 0, 0], "before the first stop it is that stop's color");
    assert_eq!(state.alpha_at(5.0), 255);
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
    let editor = editor();
    let image = paint(&editor.shared.get(), (224, 104));
    // The start is solid red and the end solid blue; in the middle it is see-through, so the checkerboard's grays show.
    let start = pixel(&image, 14, 40);
    assert!(start[0] > 230 && start[2] < 30, "{start:?}");
    let end = pixel(&image, 210, 40);
    assert!(end[2] > 230 && end[0] < 30, "{end:?}");
    let middle = pixel(&image, 112, 40);
    let (low, high) = (middle[..3].iter().min().unwrap(), middle[..3].iter().max().unwrap());
    assert!(high - low <= 6, "a gray: {middle:?}");
  }
}
