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

struct Renderer<T, F> {
  pending: Arc<Mutex<Pending<T>>>,
  render: Mutex<F>,
}

impl<T, F> ErasedView for Renderer<T, F>
where
  T: Send + 'static,
  F: FnMut(&T) -> Option<RenderedFrame> + Send + 'static,
{
  fn id(&self) -> &str {
    "offscreen"
  }

  fn is_alive(&self) -> bool {
    true
  }

  fn pacing(&self) -> Pacing {
    Pacing::OnDemand
  }

  fn attach(&self, p_waker: Waker) {
    self.pending.lock().unwrap().waker = p_waker;
  }

  fn tick(&self, _p_delta: Duration) -> Option<RenderedFrame> {
    let value = self.pending.lock().unwrap().value.take()?;
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
      pixels: vec![p_red, 0, 0, 255],
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
}
