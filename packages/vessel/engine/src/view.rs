//! One view: a media source plus a mini app's messages and handler.

use std::{
  collections::VecDeque,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use pub_sub::Subscription;

use crate::{MediaSource, Pacing, Waker, gpu::RenderedFrame as Rendered};

struct Inner<M> {
  id: String,
  media: Mutex<Box<dyn MediaSource>>,
  inbox: Mutex<VecDeque<M>>,
  handler: Mutex<Option<Box<dyn FnMut(&M) + Send>>>,
  waker: Mutex<Option<Waker>>,
  alive: AtomicBool,
}

/// A view, and the stream of messages sent to it. A mini app `subscribe`s to it and changes its own state in the
/// handler; it never says when to draw, the engine decides that. Cloning gives another handle to the same view.
///
/// ```ignore
/// let view = View::new("main_image", canvas);
/// let subscription = view.subscribe(move |message| { /* change the canvas */ });
/// engine.add(&view);
/// view.send(Message::Brightness(0.3));
/// ```
pub struct View<M> {
  inner: Arc<Inner<M>>,
}

impl<M> Clone for View<M> {
  fn clone(&self) -> Self {
    Self {
      inner: Arc::clone(&self.inner),
    }
  }
}

impl<M: Send + 'static> View<M> {
  /// Creates a view that shows `p_media`. It draws nothing until it is added to an [`Engine`](crate::Engine).
  pub fn new(p_id: impl Into<String>, p_media: impl MediaSource) -> Self {
    Self {
      inner: Arc::new(Inner {
        id: p_id.into(),
        media: Mutex::new(Box::new(p_media)),
        inbox: Mutex::new(VecDeque::new()),
        handler: Mutex::new(None),
        waker: Mutex::new(None),
        alive: AtomicBool::new(true),
      }),
    }
  }

  /// The id this view was created with.
  pub fn id(&self) -> &str {
    &self.inner.id
  }

  /// Queues `p_message` for the handler and wakes the engine. Ignored once the view's subscription is gone.
  pub fn send(&self, p_message: M) {
    if !self.inner.alive.load(Ordering::SeqCst) {
      return;
    }
    self.inner.inbox.lock().unwrap().push_back(p_message);
    if let Some(waker) = self.inner.waker.lock().unwrap().as_ref() {
      waker.wake();
    }
  }

  /// Calls `p_handler` with each message sent to this view, one at a time and in order, so it can keep and change its
  /// own state. Replaces an earlier handler. Dropping the returned [`Subscription`] stops the handler and removes the
  /// view from its engine, so the subscription is the mini app's life.
  ///
  /// - `p_handler`: Runs on the engine's thread, and should change state only. Drawing is the engine's decision.
  pub fn subscribe(&self, p_handler: impl FnMut(&M) + Send + 'static) -> Subscription {
    *self.inner.handler.lock().unwrap() = Some(Box::new(p_handler));
    let inner = Arc::clone(&self.inner);
    Subscription::new(move || {
      inner.alive.store(false, Ordering::SeqCst);
      *inner.handler.lock().unwrap() = None;
      inner.inbox.lock().unwrap().clear();
    })
  }
}

/// What the engine needs from a view, without knowing its message type.
pub(crate) trait ErasedView: Send {
  fn id(&self) -> &str;
  fn is_alive(&self) -> bool;
  fn pacing(&self) -> Pacing;
  fn attach(&self, p_waker: Waker);
  /// Runs the queued messages through the handler, then returns a frame if the media needs one.
  fn tick(&self, p_delta: Duration) -> Option<Rendered>;
}

impl<M: Send + 'static> ErasedView for View<M> {
  fn id(&self) -> &str {
    &self.inner.id
  }

  fn is_alive(&self) -> bool {
    self.inner.alive.load(Ordering::SeqCst)
  }

  fn pacing(&self) -> Pacing {
    self.inner.media.lock().unwrap().pacing()
  }

  fn attach(&self, p_waker: Waker) {
    self.inner.media.lock().unwrap().set_waker(p_waker.clone());
    *self.inner.waker.lock().unwrap() = Some(p_waker);
  }

  fn tick(&self, p_delta: Duration) -> Option<Rendered> {
    // Taken out first so a handler that sends a message does not wait on the inbox.
    let messages: Vec<M> = self.inner.inbox.lock().unwrap().drain(..).collect();
    if !messages.is_empty()
      && let Some(handler) = self.inner.handler.lock().unwrap().as_mut()
    {
      messages.iter().for_each(|message| handler(message));
    }
    let mut media = self.inner.media.lock().unwrap();
    if media.pacing() != Pacing::Continuous && !media.has_changed() {
      return None;
    }
    Some(match media.render_gpu(p_delta) {
      Some(frame) => Rendered::Gpu(frame),
      None => Rendered::Pixels(Arc::new(media.render(p_delta))),
    })
  }
}
