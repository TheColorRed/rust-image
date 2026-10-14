//! Rendering a stream's newest state without a window or a surface.

use std::any::Any;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pub_sub::{BehaviorSubject, Observable, Observer, Subscription};

use crate::{Engine, Pacing, RenderedFrame, Waker, view::ErasedView};

struct Pending<T> {
  value: Option<T>,
  waker: Waker,
}

/// What an offscreen renderer that watches something outside its own input is told about it.
struct Tracking {
  /// A number that is different whenever there is something new to draw.
  version: Box<dyn Fn() -> u64 + Send + Sync>,
  /// Whether more is still on its way, so the next frame should be drawn too.
  is_busy: Box<dyn Fn() -> bool + Send + Sync>,
}

/// The newest input and the version that was last drawn, for a renderer that draws again when its version changes.
struct Seen<T> {
  value: Option<T>,
  version: Option<u64>,
}

struct Renderer<T, F> {
  pending: Arc<Mutex<Pending<T>>>,
  render: Mutex<F>,
  tracking: Option<Tracking>,
  seen: Mutex<Seen<T>>,
}

impl<T, F> ErasedView for Renderer<T, F>
where
  T: Clone + Send + 'static,
  F: FnMut(&T) -> Option<RenderedFrame> + Send + 'static,
{
  fn id(&self) -> &str {
    "offscreen"
  }

  fn is_alive(&self) -> bool {
    true
  }

  fn pacing(&self) -> Pacing {
    // A renderer that watches a version keeps the engine running while there is something to draw or more on the way, and lets
    // it sleep the moment there is not. Anything else is drawn only when its input changes.
    let Some(tracking) = &self.tracking else { return Pacing::OnDemand };
    let drawn = self.seen.lock().unwrap().version;
    if drawn != Some((tracking.version)()) || (tracking.is_busy)() { Pacing::Continuous } else { Pacing::OnDemand }
  }

  fn attach(&self, p_waker: Waker) {
    self.pending.lock().unwrap().waker = p_waker;
  }

  fn tick(&self, _p_delta: Duration) -> Option<RenderedFrame> {
    let fresh = self.pending.lock().unwrap().value.take();
    let Some(tracking) = &self.tracking else {
      return (self.render.lock().unwrap())(&fresh?);
    };
    let mut seen = self.seen.lock().unwrap();
    let mut needed = fresh.is_some();
    if fresh.is_some() {
      seen.value = fresh;
    }
    let version = (tracking.version)();
    needed |= seen.version != Some(version) || (tracking.is_busy)();
    if !needed {
      return None;
    }
    // Nothing can be drawn before the first input. The version is left alone, so it is drawn when the input comes.
    let value = seen.value.clone()?;
    seen.version = Some(version);
    drop(seen);
    (self.render.lock().unwrap())(&value)
  }
}

struct Inner {
  // Stop input before stopping the engine.
  input: Option<Subscription>,
  engine: Mutex<Option<Engine>>,
  frames: BehaviorSubject<Option<RenderedFrame>>,
}

impl Drop for Inner {
  fn drop(&mut self) {
    if let Some(input) = self.input.take() {
      input.unsubscribe();
    }
    if let Some(engine) = self.engine.lock().unwrap().take() {
      engine.close_offscreen();
    }
  }
}

/// An on-demand render stream owned by Vessel, with no window or surface.
///
/// Input changes only enqueue the newest state and wake the engine. Rendering runs on the engine's thread; a change
/// during a render replaces the pending state, not the frame being completed, so continuous input cannot starve the
/// display. Only a completed frame is published, and a late subscriber receives the last completed frame. GPU frames
/// stay on the GPU.
///
/// Derived streams keep the renderer alive automatically. Dropping the last stream stops input and future renders;
/// an in-flight render releases its resources without blocking the caller.
#[derive(Clone)]
pub struct Offscreen {
  inner: Arc<Inner>,
}

impl Offscreen {
  /// Renders the newest value of `p_source` using `p_render`.
  ///
  /// `None` means there is no frame ready to publish (for example, a view has no size yet). The renderer must report
  /// errors itself and must return a stable frame: subsequent renders must not modify a published GPU texture.
  pub fn new<T: Clone + Send + Sync + 'static>(
    p_source: &impl Observable<T>, p_render: impl FnMut(&T) -> Option<RenderedFrame> + Send + 'static,
  ) -> Self {
    Self::build(p_source, None, p_render)
  }

  /// Like [`new`](Self::new), and it also draws again when something outside its input has changed, with no input to say so.
  ///
  /// - `p_version`: A number that is different whenever there is something new to draw, such as the version of a picture that
  ///   is edited somewhere else. It is cheap, since it is asked every frame while the renderer is busy. The renderer draws the
  ///   newest input again whenever it changes.
  /// - `p_is_busy`: Whether more is on its way (tiles still being put on the GPU, say), so the next frame is drawn too.
  ///
  /// The engine runs frame after frame while there is something to draw or more on the way, and sleeps when there is not, so
  /// an idle renderer costs nothing. It is woken by input, and by [`wake`](Self::wake) for a change it cannot see.
  pub fn tracking<T: Clone + Send + Sync + 'static>(
    p_source: &impl Observable<T>, p_version: impl Fn() -> u64 + Send + Sync + 'static,
    p_is_busy: impl Fn() -> bool + Send + Sync + 'static, p_render: impl FnMut(&T) -> Option<RenderedFrame> + Send + 'static,
  ) -> Self {
    let tracking = Tracking { version: Box::new(p_version), is_busy: Box::new(p_is_busy) };
    Self::build(p_source, Some(tracking), p_render)
  }

  /// Wakes the renderer's engine, so a renderer made with [`tracking`](Self::tracking) looks at its version again at once.
  pub fn wake(&self) {
    if let Some(engine) = self.inner.engine.lock().unwrap().as_ref() {
      engine.wake();
    }
  }

  fn build<T: Clone + Send + Sync + 'static>(
    p_source: &impl Observable<T>, p_tracking: Option<Tracking>,
    p_render: impl FnMut(&T) -> Option<RenderedFrame> + Send + 'static,
  ) -> Self {
    let pending = Arc::new(Mutex::new(Pending {
      value: None,
      waker: Waker::default(),
    }));
    let input = p_source.subscribe({
      let pending = Arc::clone(&pending);
      move |value| {
        let waker = {
          let mut pending = pending.lock().unwrap();
          pending.value = Some(value.clone());
          pending.waker.clone()
        };
        waker.wake();
      }
    });
    let frames = BehaviorSubject::new(None);
    let engine = Engine::with_gpu_sink(
      {
        let frames = frames.clone();
        move |_, frame| frames.next(Some(RenderedFrame::Pixels(Arc::new(frame.clone()))))
      },
      {
        let frames = frames.clone();
        move |_, frame| frames.next(Some(RenderedFrame::Gpu(Arc::clone(frame))))
      },
    );
    engine.add_erased(Box::new(Renderer {
      pending,
      render: Mutex::new(p_render),
      tracking: p_tracking,
      seen: Mutex::new(Seen { value: None, version: None }),
    }));
    Self {
      inner: Arc::new(Inner {
        input: Some(input),
        engine: Mutex::new(Some(engine)),
        frames,
      }),
    }
  }
}

impl Observable<RenderedFrame> for Offscreen {
  fn upstream(&self) -> Box<dyn Any + Send + Sync> {
    Box::new(self.clone())
  }

  fn subscribe(&self, p_listener: impl Fn(&RenderedFrame) + Send + Sync + 'static) -> Subscription {
    let subscription = self.inner.frames.subscribe(move |frame| {
      if let Some(frame) = frame {
        p_listener(frame);
      }
    });
    let inner = Arc::clone(&self.inner);
    Subscription::new(move || {
      subscription.unsubscribe();
      drop(inner);
    })
  }
}

#[cfg(test)]
mod tests {
  use std::any::Any;
  use std::sync::mpsc;
  use std::time::Duration;

  use super::*;
  use crate::{Frame, GpuFrame};
  use pub_sub::Subject;

  fn frame(p_red: u8) -> Option<RenderedFrame> {
    Some(RenderedFrame::Pixels(Arc::new(Frame {
      width: 1,
      height: 1,
      pixels: Arc::new(vec![p_red, 0, 0, 255]),
    })))
  }

  #[test]
  fn renders_without_a_surface_and_derived_streams_keep_it_alive() {
    let state = BehaviorSubject::new(12u8);
    let caller = std::thread::current().id();
    let output = Offscreen::new(&state, move |value| {
      assert_ne!(std::thread::current().id(), caller);
      frame(*value)
    })
    .map(|frame| match frame {
      RenderedFrame::Pixels(frame) => frame.pixels[0],
      RenderedFrame::Gpu(_) => panic!("expected pixels"),
    });
    let (send, receive) = mpsc::channel();
    let subscription = output.subscribe(move |red| send.send(*red).unwrap());
    // A derived Subject does not replay values sent before subscribing.
    state.next(24);
    loop {
      if receive.recv_timeout(Duration::from_secs(2)).unwrap() == 24 {
        break;
      }
    }
    drop((subscription, output));
  }

  #[test]
  fn continuous_input_does_not_starve_completed_frames_and_pending_changes_are_coalesced() {
    let state = Subject::new();
    let (started, entered) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let output = Offscreen::new(&state, move |value| {
      if *value == 1 {
        started.send(()).unwrap();
        resume.recv_timeout(Duration::from_secs(2)).unwrap();
      }
      frame(*value)
    });
    let (send, receive) = mpsc::channel();
    let _subscription = output.subscribe(move |frame| {
      let RenderedFrame::Pixels(frame) = frame else { panic!("expected pixels") };
      send.send(frame.pixels[0]).unwrap();
    });
    state.next(1);
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    state.next(2);
    state.next(3);
    release.send(()).unwrap();
    assert_eq!(receive.recv_timeout(Duration::from_secs(2)).unwrap(), 1);
    assert_eq!(receive.recv_timeout(Duration::from_secs(2)).unwrap(), 3);
    assert!(receive.recv_timeout(Duration::from_millis(50)).is_err());
    let (send, receive) = mpsc::channel();
    let _late = output.subscribe(move |frame| {
      let RenderedFrame::Pixels(frame) = frame else { panic!("expected pixels") };
      send.send(frame.pixels[0]).unwrap();
    });
    assert_eq!(receive.recv_timeout(Duration::from_secs(2)).unwrap(), 3);
  }

  #[test]
  fn a_subscription_keeps_the_renderer_alive_and_drop_stops_persistent_input() {
    let state = Subject::persistent();
    let (rendered, receive_rendered) = mpsc::channel();
    let (send, receive) = mpsc::channel();
    let subscription = Offscreen::new(&state, move |value| {
      rendered.send(*value).unwrap();
      frame(*value)
    })
    .subscribe(move |_| send.send(()).unwrap());
    state.next(17);
    assert_eq!(receive_rendered.recv_timeout(Duration::from_secs(2)).unwrap(), 17);
    receive.recv_timeout(Duration::from_secs(2)).unwrap();
    drop(subscription);
    state.next(18);
    assert!(receive_rendered.recv_timeout(Duration::from_millis(50)).is_err());
  }

  #[test]
  fn gpu_frames_are_published_without_readback() {
    struct Texture;
    impl GpuFrame for Texture {
      fn size(&self) -> (u32, u32) {
        (1, 1)
      }

      fn as_any(&self) -> &dyn Any {
        self
      }

      fn read_back(&self) -> Option<Frame> {
        panic!("offscreen rendering must not read back a GPU frame");
      }
    }
    let state = Subject::new();
    let output = Offscreen::new(&state, |_| Some(RenderedFrame::Gpu(Arc::new(Texture))));
    let (send, receive) = mpsc::channel();
    let _subscription = output.subscribe(move |frame| send.send(matches!(frame, RenderedFrame::Gpu(_))).unwrap());
    state.next(());
    assert!(receive.recv_timeout(Duration::from_secs(2)).unwrap());
  }

  #[test]
  fn no_frame_keeps_the_last_completed_frame() {
    let state = Subject::new();
    let output = Offscreen::new(&state, |value| if *value == 0 { None } else { frame(*value) });
    let (send, receive) = mpsc::channel();
    let _subscription = output.subscribe(move |_| send.send(()).unwrap());
    state.next(9);
    receive.recv_timeout(Duration::from_secs(2)).unwrap();
    state.next(0);
    assert!(receive.recv_timeout(Duration::from_millis(50)).is_err());
    let (send, receive) = mpsc::channel();
    let _late = output.subscribe(move |frame| {
      let RenderedFrame::Pixels(frame) = frame else { panic!("expected pixels") };
      send.send(frame.pixels[0]).unwrap();
    });
    assert_eq!(receive.recv_timeout(Duration::from_secs(2)).unwrap(), 9);
  }

  #[test]
  fn a_listener_can_stop_its_renderer_on_the_engine_thread() {
    let state = Subject::new();
    let subscription = Arc::new(Mutex::new(None::<Subscription>));
    let (send, receive) = mpsc::channel();
    *subscription.lock().unwrap() = Some(Offscreen::new(&state, |value| frame(*value)).subscribe({
      let subscription = Arc::clone(&subscription);
      move |_| {
        subscription.lock().unwrap().take().unwrap().unsubscribe();
        send.send(()).unwrap();
      }
    }));
    state.next(1);
    receive.recv_timeout(Duration::from_secs(2)).unwrap();
    state.next(2);
    assert!(receive.recv_timeout(Duration::from_millis(50)).is_err());
  }

  #[test]
  fn retiring_a_busy_offscreen_renderer_does_not_block_its_caller() {
    let state = Subject::new();
    let (started, entered) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let output = Offscreen::new(&state, move |value| {
      started.send(()).unwrap();
      resume.recv_timeout(Duration::from_secs(2)).unwrap();
      frame(*value)
    });
    state.next(1);
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    let (stopped, dropped) = mpsc::channel();
    let dropping = std::thread::spawn(move || {
      drop(output);
      stopped.send(()).unwrap();
    });
    let result = dropped.recv_timeout(Duration::from_millis(100));
    release.send(()).unwrap();
    dropping.join().unwrap();
    result.expect("offscreen teardown blocked on the in-flight render");
  }
  /// The frames a renderer has published so far.
  fn counted(p_output: &Offscreen) -> (Arc<std::sync::atomic::AtomicUsize>, Subscription) {
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let subscription = p_output.subscribe({
      let count = Arc::clone(&count);
      move |_| {
        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
      }
    });
    (count, subscription)
  }

  fn frames(p_count: &std::sync::atomic::AtomicUsize) -> usize {
    p_count.load(std::sync::atomic::Ordering::SeqCst)
  }

  /// Waits until `p_count` reaches `p_frames`, or two seconds pass.
  fn wait_for(p_count: &std::sync::atomic::AtomicUsize, p_frames: usize) {
    let started = std::time::Instant::now();
    while frames(p_count) < p_frames && started.elapsed() < Duration::from_secs(2) {
      std::thread::sleep(Duration::from_millis(2));
    }
  }

  #[test]
  fn a_tracking_renderer_draws_when_its_version_changes_with_no_input_and_sleeps_when_it_does_not() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let state = BehaviorSubject::new(5u8);
    let version = Arc::new(AtomicU64::new(1));
    let output = Offscreen::tracking(&state, { let version = Arc::clone(&version); move || version.load(Ordering::SeqCst) }, || false, |value| frame(*value));
    let (count, _subscription) = counted(&output);
    // The first input is drawn, and then nothing is, however long it waits.
    state.next(5);
    wait_for(&count, 1);
    std::thread::sleep(Duration::from_millis(60));
    assert_eq!(frames(&count), 1, "an idle renderer draws nothing");
    // A change somewhere else is drawn once, as soon as the engine is woken, from the same input.
    version.store(2, Ordering::SeqCst);
    output.wake();
    wait_for(&count, 2);
    std::thread::sleep(Duration::from_millis(60));
    assert_eq!(frames(&count), 2, "one draw for one change, and then it sleeps again");
  }

  #[test]
  fn a_tracking_renderer_draws_every_frame_while_it_is_busy_and_stops_when_it_is_not() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let state = BehaviorSubject::new(1u8);
    let left = Arc::new(AtomicUsize::new(3));
    let output = Offscreen::tracking(
      &state,
      || 1,
      { let left = Arc::clone(&left); move || left.load(Ordering::SeqCst) > 0 },
      { let left = Arc::clone(&left); move |value| { left.fetch_sub(1, Ordering::SeqCst); frame(*value) } },
    );
    let (count, _subscription) = counted(&output);
    state.next(1);
    wait_for(&count, 3);
    std::thread::sleep(Duration::from_millis(60));
    assert_eq!(frames(&count), 3, "one frame for each of the three it was busy for, then it stops");
  }

  #[test]
  fn a_tracking_renderer_with_no_input_yet_draws_nothing_and_draws_when_the_input_comes() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let state = Subject::new();
    let version = Arc::new(AtomicU64::new(1));
    let output = Offscreen::tracking(&state, { let version = Arc::clone(&version); move || version.load(Ordering::SeqCst) }, || false, |value: &u8| frame(*value));
    let (count, _subscription) = counted(&output);
    version.store(2, Ordering::SeqCst);
    output.wake();
    std::thread::sleep(Duration::from_millis(60));
    assert_eq!(frames(&count), 0, "there is nothing to draw without an input");
    state.next(9);
    wait_for(&count, 1);
    assert_eq!(frames(&count), 1);
  }
}
