//! What a view shows. A [`MediaSource`] can be a canvas, a video, a camera, a game, a desktop application, or
//! anything else that can answer a few questions. The engine asks them and decides for itself when to draw.

use std::{
  sync::{Arc, Condvar, Mutex},
  time::Duration,
};

use crate::{Frame, GpuFrame};

/// How often a source needs frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pacing {
  /// Only when the source says it has changed, such as a canvas after an edit.
  OnDemand,
  /// On every frame, such as video, a camera or a game.
  Continuous,
}

/// Anything a view can show.
pub trait MediaSource: Send + 'static {
  /// How often this source needs frames.
  fn pacing(&self) -> Pacing;

  /// Whether the source has something new to show since it was last rendered. A canvas reports its dirty flag, a video
  /// whether the next frame is ready. Sources that always have something new return `true`.
  fn has_changed(&self) -> bool;

  /// Draws the source's current state and returns the frame. The engine calls this when it decides to draw, not the
  /// source.
  ///
  /// - `p_delta`: Time since the engine last drew this view, for sources that move on their own, such as a game.
  fn render(&mut self, p_delta: Duration) -> Frame;

  /// A finished frame that lives on the GPU, if the source has one instead of pixels. The engine asks this before
  /// [`render`](Self::render) and uses it when there is one, so a source that renders on the GPU never has its pictures
  /// copied to the CPU. A source that only makes pixels leaves this alone.
  fn render_gpu(&mut self, _p_delta: Duration) -> Option<Arc<dyn GpuFrame>> {
    None
  }

  /// Hands the source a [`Waker`] when it is added to an engine. Sources that make frames on their own thread, such as a
  /// camera or a screen capture, keep it and call [`Waker::wake`] when a frame is ready, so an idle engine looks again.
  fn set_waker(&mut self, _waker: Waker) {}
}

/// Wakes an idle engine so it looks at its views again. Cheap to clone and safe to call from any thread.
#[derive(Clone, Default)]
pub struct Waker {
  signal: Arc<Signal>,
}

#[derive(Default)]
struct Signal {
  woken: Mutex<bool>,
  condvar: Condvar,
}

impl Waker {
  /// Wakes the engine, or makes its next sleep end at once if it is awake.
  pub fn wake(&self) {
    *self.signal.woken.lock().unwrap() = true;
    self.signal.condvar.notify_one();
  }

  /// Sleeps until woken, or until `p_timeout` passes. A wake that came earlier counts.
  pub(crate) fn wait(&self, p_timeout: Option<Duration>) {
    let mut woken = self.signal.woken.lock().unwrap();
    if !*woken {
      woken = match p_timeout {
        Some(timeout) => self.signal.condvar.wait_timeout(woken, timeout).unwrap().0,
        None => self.signal.condvar.wait(woken).unwrap(),
      };
    }
    *woken = false;
  }
}
