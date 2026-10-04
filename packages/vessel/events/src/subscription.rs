/// A listener's connection to a [`Subject`](crate::Subject) or [`BehaviorSubject`](crate::BehaviorSubject).
///
/// What dropping it does depends on the stream. On an ordinary stream, dropping it stops the listening, so hold it for as
/// long as you want to listen. On a [`Subject::persistent`](crate::Subject::persistent) stream the listener stays until the
/// stream is dropped, and the subscription is only for stopping one early with [`unsubscribe`](Self::unsubscribe).
pub struct Subscription {
  unsubscribe: Option<Box<dyn FnOnce() + Send + Sync>>,
  /// Whether dropping the subscription stops the listener.
  stops_on_drop: bool,
}

impl Subscription {
  /// Creates a subscription that runs `p_unsubscribe` once when it is dropped. For types that offer their own
  /// subscriptions, the way a [`Subject`](crate::Subject) does.
  pub fn new(p_unsubscribe: impl FnOnce() + Send + Sync + 'static) -> Self {
    Self {
      unsubscribe: Some(Box::new(p_unsubscribe)),
      stops_on_drop: true,
    }
  }

  /// Creates a subscription that does nothing when it is dropped: the listener stays until the stream it listens to goes
  /// away. Only [`unsubscribe`](Self::unsubscribe) runs `p_unsubscribe`.
  pub fn detached(p_unsubscribe: impl FnOnce() + Send + Sync + 'static) -> Self {
    Self {
      unsubscribe: Some(Box::new(p_unsubscribe)),
      stops_on_drop: false,
    }
  }

  /// Stops the listening now, whatever dropping would have done.
  pub fn unsubscribe(mut self) {
    if let Some(unsubscribe) = self.unsubscribe.take() {
      unsubscribe();
    }
  }
}

impl Drop for Subscription {
  fn drop(&mut self) {
    if self.stops_on_drop {
      if let Some(unsubscribe) = self.unsubscribe.take() {
        unsubscribe();
      }
    }
  }
}
