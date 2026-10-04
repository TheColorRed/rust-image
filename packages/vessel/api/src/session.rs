//! Showing a component inside a view that a host application owns.
//!
//! A [`Session`] is the host for an inline component, such as a React Native view: the host application owns the window,
//! and Vessel draws into a surface it registered. Like every host it does three things with the component: it keeps the
//! component's size current, it can send it neutral input events, and the app listens for [`Quit`](crate::Quit) on it.
//! The component never knows it is in a session.
//!
//! A surface often appears after the session starts, so the session keeps the newest frame and draws it the moment a
//! surface is attached.

use std::sync::{Arc, Mutex};

use pub_sub::{Observer, Subject};
use vessel_engine::{Engine, Frame, GpuFrame, GpuTarget, View, surface};

use crate::{Component, Font, Renderable, Theme};

/// A frame that could not be drawn yet because there was no usable surface.
enum Waiting {
  Pixels(Frame),
  Gpu(Arc<dyn GpuFrame>),
}

/// Where finished frames go.
struct Output {
  /// The registered surface frames are drawn on, once one is attached.
  surface_id: Option<i32>,
  /// The newest frame that could not be drawn because there was no usable surface yet.
  waiting: Option<Waiting>,
  /// Draws frames straight from GPU memory. Once there is one, everything shown on the surface goes through it.
  target: Option<Box<dyn GpuTarget>>,
  /// Whether a GPU frame already failed to make a target for the attached surface, so it is not tried on every frame.
  no_target: bool,
  /// Whether anything has been drawn on the surface yet.
  drawn_any: bool,
}

impl Output {
  /// Shows a frame made of pixels. Returns whether it reached the surface.
  fn show_pixels(&mut self, p_frame: &Frame) -> bool {
    if let Some(target) = &mut self.target {
      if target.present_pixels(p_frame) {
        self.drawn_any = true;
        return true;
      }
      // The surface is gone; the next attach makes a new target.
      self.target = None;
      return false;
    }
    // A blank first frame draws nothing worth showing, and drawing it with the CPU would claim the window for the CPU, which
    // keeps it from ever being drawn on by the GPU.
    if !self.drawn_any && p_frame.pixels.chunks_exact(4).all(|pixel| pixel[3] == 0) {
      return true;
    }
    let drawn = self.surface_id.is_some_and(|id| surface::draw_rgba(id, p_frame.width, p_frame.height, &p_frame.pixels));
    self.drawn_any |= drawn;
    drawn
  }

  /// Shows a frame that lives on the GPU: from GPU memory when there is a target, otherwise by reading it back.
  fn show_gpu(&mut self, p_frame: &Arc<dyn GpuFrame>) -> bool {
    // The first GPU frame on a surface decides whether it can be drawn from GPU memory: the frame knows its GPU.
    if self.target.is_none()
      && !self.no_target
      && let Some(id) = self.surface_id
    {
      // A GPU that cannot draw on this window (it may be in use already) is not a reason to stop: the frame is read back.
      self.target = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| p_frame.target_for(id))).ok().flatten();
      self.no_target = self.target.is_none();
    }
    if let Some(target) = &mut self.target {
      if target.present(p_frame.as_ref()) {
        self.drawn_any = true;
        return true;
      }
      self.target = None;
      return false;
    }
    match p_frame.read_back() {
      Some(pixels) => self.show_pixels(&pixels),
      None => false,
    }
  }
}

/// Sent on [`Session::drawn`] each time a frame reaches the surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drawn {
  /// The width in pixels of the frame.
  pub width: u32,
  /// The height in pixels of the frame.
  pub height: u32,
}

/// A running component on a surface that a host application owns. Open it, attach the surface when the host's view
/// appears, and close it when the host is done.
///
/// ```ignore
/// let session = Session::open(&component);
/// session.attach_surface(surface_id);   // call again until it returns true if the view is not ready
/// session.resize(width, height);        // when the host's view changes size
/// session.close();
/// ```
pub struct Session {
  component: Component,
  engine: Mutex<Option<Engine>>,
  /// Kept so the engine keeps running the component.
  view: Mutex<Option<View<()>>>,
  output: Arc<Mutex<Output>>,
  drawn: Subject<Drawn>,
}

impl Session {
  /// Starts running `p_component`. Nothing is drawn until it has a size and a surface is attached.
  pub fn open(p_renderable: &impl Renderable) -> Self {
    let p_component = p_renderable.component();
    let output = Arc::new(Mutex::new(Output {
      surface_id: None,
      waiting: None,
      target: None,
      no_target: false,
      drawn_any: false,
    }));
    let drawn = Subject::new();
    let engine = Engine::with_gpu_sink(
      {
        let (output, drawn) = (Arc::clone(&output), drawn.clone());
        move |_view, frame| {
          let was_drawn = {
            let mut output = output.lock().unwrap();
            let was_drawn = output.show_pixels(frame);
            output.waiting = (!was_drawn).then(|| Waiting::Pixels(frame.clone()));
            was_drawn
          };
          if was_drawn {
            drawn.next(Drawn {
              width: frame.width,
              height: frame.height,
            });
          }
        }
      },
      {
        let (output, drawn) = (Arc::clone(&output), drawn.clone());
        move |_view, frame| {
          let was_drawn = {
            let mut output = output.lock().unwrap();
            let was_drawn = output.show_gpu(frame);
            output.waiting = (!was_drawn).then(|| Waiting::Gpu(Arc::clone(frame)));
            was_drawn
          };
          if was_drawn {
            let (width, height) = frame.size();
            drawn.next(Drawn { width, height });
          }
        }
      },
    );
    let view = View::<()>::new(p_component.name(), p_component.clone());
    engine.add(&view);

    Self {
      component: p_component.clone(),
      engine: Mutex::new(Some(engine)),
      view: Mutex::new(Some(view)),
      output,
      drawn,
    }
  }

  /// Sends a [`Drawn`] each time a frame reaches the surface, so a host can measure how long a change took to show.
  pub fn drawn(&self) -> Subject<Drawn> {
    self.drawn.clone()
  }

  /// The component the session runs, for sending it input and listening to its events.
  pub fn component(&self) -> &Component {
    &self.component
  }

  /// Draws frames on the surface registered under `p_surface_id`. If the component has no size yet it takes the surface's.
  /// The host's view may appear after this call, so call it again until it returns true: that means the surface is usable
  /// and the newest frame is on it.
  pub fn attach_surface(&self, p_surface_id: i32) -> bool {
    // Set before the output is locked: a new size wakes the engine, which needs the output to present.
    if self.component.size().value() == (0, 0) {
      if let Some(surface) = surface::get(p_surface_id) {
        let size = surface.size();
        if size.0 > 0 && size.1 > 0 {
          self.component.size().next(size);
        }
      }
    }

    let waiting = {
      let mut output = self.output.lock().unwrap();
      output.surface_id = Some(p_surface_id);
      output.no_target = false;
      output.waiting.take()
    };
    let Some(waiting) = waiting else {
      return surface::is_alive(p_surface_id);
    };
    let (shown, size) = {
      let mut output = self.output.lock().unwrap();
      match &waiting {
        Waiting::Pixels(frame) => (output.show_pixels(frame), (frame.width, frame.height)),
        Waiting::Gpu(frame) => (output.show_gpu(frame), frame.size()),
      }
    };
    if shown {
      self.drawn.next(Drawn {
        width: size.0,
        height: size.1,
      });
      return true;
    }
    // Put back, unless a newer frame arrived meanwhile.
    self.output.lock().unwrap().waiting.get_or_insert(waiting);
    false
  }

  /// Sets the font the host application uses, for text where the component's app sets none. A session does not guess: the
  /// application the view sits in knows its own font (for example its typeface file), so it passes it here. Until it does,
  /// text uses the built-in bitmap font.
  pub fn set_default_font(&self, p_font: Font) {
    self.component.set_default_font(p_font);
  }

  /// Sets the theme the host application uses, for the same reason as [`set_default_font`](Self::set_default_font): the
  /// application the view sits in knows whether it is dark or light and what its accent color is. Until it says, the light
  /// theme is used.
  pub fn set_default_theme(&self, p_theme: Theme) {
    self.component.set_default_theme(p_theme);
  }

  /// Whether frames are reaching a usable surface: one is attached, it is still alive, and no frame is waiting for it.
  pub fn is_presenting(&self) -> bool {
    let output = self.output.lock().unwrap();
    output.waiting.is_none() && output.surface_id.is_some_and(surface::is_alive)
  }

  /// Tells the component its new size in pixels, after the host's view was resized.
  pub fn resize(&self, p_width: u32, p_height: u32) {
    self.component.size().next((p_width, p_height));
  }

  /// Ends the session: stops the engine and frees what it holds. Safe to call more than once.
  pub fn close(&self) {
    self.view.lock().unwrap().take();
    if let Some(engine) = self.engine.lock().unwrap().take() {
      engine.close();
    }
  }
}

impl Drop for Session {
  fn drop(&mut self) {
    self.close();
  }
}

#[cfg(test)]
mod tests {
  use std::any::Any;
  use std::time::{Duration, Instant};

  use vessel_engine::surface::Surface;

  use pub_sub::prelude::*;

  use super::*;
  use crate::{Canvas, GpuPicture, KeyEvent};

  /// A surface that records what is drawn on it.
  struct Recorder {
    size: (u32, u32),
    frames: Mutex<Vec<(u32, u32, u8)>>,
  }

  impl Surface for Recorder {
    fn is_alive(&self) -> bool {
      true
    }

    fn size(&self) -> (u32, u32) {
      self.size
    }

    fn resize(&self, _p_width: u32, _p_height: u32) {}

    fn draw_rgba(&self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> bool {
      self.frames.lock().unwrap().push((p_width, p_height, p_rgba[0]));
      true
    }

    fn retire(&self) {}

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
      self
    }
  }

  /// Each test uses its own id, since the surface registry is shared by the whole process.
  fn recorder(p_id: i32, p_size: (u32, u32)) -> Arc<Recorder> {
    let recorder = Arc::new(Recorder {
      size: p_size,
      frames: Mutex::new(Vec::new()),
    });
    surface::register_surface(p_id, recorder.clone());
    recorder
  }

  /// Waits until the recorder has a frame that `p_ready` accepts, and returns the newest one.
  fn newest(p_recorder: &Recorder, p_ready: impl Fn((u32, u32, u8)) -> bool) -> (u32, u32, u8) {
    let start = Instant::now();
    loop {
      if let Some(frame) = p_recorder.frames.lock().unwrap().last().copied().filter(|frame| p_ready(*frame)) {
        return frame;
      }
      assert!(start.elapsed() < Duration::from_secs(5), "no matching frame arrived");
      std::thread::sleep(Duration::from_millis(5));
    }
  }

  /// A component that draws with `p_draw`: the one place the tests build a drawing component in a single expression.
  fn drawing(p_component: Component, p_draw: impl Fn(&Canvas) + Send + Sync + 'static) -> Component {
    p_component.subject::<Canvas>().subscribe(p_draw);
    p_component
  }

  fn swatch(p_level: &BehaviorSubject<u8>) -> Component {
    drawing(Component::new("swatch"), {
      let level = p_level.clone();
      move |canvas| canvas.fill([level.value(), 0, 0, 255])
    })
  }

  #[test]
  fn a_frame_made_before_the_surface_appears_is_drawn_when_it_is_attached() {
    let level = BehaviorSubject::new(40u8);
    let session = Session::open(&swatch(&level).with_size(6, 4));
    let surface = recorder(9001, (6, 4));

    // Wait for the engine to make the frame, which has nowhere to go yet.
    let start = Instant::now();
    while session.output.lock().unwrap().waiting.is_none() {
      assert!(start.elapsed() < Duration::from_secs(5), "no frame was made");
      std::thread::sleep(Duration::from_millis(5));
    }
    assert!(surface.frames.lock().unwrap().is_empty());

    assert!(session.attach_surface(9001));
    assert_eq!(newest(&surface, |_| true), (6, 4, 40));
    session.close();
  }

  #[test]
  fn drawn_is_sent_each_time_a_frame_reaches_the_surface() {
    let level = BehaviorSubject::new(1u8);
    let session = Session::open(&swatch(&level).with_size(3, 2));
    let surface = recorder(9005, (3, 2));
    let count = Arc::new(Mutex::new(Vec::new()));
    let _drawn = {
      let count = Arc::clone(&count);
      session.drawn().subscribe(move |drawn| count.lock().unwrap().push(*drawn))
    };
    session.attach_surface(9005);
    newest(&surface, |frame| frame.2 == 1);
    level.next(2);
    newest(&surface, |frame| frame.2 == 2);

    let start = Instant::now();
    while count.lock().unwrap().len() < 2 {
      assert!(start.elapsed() < Duration::from_secs(5), "Drawn was not sent");
      std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(count.lock().unwrap()[0], Drawn { width: 3, height: 2 });
    session.close();
  }

  /// A GPU frame whose surfaces get a target that records into the log.
  struct Picture(Arc<Mutex<Vec<&'static str>>>);

  impl GpuFrame for Picture {
    fn size(&self) -> (u32, u32) {
      (2, 2)
    }

    fn as_any(&self) -> &dyn std::any::Any {
      self
    }

    fn read_back(&self) -> Option<Frame> {
      panic!("a frame must not be copied to the CPU when the surface has a GPU target");
    }

    fn target_for(&self, _surface_id: i32) -> Option<Box<dyn GpuTarget>> {
      Some(Box::new(Target(Arc::clone(&self.0))))
    }
  }

  /// A target that records what it was asked to draw.
  struct Target(Arc<Mutex<Vec<&'static str>>>);

  impl GpuTarget for Target {
    fn present(&mut self, _frame: &dyn GpuFrame) -> bool {
      self.0.lock().unwrap().push("gpu");
      true
    }

    fn present_pixels(&mut self, _frame: &Frame) -> bool {
      self.0.lock().unwrap().push("pixels");
      true
    }
  }

  fn wait_for(p_log: &Mutex<Vec<&'static str>>, p_what: &str, p_count: usize) {
    let start = Instant::now();
    while p_log.lock().unwrap().iter().filter(|entry| **entry == p_what).count() < p_count {
      assert!(start.elapsed() < Duration::from_secs(5), "the target was never asked for {p_what}");
      std::thread::sleep(Duration::from_millis(5));
    }
  }

  #[test]
  fn the_first_gpu_frame_makes_the_target_and_after_that_nothing_is_copied_or_blitted_on_the_cpu() {
    let level = BehaviorSubject::new(1u8);
    let component = swatch(&level).with_size(2, 2);
    let session = Session::open(&component);
    let log = Arc::new(Mutex::new(Vec::new()));
    let surface = recorder(9101, (2, 2));
    session.attach_surface(9101);

    component.subject::<GpuPicture>().next(GpuPicture(Some(Arc::new(Picture(Arc::clone(&log))))));
    wait_for(&log, "gpu", 1);
    let blitted = surface.frames.lock().unwrap().len();
    component.subject::<GpuPicture>().next(GpuPicture(None));
    level.next(9);
    wait_for(&log, "pixels", 1);

    assert_eq!(surface.frames.lock().unwrap().len(), blitted, "nothing was blitted on the CPU behind the target's back");
    session.close();
  }

  #[test]
  fn without_a_target_a_gpu_frame_is_read_back_and_a_surface_without_a_gpu_just_gets_pixels() {
    struct Readable;
    impl GpuFrame for Readable {
      fn size(&self) -> (u32, u32) {
        (2, 2)
      }

      fn as_any(&self) -> &dyn std::any::Any {
        self
      }

      fn read_back(&self) -> Option<Frame> {
        Some(Frame {
          width: 2,
          height: 2,
          pixels: [66u8, 0, 0, 255].repeat(4),
        })
      }
    }

    let level = BehaviorSubject::new(1u8);
    let component = swatch(&level).with_size(2, 2);
    let session = Session::open(&component);
    let surface = recorder(9102, (2, 2));
    session.attach_surface(9102);

    component.subject::<GpuPicture>().next(GpuPicture(Some(Arc::new(Readable))));
    assert_eq!(newest(&surface, |frame| frame.2 == 66), (2, 2, 66));
    session.close();
  }

  #[test]
  fn a_change_after_attaching_is_drawn_on_the_surface() {
    let level = BehaviorSubject::new(10u8);
    let session = Session::open(&swatch(&level).with_size(2, 2));
    let surface = recorder(9002, (2, 2));
    session.attach_surface(9002);
    newest(&surface, |frame| frame.2 == 10);

    level.next(90);
    assert_eq!(newest(&surface, |frame| frame.2 == 90), (2, 2, 90));
    session.close();
  }

  #[test]
  fn a_component_with_no_size_takes_the_surfaces_and_resize_changes_it() {
    let level = BehaviorSubject::new(5u8);
    let session = Session::open(&swatch(&level));
    let surface = recorder(9003, (8, 3));
    session.attach_surface(9003);
    assert_eq!(newest(&surface, |_| true), (8, 3, 5));

    session.resize(4, 2);
    assert_eq!(newest(&surface, |frame| frame.0 == 4), (4, 2, 5));
    session.close();
  }

  #[test]
  fn input_sent_to_the_component_reaches_it_and_close_stops_drawing() {
    let level = BehaviorSubject::new(1u8);
    let component = swatch(&level).with_size(2, 2);
    let session = Session::open(&component);
    let surface = recorder(9004, (2, 2));
    session.attach_surface(9004);
    newest(&surface, |frame| frame.2 == 1);

    let heard = Arc::new(Mutex::new(0));
    let _keys = {
      let heard = Arc::clone(&heard);
      session.component().subject::<KeyEvent>().subscribe(move |_| *heard.lock().unwrap() += 1)
    };
    session.component().next(crate::KeyEvent {
      key: "a".to_string(),
      pressed: true,
    });
    assert_eq!(*heard.lock().unwrap(), 1);

    session.close();
    let drawn = surface.frames.lock().unwrap().len();
    level.next(200);
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(surface.frames.lock().unwrap().len(), drawn, "a closed session draws nothing");
    session.close();
  }
}
