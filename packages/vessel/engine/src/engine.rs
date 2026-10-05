//! The loop that runs views. It owns the schedule: views change state when they get messages, and the engine
//! decides when to draw them.

use std::{
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  thread::JoinHandle,
  time::{Duration, Instant},
};

use crate::{
  Frame, GpuFrame, Pacing, Waker,
  gpu::RenderedFrame as Rendered,
  view::{ErasedView, View},
};

/// Receives the GPU frames a view finishes, with the id of the view.
type GpuSink = Box<dyn Fn(&str, &Arc<dyn GpuFrame>) + Send>;

/// How long the loop waits between frames while a continuous view is running (about 60 frames a second).
const FRAME: Duration = Duration::from_millis(16);

struct Shared {
  views: Mutex<Vec<Box<dyn ErasedView>>>,
  waker: Waker,
  running: AtomicBool,
}

/// Runs views on its own thread until it is closed or dropped. With only on-demand views it sleeps until a message
/// or a [`Waker`] wakes it; with a continuous view it draws every frame.
pub struct Engine {
  shared: Arc<Shared>,
  thread: Option<JoinHandle<()>>,
}

/// Frames drawn on a surface registered with the `surface` crate.
#[cfg(any(feature = "android-surface", feature = "desktop-window"))]
impl Engine {
  /// Starts the loop, drawing every finished frame on the surface registered under `p_surface_id`. A surface that is not
  /// registered yet, or is gone, simply misses the frame.
  pub fn for_surface(p_surface_id: i32) -> Self {
    Self::new(move |_view, frame| {
      surface::draw_rgba(p_surface_id, frame.width, frame.height, frame.pixels.as_slice());
    })
  }
}

impl Engine {
  /// Starts the loop.
  ///
  /// - `p_sink`: Receives each finished frame with the id of its view. Runs on the engine's thread.
  pub fn new(p_sink: impl Fn(&str, &Frame) + Send + 'static) -> Self {
    Self::start(p_sink, None)
  }

  /// Starts the loop with a second sink for GPU frames, so a source that renders on the GPU is drawn without its pictures
  /// being copied to the CPU. Without one, a GPU frame is read back into pixels and goes to the first sink.
  ///
  /// - `p_sink`: Receives each frame made of pixels.
  /// - `p_gpu_sink`: Receives each frame that lives on the GPU. Runs on the engine's thread.
  pub fn with_gpu_sink(
    p_sink: impl Fn(&str, &Frame) + Send + 'static, p_gpu_sink: impl Fn(&str, &Arc<dyn GpuFrame>) + Send + 'static,
  ) -> Self {
    Self::start(p_sink, Some(Box::new(p_gpu_sink)))
  }

  fn start(p_sink: impl Fn(&str, &Frame) + Send + 'static, p_gpu_sink: Option<GpuSink>) -> Self {
    let shared = Arc::new(Shared {
      views: Mutex::new(Vec::new()),
      waker: Waker::default(),
      running: AtomicBool::new(true),
    });
    let thread = {
      let shared = Arc::clone(&shared);
      std::thread::spawn(move || run(&shared, p_sink, p_gpu_sink))
    };
    Self {
      shared,
      thread: Some(thread),
    }
  }

  /// Starts running `p_view`. It stays until its subscription is dropped or the engine closes.
  pub fn add<M: Send + 'static>(&self, p_view: &View<M>) {
    self.add_erased(Box::new(p_view.clone()));
  }

  pub(crate) fn add_erased(&self, p_view: Box<dyn ErasedView>) {
    p_view.attach(self.shared.waker.clone());
    self.shared.views.lock().unwrap().push(p_view);
    self.shared.waker.wake();
  }

  /// Stops the loop and waits for it to finish. Dropping the engine does the same.
  pub fn close(mut self) {
    self.stop();
  }

  pub(crate) fn close_offscreen(mut self) {
    self.shared.running.store(false, Ordering::SeqCst);
    self.shared.waker.wake();
    // There is no surface to retire. Let the in-flight job release its resources without blocking the UI's teardown.
    self.thread.take();
  }

  fn stop(&mut self) {
    self.shared.running.store(false, Ordering::SeqCst);
    self.shared.waker.wake();
    let Some(thread) = self.thread.take() else { return };
    // An offscreen subscriber can release its renderer from the engine's own output callback.
    if thread.thread().id() == std::thread::current().id() {
      return;
    }
    let _ = thread.join();
    // If the loop panicked it left the lock poisoned. Stopping must still work: it often runs while something else is
    // already being torn down, where a second panic would abort the whole process.
    self.shared.views.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clear();
  }
}

impl Drop for Engine {
  fn drop(&mut self) {
    self.stop();
  }
}

fn run(p_shared: &Shared, p_sink: impl Fn(&str, &Frame), p_gpu_sink: Option<GpuSink>) {
  let mut last = Instant::now();
  while p_shared.running.load(Ordering::SeqCst) {
    let now = Instant::now();
    let delta = now - last;
    last = now;

    let mut continuous = false;
    {
      let mut views = p_shared.views.lock().unwrap();
      views.retain(|view| view.is_alive());
      for view in views.iter() {
        continuous |= view.pacing() == Pacing::Continuous;
        match view.tick(delta) {
          Some(Rendered::Pixels(frame)) => p_sink(view.id(), &frame),
          Some(Rendered::Gpu(frame)) => match &p_gpu_sink {
            Some(gpu_sink) => gpu_sink(view.id(), &frame),
            // Nowhere to draw it from the GPU: copy it to the CPU, which every sink can show.
            None => {
              if let Some(pixels) = frame.read_back() {
                p_sink(view.id(), &pixels);
              }
            }
          },
          None => {}
        }
      }
    }
    p_shared.waker.wait(continuous.then_some(FRAME));
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    mpsc::{self, Receiver},
  };

  use super::*;
  use crate::MediaSource;

  /// A one pixel source whose colour is `value`. Draws only after `changed` is set, unless continuous.
  struct Test {
    value: Arc<AtomicU8>,
    changed: Arc<AtomicBool>,
    pacing: Pacing,
    waker: Arc<Mutex<Option<Waker>>>,
  }

  impl MediaSource for Test {
    fn pacing(&self) -> Pacing {
      self.pacing
    }

    fn has_changed(&self) -> bool {
      self.changed.load(Ordering::SeqCst)
    }

    fn render(&mut self, _delta: Duration) -> Frame {
      self.changed.store(false, Ordering::SeqCst);
      Frame {
        width: 1,
        height: 1,
        pixels: Arc::new(vec![self.value.load(Ordering::SeqCst), 0, 0, 255]),
      }
    }

    fn set_waker(&mut self, p_waker: Waker) {
      *self.waker.lock().unwrap() = Some(p_waker);
    }
  }

  /// A source whose pictures live on the GPU: the frame is a number, and reading it back gives one pixel of it.
  struct Gpu(u8);

  impl GpuFrame for Gpu {
    fn size(&self) -> (u32, u32) {
      (1, 1)
    }

    fn as_any(&self) -> &dyn std::any::Any {
      self
    }

    fn read_back(&self) -> Option<Frame> {
      Some(Frame {
        width: 1,
        height: 1,
        pixels: Arc::new(vec![self.0, 0, 0, 255]),
      })
    }
  }

  struct GpuSource(Arc<AtomicBool>);

  impl MediaSource for GpuSource {
    fn pacing(&self) -> Pacing {
      Pacing::OnDemand
    }

    fn has_changed(&self) -> bool {
      self.0.swap(false, Ordering::SeqCst)
    }

    fn render(&mut self, _delta: Duration) -> Frame {
      panic!("a source with a GPU frame must not be asked for pixels");
    }

    fn render_gpu(&mut self, _delta: Duration) -> Option<Arc<dyn GpuFrame>> {
      Some(Arc::new(Gpu(77)))
    }
  }

  #[test]
  fn a_gpu_frame_goes_to_the_gpu_sink_and_is_never_read_back() {
    let (sender, frames) = mpsc::channel();
    let engine = Engine::with_gpu_sink(
      |_id, _frame| panic!("pixels were drawn although the frame is on the GPU"),
      move |_id, frame| {
        let _ = sender.send(frame.size());
      },
    );
    engine.add(&View::<()>::new("gpu", GpuSource(Arc::new(AtomicBool::new(true)))));
    assert_eq!(frames.recv_timeout(Duration::from_secs(2)), Ok((1, 1)));
  }

  #[test]
  fn without_a_gpu_sink_a_gpu_frame_is_read_back_into_the_pixel_sink() {
    let (sender, frames) = mpsc::channel();
    let engine = Engine::new(move |_id, frame| {
      let _ = sender.send(frame.pixels[0]);
    });
    engine.add(&View::<()>::new("gpu", GpuSource(Arc::new(AtomicBool::new(true)))));
    assert_eq!(frames.recv_timeout(Duration::from_secs(2)), Ok(77));
  }

  struct Rig {
    value: Arc<AtomicU8>,
    changed: Arc<AtomicBool>,
    waker: Arc<Mutex<Option<Waker>>>,
    view: View<u8>,
    frames: Receiver<u8>,
    engine: Engine,
  }

  fn rig(p_pacing: Pacing) -> Rig {
    let value = Arc::new(AtomicU8::new(0));
    let changed = Arc::new(AtomicBool::new(true));
    let waker = Arc::new(Mutex::new(None));
    let media = Test {
      value: Arc::clone(&value),
      changed: Arc::clone(&changed),
      pacing: p_pacing,
      waker: Arc::clone(&waker),
    };
    let (sender, frames) = mpsc::channel();
    let engine = Engine::new(move |_id, frame| {
      let _ = sender.send(frame.pixels[0]);
    });
    let view = View::new("test", media);
    engine.add(&view);
    Rig {
      value,
      changed,
      waker,
      view,
      frames,
      engine,
    }
  }

  const WAIT: Duration = Duration::from_secs(2);
  const QUIET: Duration = Duration::from_millis(150);

  #[test]
  fn an_on_demand_view_draws_once_then_sleeps() {
    let rig = rig(Pacing::OnDemand);
    assert_eq!(rig.frames.recv_timeout(WAIT), Ok(0));
    assert!(rig.frames.recv_timeout(QUIET).is_err());
  }

  #[test]
  fn many_messages_draw_the_newest_state() {
    let rig = rig(Pacing::OnDemand);
    rig.frames.recv_timeout(WAIT).unwrap();
    let (value, changed) = (Arc::clone(&rig.value), Arc::clone(&rig.changed));
    let _subscription = rig.view.subscribe(move |message| {
      value.store(*message, Ordering::SeqCst);
      changed.store(true, Ordering::SeqCst);
    });
    (1..=100).for_each(|message| rig.view.send(message));

    let mut drawn = Vec::new();
    while drawn.last() != Some(&100) {
      drawn.push(rig.frames.recv_timeout(WAIT).expect("the newest state should be drawn"));
    }
    assert!(drawn.len() <= 100);
    assert!(drawn.windows(2).all(|pair| pair[0] < pair[1]), "frames went backwards: {drawn:?}");
  }

  #[test]
  fn a_continuous_view_draws_every_frame() {
    let rig = rig(Pacing::Continuous);
    for _ in 0..4 {
      rig.frames.recv_timeout(WAIT).unwrap();
    }
  }

  #[test]
  fn a_source_can_wake_an_idle_engine() {
    let rig = rig(Pacing::OnDemand);
    rig.frames.recv_timeout(WAIT).unwrap();
    rig.value.store(9, Ordering::SeqCst);
    rig.changed.store(true, Ordering::SeqCst);
    let waker = rig.waker.lock().unwrap().clone().expect("the engine gives the source a waker");
    std::thread::spawn(move || waker.wake());
    assert_eq!(rig.frames.recv_timeout(WAIT), Ok(9));
  }

  #[test]
  fn dropping_the_subscription_removes_the_view() {
    let rig = rig(Pacing::OnDemand);
    let subscription = rig.view.subscribe(|_| {});
    rig.frames.recv_timeout(WAIT).unwrap();
    drop(subscription);
    rig.changed.store(true, Ordering::SeqCst);
    rig.view.send(1);
    assert!(rig.frames.recv_timeout(QUIET).is_err());
  }

  #[test]
  fn closing_stops_the_loop() {
    let rig = rig(Pacing::Continuous);
    rig.frames.recv_timeout(WAIT).unwrap();
    rig.engine.close();
    while rig.frames.try_recv().is_ok() {}
    assert!(rig.frames.recv_timeout(QUIET).is_err());
  }
}
