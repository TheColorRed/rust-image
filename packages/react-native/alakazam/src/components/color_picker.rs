use std::sync::{Arc, Mutex};

use abra::abra_core::{Area, Channels, Color, ColorStop, Fill, Gradient, Image as Pixels, Path};
use abra::drawing::prelude::fill;
use vessel::prelude::*;

use crate::components::{color_of, layer};

/// The width of the bar that picks which color it is, and the room between it and the square.
const HUE_BAR_WIDTH: f32 = 56.0;
const GAP: f32 = 12.0;
/// The height of the bar that picks how solid the color is, when there is one.
const STRENGTH_HEIGHT: f32 = 28.0;
/// The squares of the checkerboard under the strength bar.
const CELL: f32 = 8.0;
const RING_RADIUS: f32 = 14.0;
/// The colors of the bar of hues, from red at the top round the color wheel and back to red at the bottom.
const HUES: [f32; 7] = [0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0];

/// Where the parts of the picker are on a surface. The parts sit inside a margin all round, as wide as the ring is, so a mark at the edge of
/// a part is still whole.
#[derive(Clone, Copy)]
struct Layout {
  inset: f32,
  /// The side of the square that picks how colorful and how bright.
  side: f32,
  /// Whether there is a bar under the square for how solid the color is.
  strength: bool,
  /// The width of the parts together: the square, the gap and the bar of colors, which the strength bar spans.
  width: f32,
}

impl Layout {
  fn new(p_size: (u32, u32), p_strength: bool, p_density: f32) -> Self {
    let inset = ((RING_RADIUS + 3.0) * p_density).ceil();
    let (width, height) = ((p_size.0 as f32 - inset * 2.0).max(1.0), (p_size.1 as f32 - inset * 2.0).max(1.0));
    let below = if p_strength { STRENGTH_HEIGHT + GAP } else { 0.0 };
    Self { inset, side: (width - HUE_BAR_WIDTH - GAP).min(height - below).max(20.0), strength: p_strength, width }
  }

  /// From the left of the parts to the bar of colors.
  fn hue_bar_left(&self) -> f32 {
    self.side + GAP
  }

  /// From the top of the parts to the bar of strengths.
  fn strength_top(&self) -> f32 {
    self.side + GAP
  }

  /// Which part a place on the surface is in, from where a touch begins.
  fn part_at(&self, p_x: f32, p_y: f32) -> Option<Part> {
    let (x, y) = (p_x - self.inset, p_y - self.inset);
    if x <= self.side && y <= self.side {
      Some(Part::Square)
    } else if x >= self.hue_bar_left() && y <= self.side {
      Some(Part::Hue)
    } else if self.strength && y >= self.strength_top() {
      Some(Part::Strength)
    } else {
      None
    }
  }
}

/// A part of the picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
  /// The square: colorfulness across and brightness down.
  Square,
  /// The bar to the right of it: which color.
  Hue,
  /// The bar under it: how solid.
  Strength,
}

/// A line from `p_from` to `p_to`, which a gradient follows.
fn along(p_from: (f32, f32), p_to: (f32, f32)) -> Path {
  let mut path = Path::new();
  path.move_to(p_from).line_to(p_to);
  path
}

/// The color as the hue (0 to 360), how colorful it is (0 to 1), how bright (0 to 1) and how solid (0 to 255), and the children that show it.
/// The square, the bar of colors and the bar of strengths are layers that are drawn once at their size and again only when what they show
/// changes; the marks are small children that are drawn once and moved.
struct Parts {
  hue: BehaviorSubject<f32>,
  saturation: BehaviorSubject<f32>,
  brightness: BehaviorSubject<f32>,
  alpha: BehaviorSubject<u8>,
  size: BehaviorSubject<(u32, u32)>,
  strength: bool,
  density: f32,
  square: Component,
  veil: Component,
  hue_bar: Component,
  checkerboard: Component,
  fade: Component,
  ring_mark: Component,
  hue_mark: Component,
  alpha_mark: Component,
}

impl Parts {
  /// Puts every child where the colors and the size of the surface say.
  fn arrange(&self) {
    let layout = Layout::new(self.size.peek(), self.strength, self.density);
    let Layout { inset, side, width, .. } = layout;
    for part in [&self.square, &self.veil] {
      part.place((inset).round() as u32, (inset).round() as u32, (side).round() as u32, (side).round() as u32);
    }
    self.hue_bar.place((inset + layout.hue_bar_left()).round() as u32, (inset).round() as u32, (HUE_BAR_WIDTH).round() as u32, (side).round() as u32);
    let reach = inset * 2.0;
    self.ring_mark.place((self.saturation.peek() * side).round() as u32, ((1.0 - self.brightness.peek()) * side).round() as u32, (reach).round() as u32, (reach).round() as u32);
    let mark_y = self.hue.peek() / 360.0 * side;
    self.hue_mark.place((inset + layout.hue_bar_left() - 2.0).round() as u32, (inset + mark_y - 4.0).round() as u32, (HUE_BAR_WIDTH + 4.0).round() as u32, 8.0 as u32);
    if layout.strength {
      let top = inset + layout.strength_top();
      self.checkerboard.place((inset).round() as u32, (top).round() as u32, (width).round() as u32, (STRENGTH_HEIGHT).round() as u32);
      self.fade.place((inset).round() as u32, (top).round() as u32, (width).round() as u32, (STRENGTH_HEIGHT).round() as u32);
      let x = inset + self.alpha.peek() as f32 / 255.0 * width;
      self.alpha_mark.place((x - 4.0).round() as u32, (top - 2.0).round() as u32, 8.0 as u32, (STRENGTH_HEIGHT + 4.0).round() as u32);
    } else {
      // Nothing is under the square: the strength children take no room.
      for spot in [&self.checkerboard, &self.fade, &self.alpha_mark] {
        spot.place(0.0 as u32, 0.0 as u32, 1.0 as u32, 1.0 as u32);
      }
    }
  }

  /// Moves a part to a place on the surface.
  fn drag(&self, p_part: Part, p_at: (f32, f32)) {
    let layout = Layout::new(self.size.peek(), self.strength, self.density);
    let (x, y) = (p_at.0 - layout.inset, p_at.1 - layout.inset);
    match p_part {
      Part::Square => {
        self.saturation.next((x / layout.side).clamp(0.0, 1.0));
        self.brightness.next(1.0 - (y / layout.side).clamp(0.0, 1.0));
      }
      Part::Hue => self.hue.next(((y / layout.side).clamp(0.0, 1.0) * 360.0).min(359.999)),
      Part::Strength => self.alpha.next(((x / layout.width).clamp(0.0, 1.0) * 255.0).round() as u8),
    }
    self.arrange();
  }
}

/// The color picker's component: a square to drag a finger over for how colorful and how bright the color is, a bar beside it for which color it
/// is, and, when asked, a bar under it for how solid it is. A touch that begins on a part keeps moving that part until the finger lifts, and stops
/// at its edge. It is made of Vessel children, so a drag moves a mark and nothing else is drawn again: the square is drawn again only when the hue
/// changes. abra draws the pictures; the swatches, the number boxes, the web color and the buttons are the app's.
///
/// Read `hue`, `saturation`, `brightness` and `alpha` while the finger moves and when it lifts; the app changes them with `set_color` and `set_alpha`.
#[derive(uniffi::Object, Component)]
pub struct ColorPicker {
  root: Container,
  parts: Arc<Parts>,
}

#[uniffi::export]
impl ColorPicker {
  /// A picker that starts at this color: the `hue` from 0 to 360, `saturation` (how colorful) and `brightness` from 0 to 1. With `strength` it has the bar
  /// for how solid the color is, starting at `alpha` (0 to 255). `ring` and `shadow` are colors, each as red, green, blue and alpha from 0 to 255, for the
  /// marks on the parts: the marks are a ring of the first color on a halo of the second. `density` converts the handle's dp measurements to surface pixels.
  ///
  /// # Panics
  /// When a color has fewer than four numbers.
  #[uniffi::constructor]
  pub fn new(hue: f64, saturation: f64, brightness: f64, strength: bool, alpha: u8, ring: Vec<u8>, shadow: Vec<u8>, density: f64) -> Arc<Self> {
    let (ring_color, shadow) = (color_of(&ring), color_of(&shadow));
    let density = (density as f32).max(0.1);
    let hue = BehaviorSubject::new((hue as f32).rem_euclid(360.0));
    let saturation = BehaviorSubject::new((saturation as f32).clamp(0.0, 1.0));
    let brightness = BehaviorSubject::new((brightness as f32).clamp(0.0, 1.0));
    let alpha = BehaviorSubject::new(alpha);

    // The square: white to the pure color across. It depends on the hue alone, so it is the only part of the square drawn again when the hue moves.
    let square = layer("color-picker-field", {
      let hue = hue.clone();
      move |size| {
        // Every row is the same, so one row is drawn and copied down: drawing the whole square is much slower, and this is what is drawn again
        // for each move of the bar of colors.
        let mut row = Pixels::new(size.0, 1);
        let gradient = Gradient::from_to(Color::white(), Color::from_hsv(hue.value(), 1.0, 1.0));
        fill(Area::rect((0.0, 0.0), (size.0 as f32, 1.0)), &gradient).apply(&mut row);
        Pixels::new_from_pixels(size.0, size.1, row.rgba().repeat(size.1 as usize), Channels::RGBA)
      }
    });
    // Over the square, a veil from clear to black down. It never changes.
    let veil = layer("color-picker-veil", |size| {
      let mut image = Pixels::new(size.0, size.1);
      let veil = Gradient::from_to(Color::from_rgba(0, 0, 0, 0), Color::black()).with_direction(along((0.0, 0.0), (0.0, size.1 as f32)));
      fill(Area::rect((0.0, 0.0), (size.0 as f32, size.1 as f32)), &veil).apply(&mut image);
      image
    });
    // The bar of colors. It never changes.
    let hue_bar = layer("color-picker-hues", |size| {
      let mut image = Pixels::new(size.0, size.1);
      let hues = Gradient::evenly(HUES.iter().map(|hue| Color::from_hsv(*hue, 1.0, 1.0)).collect()).with_direction(along((0.0, 0.0), (0.0, size.1 as f32)));
      fill(Area::rect((0.0, 0.0), (size.0 as f32, size.1 as f32)), &hues).apply(&mut image);
      image
    });
    // The strength bar: a checkerboard that never changes, and under the marks the color going from not there to solid.
    let checkerboard = layer("color-picker-checkerboard", |size| {
      let mut image = Pixels::new(size.0, size.1);
      fill(Area::rect((0.0, 0.0), (size.0 as f32, size.1 as f32)), Fill::checkerboard(CELL as u32, (255, 255, 255), (200, 200, 200))).apply(&mut image);
      image
    });
    let fade = layer("color-picker-strength", {
      let (hue, saturation, brightness) = (hue.clone(), saturation.clone(), brightness.clone());
      move |size| {
        let color = Color::from_hsv(hue.value(), saturation.value(), brightness.value());
        // Every row is the same, so one row is drawn and copied down.
        let mut row = Pixels::new(size.0, 1);
        let stops = vec![ColorStop::new(Color::from_rgba(color.r, color.g, color.b, 0), 0.0), ColorStop::new(color, 1.0)];
        fill(Area::rect((0.0, 0.0), (size.0 as f32, 1.0)), &Gradient::new(stops)).apply(&mut row);
        Pixels::new_from_pixels(size.0, size.1, row.rgba().repeat(size.1 as usize), Channels::RGBA)
      }
    });
    // The marks: a ring of the first color on a halo of the second, a stripe across the bar of colors in the color it marks, and a stripe across the strength bar.
    let ring_mark = layer("color-picker-ring", move |size| {
      let mut image = Pixels::new(size.0, size.1);
      let middle = (size.0 as f32 / 2.0, size.1 as f32 / 2.0);
      fill(Area::circle(middle, (RING_RADIUS + 1.0) * density).stroke(4.0 * density), Fill::solid(shadow)).apply(&mut image);
      fill(Area::circle(middle, RING_RADIUS * density).stroke(2.0 * density), Fill::solid(ring_color)).apply(&mut image);
      image
    });
    let hue_mark = layer("color-picker-hue-mark", {
      let hue = hue.clone();
      move |size| {
        let (width, height) = (size.0 as f32, size.1 as f32);
        let mut image = Pixels::new(size.0, size.1);
        fill(Area::rect((0.0, 0.0), (width, height)), Fill::solid(shadow)).apply(&mut image);
        fill(Area::rect((1.0, 1.0), (width - 2.0, height - 2.0)), Fill::solid(ring_color)).apply(&mut image);
        fill(Area::rect((2.0, 2.0), (width - 4.0, height - 4.0)), Fill::solid(Color::from_hsv(hue.value(), 1.0, 1.0))).apply(&mut image);
        image
      }
    });
    let alpha_mark = layer("color-picker-strength-mark", move |size| {
      let (width, height) = (size.0 as f32, size.1 as f32);
      let mut image = Pixels::new(size.0, size.1);
      fill(Area::rect((0.0, 0.0), (width, height)), Fill::solid(shadow)).apply(&mut image);
      fill(Area::rect((1.0, 1.0), (width - 2.0, height - 2.0)), Fill::solid(ring_color)).apply(&mut image);
      image
    });

    let root = Container::new();
    for spot in [&square, &veil, &hue_bar, &checkerboard, &fade, &ring_mark, &hue_mark, &alpha_mark] {
      root.add(spot);
    }
    let parts = Arc::new(Parts {
      hue,
      saturation,
      brightness,
      alpha,
      size: root.size().clone(),
      strength,
      density,
      square,
      veil,
      hue_bar,
      checkerboard,
      fade,
      ring_mark,
      hue_mark,
      alpha_mark,
    });
    parts.arrange();
    root.keep(root.size().subscribe({
      let parts = Arc::clone(&parts);
      move |_| parts.arrange()
    }));

    // The part a drag began on. A drag keeps moving that part until the finger lifts, and stops at its edge.
    let began_on = Mutex::new(None::<Part>);
    root.subject::<Drag>().subscribe({
      let parts = Arc::clone(&parts);
      move |drag| match drag {
        Drag::Start { x, y } => {
          let part = Layout::new(parts.size.peek(), parts.strength, parts.density).part_at(*x, *y);
          *began_on.lock().unwrap() = part;
          if let Some(part) = part {
            parts.drag(part, (*x, *y));
          }
        }
        Drag::Move { x, y } => {
          let part = *began_on.lock().unwrap();
          if let Some(part) = part {
            parts.drag(part, (*x, *y));
          }
        }
        Drag::End { .. } => *began_on.lock().unwrap() = None,
      }
    });

    Arc::new(Self { root, parts })
  }

  /// The hue now, 0 to 360.
  pub fn hue(&self) -> f64 {
    self.parts.hue.peek() as f64
  }

  /// How colorful the color is now, 0 to 1.
  pub fn saturation(&self) -> f64 {
    self.parts.saturation.peek() as f64
  }

  /// How bright the color is now, 0 to 1.
  pub fn brightness(&self) -> f64 {
    self.parts.brightness.peek() as f64
  }

  /// How solid the color is now, 0 to 255 (the starting value when the picker has no bar for it).
  pub fn alpha(&self) -> u8 {
    self.parts.alpha.peek()
  }

  /// Moves the marks to this color, as when a swatch is chosen or a number is typed. The `hue` is 0 to 360, the other two 0 to 1.
  pub fn set_color(&self, hue: f64, saturation: f64, brightness: f64) {
    self.parts.hue.next((hue as f32).rem_euclid(360.0));
    self.parts.saturation.next((saturation as f32).clamp(0.0, 1.0));
    self.parts.brightness.next((brightness as f32).clamp(0.0, 1.0));
    self.parts.arrange();
  }

  /// Moves the mark of the strength bar, 0 to 255.
  pub fn set_alpha(&self, alpha: u8) {
    self.parts.alpha.next(alpha);
    self.parts.arrange();
  }
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;
  use crate::components::pixel;

  /// A picker 240 pixels wide and 300 high with a strength bar. The parts sit 17 pixels in from each edge: a square of 138 (206 - 56 - 12) at
  /// (17, 17), the color bar from x = 167, and the strength bar from y = 167.
  fn picker(p_strength: bool) -> Arc<ColorPicker> {
    let picker = ColorPicker::new(0.0, 1.0, 1.0, p_strength, 255, vec![255, 255, 255, 255], vec![0, 0, 0, 200], 1.0);
    picker.size().next((240, 300));
    picker
  }

  fn render(p_picker: &ColorPicker) -> Frame {
    let mut component = p_picker.component().clone();
    component.render(Duration::ZERO)
  }

  fn touch(p_picker: &ColorPicker, p_x: f32, p_y: f32, p_pressed: Option<bool>) {
    p_picker.next(PointerEvent::Moved { x: p_x, y: p_y });
    if let Some(pressed) = p_pressed {
      p_picker.next(PointerEvent::Button { button: PointerButton::Left, pressed });
    }
  }

  #[test]
  fn a_touch_on_the_square_sets_how_colorful_and_how_bright_and_keeps_following_past_its_edge() {
    let picker = picker(true);
    touch(&picker, 51.5, 51.5, Some(true));
    assert!((picker.saturation() - 0.25).abs() < 0.01 && (picker.brightness() - 0.75).abs() < 0.01);
    touch(&picker, 900.0, 900.0, None);
    assert_eq!((picker.saturation(), picker.brightness()), (1.0, 0.0), "held at the corner");
    assert_eq!(picker.hue(), 0.0, "the square does not change the hue");
    touch(&picker, 900.0, 900.0, Some(false));
    touch(&picker, 30.0, 30.0, None);
    assert_eq!(picker.saturation(), 1.0, "let go");
  }

  #[test]
  fn the_color_bar_sets_the_hue_and_the_strength_bar_how_solid() {
    let picker = picker(true);
    touch(&picker, 180.0, 86.0, Some(true));
    assert!((picker.hue() - 180.0).abs() < 1.0, "halfway down is the opposite color: {}", picker.hue());
    touch(&picker, 180.0, 86.0, Some(false));
    touch(&picker, 120.0, 181.0, Some(true));
    assert!((picker.alpha() as i32 - 128).abs() <= 1, "{}", picker.alpha());
    touch(&picker, 120.0, 181.0, Some(false));
    // A touch between the parts is nothing.
    touch(&picker, 161.0, 50.0, Some(true));
    assert!((picker.alpha() as i32 - 128).abs() <= 1);
  }

  #[test]
  fn a_picker_without_the_strength_bar_has_nothing_under_the_square() {
    let picker = picker(false);
    touch(&picker, 100.0, 200.0, Some(true));
    assert_eq!(picker.alpha(), 255);
    picker.set_color(200.0, 0.3, 0.4);
    assert_eq!((picker.hue(), picker.alpha()), (200.0, 255));
    picker.set_alpha(10);
    assert_eq!(picker.alpha(), 10);
  }

  #[test]
  fn abra_draws_the_square_in_the_colors_it_makes() {
    let frame = render(&picker(true));
    // The top right of the square is the pure hue (red), the top left is white, and the bottom is black.
    let right = pixel(&frame, 147, 37);
    assert!(right[0] > 200 && right[1] < 60, "{right:?}");
    let left = pixel(&frame, 22, 37);
    assert!(left[0] > 200 && left[1] > 200, "{left:?}");
    assert!(pixel(&frame, 80, 147)[0] < 40);
    // A quarter of the way down the bar of colors is between yellow and green.
    let bar = pixel(&frame, 190, 51);
    assert!(bar[1] > 200 && bar[2] < 60, "{bar:?}");
  }

  #[test]
  fn dragging_moves_the_ring_and_leaves_no_ring_behind() {
    let picker = picker(true);
    // The ring starts on the top right corner of the square, (155, 17), and its white line is 15 pixels to the left of the middle.
    let before = render(&picker);
    assert_eq!(pixel(&before, 140, 17), [255, 255, 255, 255]);
    touch(&picker, 51.5, 51.5, Some(true));
    let after = render(&picker);
    assert_eq!(pixel(&after, 37, 52), [255, 255, 255, 255], "the ring is at the finger");
    assert_ne!(pixel(&after, 140, 17), [255, 255, 255, 255], "and not where it was");
  }

  #[test]
  fn changing_the_hue_changes_the_square() {
    let picker = picker(true);
    picker.set_color(120.0, 1.0, 1.0);
    let frame = render(&picker);
    let right = pixel(&frame, 147, 37);
    assert!(right[1] > 200 && right[0] < 60, "green now: {right:?}");
  }

  /// Dragging on one thread while another draws must not leave either waiting for the other. A hang fails the test at the timeout.
  #[test]
  fn dragging_while_another_thread_draws_does_not_wait_forever() {
    let subject = picker(true);
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
        touch(&subject, 51.5, 51.5, Some(true));
        for step in 0..400u32 {
          touch(&subject, 20.0 + (step % 150) as f32, 20.0 + (step % 90) as f32, None);
          subject.set_alpha(step as u8);
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
    let subject = picker(true);
    let weak = subject.component().downgrade();
    render(&subject);
    touch(&subject, 51.5, 51.5, Some(true));
    drop(subject);
    assert!(weak.upgrade().is_none(), "the color picker is still alive after it was dropped: something in it holds itself");
  }

  /// How long a frame takes while the picker is dragged on a phone-sized surface: a finger on the square (the ring moves, the strength bar
  /// changes color) and a finger on the bar of colors (the square is drawn again). Run with
  /// `cargo test -p alakazam-mobile --lib timing_of_a_frame_while_dragging -- --ignored --nocapture`.
  #[test]
  #[ignore]
  fn timing_of_a_frame_while_dragging() {
    let picker = ColorPicker::new(0.0, 1.0, 1.0, true, 255, vec![255, 255, 255, 255], vec![0, 0, 0, 200], 3.0);
    picker.size().next((1000, 1200));
    render(&picker);
    let average = |times: &[f64]| times.iter().sum::<f64>() / times.len() as f64;
    // The square: 1000 - 56 - 12 - 2 * 51 wide.
    touch(&picker, 300.0, 300.0, Some(true));
    let mut square = Vec::new();
    for step in 0..100u32 {
      touch(&picker, 200.0 + (step % 50) as f32 * 6.0, 200.0 + (step % 40) as f32 * 6.0, None);
      let started = std::time::Instant::now();
      render(&picker);
      square.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    touch(&picker, 300.0, 300.0, Some(false));
    println!("a frame while the square is dragged: {:.3} ms", average(&square));
    // The bar of colors is at the right of the square.
    touch(&picker, 900.0, 300.0, Some(true));
    let mut hues = Vec::new();
    for step in 0..100u32 {
      touch(&picker, 900.0, 100.0 + (step % 60) as f32 * 8.0, None);
      let started = std::time::Instant::now();
      render(&picker);
      hues.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    println!("a frame while the bar of colors is dragged: {:.3} ms, worst {:.3} ms", average(&hues), hues.iter().cloned().fold(0.0, f64::max));
  }
}
