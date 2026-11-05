use std::sync::{Arc, Mutex};

use abra::abra_core::{Color, ColorStop, Gradient, Image as Pixels, Path};
use vessel::prelude::*;

use crate::components::surface::{PictureSource, Shared, blank, checkerboard, color_of, rect, ring, shade};

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

/// What the picker shows: the color as the hue (0 to 360), how colorful it is (0 to 1), how bright (0 to 1) and how solid (0 to 255), and whether there is a
/// bar for how solid it is.
#[derive(Clone, Copy)]
struct State {
  hue: f32,
  saturation: f32,
  brightness: f32,
  alpha: u8,
  strength: bool,
  /// The marks on the parts: a ring of the first color on a halo of the second.
  ring: Color,
  shadow: Color,
  density: f32,
}

/// Where the three parts of the picker are on a surface.
#[derive(Clone, Copy)]
struct Layout {
  /// The side of the square that picks how colorful and how bright.
  side: f32,
  /// Whether there is a bar under the square for how solid the color is.
  strength: bool,
  width: f32,
}

impl Layout {
  fn new(p_size: (u32, u32), p_strength: bool) -> Self {
    let (width, height) = (p_size.0 as f32, p_size.1 as f32);
    let below = if p_strength { STRENGTH_HEIGHT + GAP } else { 0.0 };
    Self { side: (width - HUE_BAR_WIDTH - GAP).min(height - below).max(20.0), strength: p_strength, width }
  }

  fn hue_bar_left(&self) -> f32 {
    self.side + GAP
  }

  fn strength_top(&self) -> f32 {
    self.side + GAP
  }

  /// Which part a place on the surface is in, from where a touch begins.
  fn part_at(&self, p_x: f32, p_y: f32) -> Option<Part> {
    if p_x <= self.side && p_y <= self.side {
      Some(Part::Square)
    } else if p_x >= self.hue_bar_left() && p_y <= self.side {
      Some(Part::Hue)
    } else if self.strength && p_y >= self.strength_top() {
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

/// The picture of the picker: abra's gradients for the square, the bar of hues and the bar of strengths, and the marks on them.
fn paint(p_state: &State, p_size: (u32, u32)) -> Pixels {
  let State { hue, saturation, brightness, alpha, strength, ring: ring_color, shadow, density } = *p_state;
  let layout = Layout::new(p_size, strength);
  let side = layout.side;
  let mut image = blank(p_size);

  // The square: white to the pure color across, and under it a veil from clear to black down.
  shade(&mut image, (0.0, 0.0), (side, side), &Gradient::from_to(Color::white(), Color::from_hsv(hue, 1.0, 1.0)));
  shade(
    &mut image,
    (0.0, 0.0),
    (side, side),
    &Gradient::from_to(Color::from_rgba(0, 0, 0, 0), Color::black()).with_direction(along((0.0, 0.0), (0.0, side))),
  );
  let center = (saturation * side, (1.0 - brightness) * side);
  ring(&mut image, center, (RING_RADIUS + 1.0) * density, 4.0 * density, shadow);
  ring(&mut image, center, RING_RADIUS * density, 2.0 * density, ring_color);

  // The bar of colors, with a mark across it where the hue is.
  let bar_left = layout.hue_bar_left();
  let hues = Gradient::evenly(HUES.iter().map(|hue| Color::from_hsv(*hue, 1.0, 1.0)).collect()).with_direction(along((0.0, 0.0), (0.0, side)));
  shade(&mut image, (bar_left, 0.0), (HUE_BAR_WIDTH, side), &hues);
  let mark_y = hue / 360.0 * side;
  rect(&mut image, (bar_left - 2.0, mark_y - 4.0), (HUE_BAR_WIDTH + 4.0, 8.0), shadow);
  rect(&mut image, (bar_left - 1.0, mark_y - 3.0), (HUE_BAR_WIDTH + 2.0, 6.0), ring_color);
  rect(&mut image, (bar_left, mark_y - 2.0), (HUE_BAR_WIDTH, 4.0), Color::from_hsv(hue, 1.0, 1.0));

  if strength {
    // The color going from not there to solid, over a checkerboard.
    let top = layout.strength_top();
    let color = Color::from_hsv(hue, saturation, brightness);
    checkerboard(&mut image, (0.0, top), (layout.width, STRENGTH_HEIGHT), CELL);
    shade(
      &mut image,
      (0.0, top),
      (layout.width, STRENGTH_HEIGHT),
      &Gradient::new(vec![ColorStop::new(Color::from_rgba(color.r, color.g, color.b, 0), 0.0), ColorStop::new(color, 1.0)]),
    );
    let x = alpha as f32 / 255.0 * layout.width;
    rect(&mut image, (x - 4.0, top - 2.0), (8.0, STRENGTH_HEIGHT + 4.0), shadow);
    rect(&mut image, (x - 3.0, top - 1.0), (6.0, STRENGTH_HEIGHT + 2.0), ring_color);
  }
  image
}

/// The color picker's surface: a square to drag a finger over for how colorful and how bright the color is, a bar beside it for which color it is, and,
/// when asked, a bar under it for how solid it is. A touch that begins on a part keeps moving that part until the finger lifts, and stops at its edge.
/// abra draws the picture and Vessel renders it and brings the touches; the swatches, the number boxes, the web color and the buttons are the app's.
///
/// Read `hue`, `saturation`, `brightness` and `alpha` while the finger moves and when it lifts; the app changes them with `set_color` and `set_alpha`.
#[derive(uniffi::Object, Component)]
pub struct ColorSquare {
  image: Image,
  shared: Arc<Shared<State>>,
}

#[uniffi::export]
impl ColorSquare {
  /// A picker that starts at this color: the `hue` from 0 to 360, `saturation` (how colorful) and `brightness` from 0 to 1. With `strength` it has the bar
  /// for how solid the color is, starting at `alpha` (0 to 255). `ring` and `shadow` are colors, each as red, green, blue and alpha from 0 to 255, for the
  /// marks on the parts: the marks are a ring of the first color on a halo of the second. `density` converts the handle's dp measurements to surface pixels.
  ///
  /// # Panics
  /// When a color has fewer than four numbers.
  #[uniffi::constructor]
  pub fn new(hue: f64, saturation: f64, brightness: f64, strength: bool, alpha: u8, ring: Vec<u8>, shadow: Vec<u8>, density: f64) -> Arc<Self> {
    let shared = Shared::new(State {
      hue: (hue as f32).rem_euclid(360.0),
      saturation: (saturation as f32).clamp(0.0, 1.0),
      brightness: (brightness as f32).clamp(0.0, 1.0),
      alpha,
      strength,
      ring: color_of(&ring),
      shadow: color_of(&shadow),
      density: (density as f32).max(0.1),
    });
    let image = Image::new().track(&PictureSource::new(&shared, paint));

    // The touch: which part it began on, and where the finger is.
    let finger = Mutex::new(((0.0f32, 0.0f32), None::<Part>));
    let this = image.downgrade();
    image.subject::<PointerEvent>().subscribe({
      let shared = Arc::clone(&shared);
      move |event| {
        let Some(image) = this.upgrade() else { return };
        let layout = Layout::new(image.size().value(), strength);
        let mut finger = finger.lock().unwrap();
        let drag = |p_part: Part, p_at: (f32, f32)| {
          shared.update(|state| match p_part {
            Part::Square => {
              state.saturation = (p_at.0 / layout.side).clamp(0.0, 1.0);
              state.brightness = 1.0 - (p_at.1 / layout.side).clamp(0.0, 1.0);
            }
            Part::Hue => state.hue = ((p_at.1 / layout.side).clamp(0.0, 1.0) * 360.0).min(359.999),
            Part::Strength => state.alpha = ((p_at.0 / layout.width).clamp(0.0, 1.0) * 255.0).round() as u8,
          });
        };
        match event {
          PointerEvent::Moved { x, y } => {
            finger.0 = (*x, *y);
            if let Some(part) = finger.1 {
              drag(part, (*x, *y));
            }
          }
          PointerEvent::Button { button: PointerButton::Left, pressed: true } => {
            finger.1 = layout.part_at(finger.0.0, finger.0.1);
            if let Some(part) = finger.1 {
              drag(part, finger.0);
            }
          }
          PointerEvent::Button { button: PointerButton::Left, pressed: false } => finger.1 = None,
          PointerEvent::Button { .. } => {}
        }
      }
    });

    Arc::new(Self { image, shared })
  }

  /// The hue now, 0 to 360.
  pub fn hue(&self) -> f64 {
    self.shared.get().hue as f64
  }

  /// How colorful the color is now, 0 to 1.
  pub fn saturation(&self) -> f64 {
    self.shared.get().saturation as f64
  }

  /// How bright the color is now, 0 to 1.
  pub fn brightness(&self) -> f64 {
    self.shared.get().brightness as f64
  }

  /// How solid the color is now, 0 to 255 (the starting value when the picker has no bar for it).
  pub fn alpha(&self) -> u8 {
    self.shared.get().alpha
  }

  /// Moves the marks to this color, as when a swatch is chosen or a number is typed. The `hue` is 0 to 360, the other two 0 to 1.
  pub fn set_color(&self, hue: f64, saturation: f64, brightness: f64) {
    self.shared.update(|state| {
      state.hue = (hue as f32).rem_euclid(360.0);
      state.saturation = (saturation as f32).clamp(0.0, 1.0);
      state.brightness = (brightness as f32).clamp(0.0, 1.0);
    });
  }

  /// Moves the mark of the strength bar, 0 to 255.
  pub fn set_alpha(&self, alpha: u8) {
    self.shared.update(|state| state.alpha = alpha);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::components::surface::pixel;

  /// A picker 200 pixels wide and 240 high with a strength bar: a square of 160 (200 - 28 - 12) at the top left, the color bar at x = 172, and the
  /// strength bar from y = 172.
  fn picker(p_strength: bool) -> Arc<ColorSquare> {
    let picker = ColorSquare::new(0.0, 1.0, 1.0, p_strength, 255, vec![255, 255, 255, 255], vec![0, 0, 0, 200], 1.0);
    picker.size().next((200, 240));
    picker
  }

  fn touch(p_picker: &ColorSquare, p_x: f32, p_y: f32, p_pressed: Option<bool>) {
    p_picker.next(PointerEvent::Moved { x: p_x, y: p_y });
    if let Some(pressed) = p_pressed {
      p_picker.next(PointerEvent::Button { button: PointerButton::Left, pressed });
    }
  }

  #[test]
  fn a_touch_on_the_square_sets_how_colorful_and_how_bright_and_keeps_following_past_its_edge() {
    let picker = picker(true);
    touch(&picker, 40.0, 40.0, Some(true));
    assert!((picker.saturation() - 0.25).abs() < 0.01 && (picker.brightness() - 0.75).abs() < 0.01);
    touch(&picker, 900.0, 900.0, None);
    assert_eq!((picker.saturation(), picker.brightness()), (1.0, 0.0), "held at the corner");
    assert_eq!(picker.hue(), 0.0, "the square does not change the hue");
    touch(&picker, 900.0, 900.0, Some(false));
    touch(&picker, 10.0, 10.0, None);
    assert_eq!(picker.saturation(), 1.0, "let go");
  }

  #[test]
  fn the_color_bar_sets_the_hue_and_the_strength_bar_how_solid() {
    let picker = picker(true);
    touch(&picker, 180.0, 80.0, Some(true));
    assert!((picker.hue() - 180.0).abs() < 1.0, "halfway down is the opposite color: {}", picker.hue());
    touch(&picker, 180.0, 80.0, Some(false));
    touch(&picker, 100.0, 185.0, Some(true));
    assert!((picker.alpha() as i32 - 128).abs() <= 1, "{}", picker.alpha());
    touch(&picker, 100.0, 185.0, Some(false));
    // A touch between the parts is nothing.
    touch(&picker, 165.0, 50.0, Some(true));
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
    let picker = picker(true);
    let image = paint(&picker.shared.get(), (200, 240));
    // The top right of the square is the pure hue (red), the top left is white, and the bottom is black.
    let right = pixel(&image, 150, 20);
    assert!(right[0] > 200 && right[1] < 60, "{right:?}");
    let left = pixel(&image, 5, 20);
    assert!(left[0] > 200 && left[1] > 200, "{left:?}");
    assert!(pixel(&image, 80, 158)[0] < 40);
    // A quarter of the way down the bar of colors is between yellow and green.
    let bar = pixel(&image, 184, 40);
    assert!(bar[1] > 200 && bar[2] < 60, "{bar:?}");
  }
}
