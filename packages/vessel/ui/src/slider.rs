use std::sync::{Arc, Mutex};

use vessel_api::prelude::*;
use vessel_macros::Component;

use crate::image::{Fit, fitted};

/// The thickness of the track, in pixels, unless set with `track_thickness`.
const TRACK: u32 = 10;
/// The size of the thumb, in pixels, or less if the slider is smaller, unless set with `thumb_size`.
const THUMB: u32 = 20;
/// How much an arrow key moves the slider, as a fraction of its range, unless set with `key_step`.
const STEP: f32 = 0.05;

/// Sent on a [`Slider`] with the new value (0 to 1) when the user moves it, by dragging or with the arrow keys. It is not
/// sent when the program calls `set_value`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Changed(pub f32);

/// The track picture made ready to draw: stretched to the slider's width and its ends rounded. It is the same for every position of the
/// thumb, so it is made once for a picture and a size and kept (one at most, so a replaced picture is let go).
struct PreparedTrack {
  /// The picture it was made from, which names it: another picture is another track.
  source: Arc<Frame>,
  /// The room it was made for: the slider's width and the height of the band.
  room: (u32, u32),
  /// Where it starts across, its size, and its pixels.
  x: i32,
  width: u32,
  height: u32,
  pixels: Arc<Vec<u8>>,
}

/// The width of the thumb: its picture's width if it has one (never wider than the slider), otherwise `p_size`.
fn thumb_width(p_width: u32, p_height: u32, p_size: u32, p_picture: &Option<Arc<Frame>>) -> u32 {
  match p_picture {
    Some(picture) => picture.width.min(p_width).max(1),
    None => p_height.min(p_width).min(p_size).max(1),
  }
}

/// A horizontal slider holding a value from 0 to 1. Press anywhere on it to jump there and drag to move; Left and Right
/// move it a little while it has the keyboard. Listen with `slider.subscribe(|changed: &Changed| ..)`.
///
/// It looks like the platform's slider unless you give it pictures: `track_image` is drawn across the whole slider in place
/// of the track (a color gradient for a color picker, say), and `thumb_image` in place of the round thumb, at its own size.
/// It works the same either way.
#[derive(Clone, Component)]
#[component(plain)]
pub struct Slider {
  component: Component,
  value: BehaviorSubject<f32>,
  color: BehaviorSubject<Option<[u8; 4]>>,
  track_image: BehaviorSubject<Option<Arc<Frame>>>,
  thumb_image: BehaviorSubject<Option<Arc<Frame>>>,
  track_thickness: BehaviorSubject<u32>,
  thumb_size: BehaviorSubject<u32>,
  key_step: BehaviorSubject<f32>,
}

impl Slider {
  component_builders!();
  color_background_builder!();

  /// A slider at `p_value` (clamped to 0 to 1), in the theme's accent and track colors. Set `color` to use another accent.
  pub fn new(p_value: f32) -> Self {
    let value = BehaviorSubject::new(p_value.clamp(0.0, 1.0));
    let color = BehaviorSubject::new(None);
    let track_image = BehaviorSubject::new(None::<Arc<Frame>>);
    let thumb_image = BehaviorSubject::new(None::<Arc<Frame>>);
    let track_thickness = BehaviorSubject::new(TRACK);
    let thumb_size = BehaviorSubject::new(THUMB);
    let key_step = BehaviorSubject::new(STEP);

    let component = Component::new("slider");
    let prepared_track = Mutex::new(None::<PreparedTrack>);
    component.subject::<Canvas>().subscribe({
      let (value, color) = (value.clone(), color.clone());
      let (track_image, thumb_image) = (track_image.clone(), thumb_image.clone());
      let (track_thickness, thumb_size) = (track_thickness.clone(), thumb_size.clone());
      move |canvas| {
        let (width, height) = (canvas.width(), canvas.height());
        let (track_picture, thumb_picture) = (track_image.value(), thumb_image.value());
        let track = track_thickness.value().max(2);
        // The thumb is kept inside the slider, so the ends of the track sit under it.
        let thumb = thumb_width(width, height, thumb_size.value(), &thumb_picture);
        let radius = thumb as f32 / 2.0;
        let travel = width as f32 - thumb as f32;
        let center = radius + value.value() * travel;
        let theme = canvas.theme();
        let fill = color.value().unwrap_or(theme.accent);

        if let Some(frame) = track_picture.as_ref() {
          // The picture's own height is the bar's thickness, stretched to the slider's width, with rounded ends like the
          // standard track and a faint outline. It does not depend on where the thumb is, so it is not made again for each move.
          let band = frame.height.min(height).max(1);
          let ready = {
            let mut kept = prepared_track.lock().unwrap();
            if !kept.as_ref().is_some_and(|track| Arc::ptr_eq(&track.source, frame) && track.room == (width, band)) {
              *kept = fitted(frame, width, band, Fit::Stretch).map(|(x, _, bar_width, bar_height, mut pixels)| {
                let rounding = bar_height as f32 / 2.0;
                for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
                  let (px, py) = ((index as u32 % bar_width) as f32 + 0.5, (index as u32 / bar_width) as f32 + 0.5);
                  // How far this pixel is outside the pill shape, which only the two rounded ends can be.
                  let across = (rounding - px).max(px - (bar_width as f32 - rounding)).max(0.0);
                  let distance = (across * across + (py - rounding) * (py - rounding)).sqrt();
                  pixel[3] = (pixel[3] as f32 * (rounding - distance + 0.5).clamp(0.0, 1.0)) as u8;
                }
                PreparedTrack { source: Arc::clone(frame), room: (width, band), x, width: bar_width, height: bar_height, pixels: Arc::new(pixels) }
              });
            }
            kept.as_ref().map(|track| (track.x, track.width, track.height, Arc::clone(&track.pixels)))
          };
          if let Some((x, bar_width, bar_height, pixels)) = ready {
            let rounding = bar_height as f32 / 2.0;
            let top = (height as i32 - bar_height as i32) / 2;
            canvas.draw_pixels(x, top, bar_width, bar_height, &pixels);
            let [red, green, blue, _] = theme.border;
            canvas.stroke_rounded_rect(x, top, bar_width, bar_height, rounding, 1.0, [red, green, blue, 110]);
          }
        } else {
          // An empty track with a faint outline, and the filled part just inside it.
          let track_top = (height as i32 - track as i32) / 2;
          let rounding = track as f32 / 2.0;
          let [red, green, blue, _] = theme.border;
          canvas.fill_rounded_rect(0, track_top, width, track, rounding, theme.track);
          canvas.fill_rounded_rect(1, track_top + 1, (center as u32).max(1) - 1, track - 2, rounding - 1.0, fill);
          canvas.stroke_rounded_rect(0, track_top, width, track, rounding, 1.0, [red, green, blue, 110]);
        }

        let left = (center - radius).round() as i32;
        match &thumb_picture {
          Some(frame) => {
            let thumb_height = frame.height.min(height).max(1);
            let top = (height as i32 - thumb_height as i32) / 2;
            canvas.draw_pixels(left, top, thumb, thumb_height, frame.pixels.as_slice());
          }
          None => {
            let top = (height as i32 - thumb as i32) / 2;
            canvas.fill_rounded_rect(left, top, thumb, thumb, radius, fill);
          }
        }
      }
    });

    // Moves the value to where the pointer is, and says so. The thumb's center travels between half a thumb in from each
    // end, so that is the range the position maps to.
    let this = component.downgrade();
    let moved_to = {
      let (value, thumb_image, thumb_size) = (value.clone(), thumb_image.clone(), thumb_size.clone());
      move |p_x: f32| {
        let Some(slider) = this.upgrade() else { return };
        let (width, height) = slider.size().value();
        let thumb = thumb_width(width, height, thumb_size.value(), &thumb_image.value()) as f32;
        let travel = (width as f32 - thumb).max(1.0);
        let new = ((p_x - thumb / 2.0) / travel).clamp(0.0, 1.0);
        if new != value.value() {
          value.next(new);
          slider.next(Changed(new));
        }
      }
    };

    component.subject::<Drag>().subscribe(move |drag| match drag {
      Drag::Start { x, .. } | Drag::Move { x, .. } => moved_to(*x),
      Drag::End { .. } => {}
    });

    let this = component.downgrade();
    component.subject::<KeyEvent>().subscribe({
      let (value, key_step) = (value.clone(), key_step.clone());
      move |event| {
        let step = key_step.value();
        let step = match (event.pressed, event.key.as_str()) {
          (true, "ArrowLeft") => -step,
          (true, "ArrowRight") => step,
          _ => return,
        };
        let new = (value.value() + step).clamp(0.0, 1.0);
        if new != value.value() {
          value.next(new);
          if let Some(slider) = this.upgrade() {
            slider.next(Changed(new));
          }
        }
      }
    });

    Self {
      component,
      value,
      color,
      track_image,
      thumb_image,
      track_thickness,
      thumb_size,
      key_step,
    }
  }

  /// The user's moves as a stream of [`Changed`].
  pub fn changes(&self) -> Subject<Changed> {
    self.component.subject::<Changed>()
  }

  /// The value now, from 0 to 1. Read in a `draw`, it makes that component redraw when the slider moves.
  pub fn value(&self) -> f32 {
    self.value.value()
  }

  /// Sets the value (clamped to 0 to 1) and redraws. This does not send [`Changed`], which is for the user's moves.
  pub fn set_value(&self, p_value: f32) {
    self.value.next(p_value.clamp(0.0, 1.0));
  }

  /// Sets the value, and returns the slider so calls can be chained.
  pub fn with_value(self, p_value: f32) -> Self {
    self.set_value(p_value);
    self
  }

  properties! {
    color: [u8; 4] => set_color, with_color;
    track_image: Arc<Frame> => set_track_image, with_track_image;
    thumb_image: Arc<Frame> => set_thumb_image, with_thumb_image;
    track_thickness: u32 => set_track_thickness, with_track_thickness;
    thumb_size: u32 => set_thumb_size, with_thumb_size;
    key_step: f32 => set_key_step, with_key_step;
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};
  use std::time::Duration;

  use super::*;

  fn sized(p_value: f32) -> Slider {
    let slider = Slider::new(p_value);
    slider.size().next((120, 20));
    slider
  }

  fn listen(p_slider: &Slider) -> Arc<Mutex<Vec<f32>>> {
    let heard = Arc::new(Mutex::new(Vec::new()));
    p_slider.subject::<Changed>().subscribe({
      let heard = Arc::clone(&heard);
      move |changed| heard.lock().unwrap().push(changed.0)
    });
    heard
  }

  fn pointer(p_slider: &Slider, p_x: f32, p_pressed: Option<bool>) {
    p_slider.next(PointerEvent::Moved { x: p_x, y: 10.0 });
    if let Some(pressed) = p_pressed {
      p_slider.next(PointerEvent::Button {
        button: PointerButton::Left,
        pressed,
      });
    }
  }

  fn render(p_slider: &Slider) -> Frame {
    let mut component = p_slider.component.clone();
    component.render(Duration::ZERO)
  }

  fn pixel(p_frame: &Frame, p_x: u32, p_y: u32) -> [u8; 4] {
    let at = ((p_y * p_frame.width + p_x) * 4) as usize;
    p_frame.pixels[at..at + 4].try_into().unwrap()
  }

  #[test]
  fn pressing_jumps_to_that_place_and_dragging_follows_until_it_is_let_go() {
    let slider = sized(0.0);
    let heard = listen(&slider);

    // 120 wide with a 20 pixel thumb: the thumb's center runs from x = 10 to x = 110.
    pointer(&slider, 60.0, Some(true));
    assert!((slider.value() - 0.5).abs() < 0.01, "{}", slider.value());
    pointer(&slider, 110.0, None);
    assert!((slider.value() - 1.0).abs() < 0.01);
    pointer(&slider, 500.0, None); // dragged far past the end
    assert_eq!(slider.value(), 1.0);
    pointer(&slider, 10.0, Some(false)); // let go at the start end: this still moves it
    let after_release = slider.value();
    pointer(&slider, 110.0, None); // moving without a button down does nothing
    assert_eq!(slider.value(), after_release);
    assert!(heard.lock().unwrap().len() >= 2);
  }

  #[test]
  fn set_value_redraws_but_does_not_send_changed_and_is_clamped() {
    let slider = sized(0.25);
    let heard = listen(&slider);
    render(&slider);
    slider.set_value(0.75);
    assert!(slider.component.clone().has_changed());
    assert_eq!(slider.value(), 0.75);
    slider.set_value(5.0);
    assert_eq!(slider.value(), 1.0);
    assert!(heard.lock().unwrap().is_empty());
    assert_eq!(Slider::new(-3.0).value(), 0.0);
  }

  #[test]
  fn arrow_keys_move_it_by_a_step_and_stop_at_the_ends() {
    let slider = sized(0.5);
    let heard = listen(&slider);
    let key = |name: &str, pressed| KeyEvent {
      key: name.to_string(),
      pressed,
    };
    slider.next(key("ArrowRight", true));
    slider.next(key("ArrowRight", false));
    slider.next(key("ArrowLeft", true));
    slider.next(key("a", true));
    assert!((slider.value() - 0.5).abs() < 1e-5);
    assert_eq!(heard.lock().unwrap().len(), 2);

    let end = sized(1.0);
    let end_heard = listen(&end);
    end.next(key("ArrowRight", true));
    assert!(end_heard.lock().unwrap().is_empty(), "already at the end, nothing changed");
  }

  #[test]
  fn the_track_thumb_and_key_step_follow_their_settings() {
    // A bigger step: one press moves a quarter of the range.
    let slider = sized(0.5).with_key_step(0.25);
    slider.next(KeyEvent { key: "ArrowRight".to_string(), pressed: true });
    assert!((slider.value() - 0.75).abs() < 1e-5);

    // A 40 pixel slider with a 30 pixel thumb: the thumb's center runs from x = 15 to x = 105 of 120.
    let slider = Slider::new(0.0).with_thumb_size(30).with_track_thickness(4);
    slider.size().next((120, 40));
    pointer(&slider, 15.0, Some(true));
    assert!(slider.value() < 0.01, "{}", slider.value());
    pointer(&slider, 105.0, None);
    assert!((slider.value() - 1.0).abs() < 0.01, "{}", slider.value());
    let frame = render(&slider);
    let theme = Theme::light();
    // The thumb is 30 tall and the empty track 4: count the rows each lights in its own column.
    let rows = |x: u32, color: [u8; 4]| (0..40).filter(|row| pixel(&frame, x, *row) == color).count();
    assert!(rows(105, theme.accent) >= 28, "the thumb is about 30 pixels tall");
    slider.set_value(1.0);
    slider.set_thumb_size(10);
    let frame = render(&slider);
    let thin = (0..40).filter(|row| pixel(&frame, 112, *row) == theme.accent).count();
    assert!(thin <= 10, "a smaller thumb: {thin}");
  }

  #[test]
  fn the_thumb_and_track_are_centered_up_and_down_in_a_taller_slider() {
    let slider = Slider::new(0.5);
    slider.size().next((120, 40));
    let frame = render(&slider);
    let theme = Theme::light();
    // Rows lit by the thumb (at its middle column) and by the empty track (near the right end), as the first and last.
    let lit = |x: u32, color: [u8; 4]| {
      let rows: Vec<u32> = (0..40).filter(|row| pixel(&frame, x, *row) == color).collect();
      (*rows.first().unwrap(), *rows.last().unwrap())
    };
    let (thumb_top, thumb_bottom) = lit(60, theme.accent);
    let (track_top, track_bottom) = lit(100, theme.track);
    // Centered means as far from the top edge as from the bottom edge (a 40 row slider: first + last = 39).
    assert_eq!(thumb_top + thumb_bottom, 39, "thumb rows {thumb_top}..{thumb_bottom}");
    assert_eq!(track_top + track_bottom, 39, "track rows {track_top}..{track_bottom}");
    assert!(thumb_top < track_top, "the thumb is taller than the track");
  }

  #[test]
  fn the_thumb_and_filled_track_follow_the_value() {
    let slider = sized(0.0);
    let frame = render(&slider);
    let theme = Theme::light();
    assert_eq!(pixel(&frame, 10, 10), theme.accent, "the thumb is at the start");
    assert_eq!(pixel(&frame, 100, 10), theme.track, "the rest of the track is empty");

    slider.set_value(1.0);
    let frame = render(&slider);
    assert_eq!(pixel(&frame, 110, 10), theme.accent, "the thumb is at the end");
    assert_eq!(pixel(&frame, 50, 10), theme.accent, "the track before it is filled");

    slider.set_default_theme(Theme::dark());
    assert_eq!(pixel(&render(&slider), 50, 10), Theme::dark().accent, "it follows the theme");
    slider.set_color([1, 2, 3, 255]);
    assert_eq!(pixel(&render(&slider), 50, 10), [1, 2, 3, 255], "unless it has its own color");
  }

  /// A picture of one flat color.
  fn flat(p_width: u32, p_height: u32, p_color: [u8; 4]) -> Arc<Frame> {
    Arc::new(Frame {
      width: p_width,
      height: p_height,
      pixels: std::sync::Arc::new(p_color.repeat((p_width * p_height) as usize)),
    })
  }

  #[test]
  fn a_track_picture_is_drawn_across_the_slider_and_a_thumb_picture_sits_at_the_value() {
    let red = [200, 0, 0, 255];
    let blue = [0, 0, 200, 255];
    let slider = sized(0.0).with_track_image(flat(4, 4, red)).with_thumb_image(flat(10, 20, blue));
    let frame = render(&slider);
    assert_eq!(pixel(&frame, 100, 10), red, "the track picture fills the slider");
    assert_eq!(pixel(&frame, 5, 10), blue, "the thumb picture is at the start");
    assert_eq!(pixel(&frame, 30, 10), red);

    slider.set_value(1.0);
    let frame = render(&slider);
    assert_eq!(pixel(&frame, 115, 10), blue, "and at the end");
    assert_eq!(pixel(&frame, 30, 10), red, "the track shows again where it was");
  }

  #[test]
  fn dragging_uses_the_width_of_the_thumb_picture() {
    // 120 wide with a 10 pixel thumb: its center runs from x = 5 to x = 115.
    let slider = sized(0.0).with_thumb_image(flat(10, 20, [0, 0, 200, 255]));
    let heard = listen(&slider);
    pointer(&slider, 60.0, Some(true));
    assert!((slider.value() - 0.5).abs() < 0.01, "{}", slider.value());
    pointer(&slider, 115.0, Some(false));
    assert!((slider.value() - 1.0).abs() < 0.01);
    assert!(!heard.lock().unwrap().is_empty());
  }

  #[test]
  fn a_track_made_for_one_picture_and_size_is_not_used_for_another() {
    let red = [200, 0, 0, 255];
    let green = [0, 200, 0, 255];
    let slider = sized(0.5).with_track_image(flat(4, 4, red));
    assert_eq!(pixel(&render(&slider), 100, 10), red);
    // The thumb moves: the same track is drawn.
    slider.set_value(0.0);
    assert_eq!(pixel(&render(&slider), 100, 10), red);
    // Another picture is another track.
    slider.set_track_image(flat(4, 4, green));
    assert_eq!(pixel(&render(&slider), 100, 10), green);
    // Another size is another track: the picture is stretched across the new width.
    slider.size().next((240, 20));
    assert_eq!(pixel(&render(&slider), 200, 10), green);
  }

  /// How long a drag takes to draw on a phone-sized slider with a picture for the track: the thumb moves a little each frame, so the
  /// track is the same each time. Run with `cargo test -p vessel-ui --release timing -- --ignored --nocapture`.
  #[test]
  #[ignore]
  fn timing_of_a_frame_while_dragging_a_slider_with_a_track_picture() {
    let slider = Slider::new(0.0).with_track_image(flat(200, 14, [200, 0, 0, 255])).with_thumb_image(flat(36, 36, [0, 0, 200, 255]));
    slider.size().next((1080, 60));
    render(&slider);
    let mut times = Vec::new();
    for step in 0..300 {
      slider.set_value((step % 100) as f32 / 100.0);
      let started = std::time::Instant::now();
      render(&slider);
      times.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    println!(
      "a slider frame: {:.3} ms on average, worst {:.3} ms",
      times.iter().sum::<f64>() / times.len() as f64,
      times.iter().cloned().fold(0.0, f64::max)
    );
  }
}
