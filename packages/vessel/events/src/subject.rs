use std::any::Any;
use std::sync::{Arc, Mutex, Weak};

use crate::{Observable, Observer, Subscription};

type Listener<T> = Arc<dyn Fn(&T) + Send + Sync>;

struct Inner<T> {
  next_id: u64,
  /// Replaced, never changed in place, so `next` can take a snapshot with one `Arc` clone and call the listeners
  /// without holding the lock.
  listeners: Arc<Vec<(u64, Listener<T>)>>,
  /// Subscriptions that feed this stream from another one (see [`Observable::filter`]), kept for as long as the stream is.
  sources: Vec<Subscription>,
  /// The streams an operator listens to, held so a chain stays connected for as long as its last link is alive.
  upstream: Vec<Box<dyn Any + Send + Sync>>,
  /// Whether listeners stay until the stream is dropped, whatever happens to the subscriptions they were given.
  persistent: bool,
}

/// A stream of events of type `T`, usually an `enum`. A listener hears every event sent after it subscribed, and nothing
/// from before.
///
/// ```ignore
/// enum LayerEvent {
///   Changed(u32),
/// }
///
/// let subject = Subject::new();
/// let subscription = subject.subscribe(|event| { /* react to the event */ });
/// subject.next(LayerEvent::Changed(1));
/// drop(subscription); // no more events
/// ```
pub struct Subject<T> {
  inner: Arc<Mutex<Inner<T>>>,
}

impl<T> Clone for Subject<T> {
  fn clone(&self) -> Self {
    Self {
      inner: Arc::clone(&self.inner),
    }
  }
}

impl<T: 'static> Default for Subject<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: 'static> Subject<T> {
  /// Creates a stream with no listeners. A listener lasts as long as the [`Subscription`] it was given.
  pub fn new() -> Self {
    Self::with_persistence(false)
  }

  /// Creates a stream whose listeners last as long as the stream does. `subscribe` is then all there is to do: the
  /// returned [`Subscription`] can be dropped, and is only needed to stop one listener early. This is what a stream that
  /// belongs to an object wants, such as a component's events, which go away with it.
  pub fn persistent() -> Self {
    Self::with_persistence(true)
  }

  fn with_persistence(p_persistent: bool) -> Self {
    Self {
      inner: Arc::new(Mutex::new(Inner {
        next_id: 0,
        listeners: Arc::new(Vec::new()),
        sources: Vec::new(),
        upstream: Vec::new(),
        persistent: p_persistent,
      })),
    }
  }

  pub(crate) fn downgrade(&self) -> WeakSubject<T> {
    WeakSubject(Arc::downgrade(&self.inner))
  }

  /// Ties `p_source` to this stream's life, so it keeps listening to its own source until this stream is dropped.
  pub(crate) fn keep(&self, p_source: Subscription) {
    self.inner.lock().unwrap().sources.push(p_source);
  }

  /// Holds `p_upstream`, a source this stream was made from, until this stream is dropped.
  pub(crate) fn retain(&self, p_upstream: Box<dyn Any + Send + Sync>) {
    self.inner.lock().unwrap().upstream.push(p_upstream);
  }

  /// Sends `p_value` to every listener without taking ownership of it.
  pub(crate) fn dispatch(&self, p_value: &T) {
    let listeners = Arc::clone(&self.inner.lock().unwrap().listeners);
    for (_, listener) in listeners.iter() {
      listener(p_value);
    }
  }
}

impl<T: 'static> Observable<T> for Subject<T> {
  fn upstream(&self) -> Box<dyn Any + Send + Sync> {
    Box::new(self.clone())
  }

  fn is_persistent(&self) -> bool {
    self.inner.lock().unwrap().persistent
  }

  fn subscribe(&self, p_listener: impl Fn(&T) + Send + Sync + 'static) -> Subscription {
    let (id, persistent) = {
      let mut inner = self.inner.lock().unwrap();
      let id = inner.next_id;
      inner.next_id += 1;
      Arc::make_mut(&mut inner.listeners).push((id, Arc::new(p_listener)));
      (id, inner.persistent)
    };
    // Held strongly: a subscription keeps the stream it listens to alive, so `a.filter(..).subscribe(..)` keeps working
    // although the filtered stream in the middle is never stored. There is no cycle, because the listener inside the
    // stream does not hold the subscription.
    let inner = Arc::clone(&self.inner);
    let unsubscribe = move || {
      Arc::make_mut(&mut inner.lock().unwrap().listeners).retain(|(listener_id, _)| *listener_id != id);
    };
    if persistent { Subscription::detached(unsubscribe) } else { Subscription::new(unsubscribe) }
  }
}

impl<T: 'static> Observer<T> for Subject<T> {
  /// Sends `p_value` to every listener, in the order they subscribed. Listeners may subscribe, unsubscribe or call `next`
  /// from inside the call. A listener removed during a `next` can still hear that one value.
  fn next(&self, p_value: T) {
    self.dispatch(&p_value);
  }
}

/// A [`Subject`] that doesn't keep the stream alive.
pub(crate) struct WeakSubject<T>(Weak<Mutex<Inner<T>>>);

impl<T> Clone for WeakSubject<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> WeakSubject<T> {
  pub(crate) fn upgrade(&self) -> Option<Subject<T>> {
    self.0.upgrade().map(|inner| Subject { inner })
  }
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicUsize, Ordering};

  use super::*;

  fn counter() -> (Arc<AtomicUsize>, impl Fn(&i32) + Send + Sync + 'static) {
    let count = Arc::new(AtomicUsize::new(0));
    let listener = {
      let count = Arc::clone(&count);
      move |_: &i32| {
        count.fetch_add(1, Ordering::SeqCst);
      }
    };
    (count, listener)
  }

  #[test]
  fn listeners_hear_events_in_subscribe_order() {
    let subject = Subject::new();
    let heard = Arc::new(Mutex::new(Vec::new()));
    let first = {
      let heard = Arc::clone(&heard);
      subject.subscribe(move |event: &i32| heard.lock().unwrap().push(("first", *event)))
    };
    let second = {
      let heard = Arc::clone(&heard);
      subject.subscribe(move |event: &i32| heard.lock().unwrap().push(("second", *event)))
    };
    subject.next(7);
    assert_eq!(*heard.lock().unwrap(), vec![("first", 7), ("second", 7)]);
    drop((first, second));
  }

  #[test]
  fn a_late_listener_hears_nothing_from_before() {
    let subject = Subject::new();
    subject.next(1);
    let (count, listener) = counter();
    let _subscription = subject.subscribe(listener);
    assert_eq!(count.load(Ordering::SeqCst), 0);
  }

  #[test]
  fn dropping_a_subscription_stops_it() {
    let subject = Subject::new();
    let (count, listener) = counter();
    let subscription = subject.subscribe(listener);
    subject.next(1);
    drop(subscription);
    subject.next(2);
    assert_eq!(count.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn clones_share_one_stream() {
    let subject = Subject::new();
    let (count, listener) = counter();
    let _subscription = subject.subscribe(listener);
    subject.clone().next(1);
    assert_eq!(count.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn a_listener_can_call_next_and_subscribe_while_it_runs() {
    let subject = Subject::new();
    let (count, listener) = counter();
    let _counting = subject.subscribe(listener);
    let inner_subject = subject.clone();
    let inner_subscriptions = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&inner_subscriptions);
    let _reentrant = subject.subscribe(move |event: &i32| {
      if *event == 0 {
        inner_subject.next(1);
        kept.lock().unwrap().push(inner_subject.subscribe(|_| {}));
      }
    });
    subject.next(0);
    assert_eq!(count.load(Ordering::SeqCst), 2);
  }

  #[test]
  fn filter_passes_only_accepted_events() {
    let subject = Subject::new();
    let even = subject.filter(|event: &i32| event % 2 == 0);
    let heard = Arc::new(Mutex::new(Vec::new()));
    let _subscription = {
      let heard = Arc::clone(&heard);
      even.subscribe(move |event| heard.lock().unwrap().push(*event))
    };
    for value in 1..=4 {
      subject.next(value);
    }
    assert_eq!(*heard.lock().unwrap(), vec![2, 4]);
  }

  #[test]
  fn a_dropped_filter_stops_listening_to_its_source() {
    let subject = Subject::new();
    let even = subject.filter(|event: &i32| event % 2 == 0);
    drop(even);
    assert!(subject.inner.lock().unwrap().listeners.is_empty());
  }

  #[test]
  fn events_cross_threads() {
    let subject = Subject::new();
    let (count, listener) = counter();
    let _subscription = subject.subscribe(listener);
    let handles: Vec<_> = (0..4)
      .map(|_| {
        let subject = subject.clone();
        std::thread::spawn(move || (0..100).for_each(|value| subject.next(value)))
      })
      .collect();
    handles.into_iter().for_each(|handle| handle.join().unwrap());
    assert_eq!(count.load(Ordering::SeqCst), 400);
  }
}
