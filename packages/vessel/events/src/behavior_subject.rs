use std::any::Any;
use std::sync::{
  Arc, Mutex, Weak,
  atomic::{AtomicU64, Ordering},
};

use crate::subject::WeakSubject;
use crate::{Observable, Observer, Subject, Subscription};

/// A value and how many times the subject has been set, so a listener can tell a stale delivery from a newer one.
struct Versioned<T> {
  version: u64,
  value: T,
}

/// A [`Subject`] that always holds a current value. A new subscriber hears the current value straight away, then every
/// change, so it never has to wait for the next change to know the state.
///
/// A listener never hears an older value after a newer one, even when `subscribe` and `next` run on different threads at
/// the same time.
///
/// ```ignore
/// let brightness = BehaviorSubject::new(0.0);
/// brightness.next(0.3);
/// let subscription = brightness.subscribe(|value| { /* hears 0.3 now, then every change */ });
/// ```
pub struct BehaviorSubject<T> {
  current: Arc<Mutex<Arc<Versioned<T>>>>,
  subject: Subject<Arc<Versioned<T>>>,
}

impl<T> Clone for BehaviorSubject<T> {
  fn clone(&self) -> Self {
    Self {
      current: Arc::clone(&self.current),
      subject: self.subject.clone(),
    }
  }
}

impl<T: Default + 'static> Default for BehaviorSubject<T> {
  fn default() -> Self {
    Self::new(T::default())
  }
}

impl<T: 'static> BehaviorSubject<T> {
  /// Creates a subject holding `p_value`.
  pub fn new(p_value: T) -> Self {
    Self {
      current: Arc::new(Mutex::new(Arc::new(Versioned {
        // Starts at 1: a listener has heard version 0, meaning nothing yet.
        version: 1,
        value: p_value,
      }))),
      subject: Subject::new(),
    }
  }

  /// A handle that does not keep the subject alive, for an operator that listens to a source and feeds this subject.
  pub(crate) fn downgrade(&self) -> WeakBehaviorSubject<T> {
    WeakBehaviorSubject {
      current: Arc::downgrade(&self.current),
      subject: self.subject.downgrade(),
    }
  }

  /// Ties `p_source` to this subject's life, so it keeps listening to its own source until this subject is dropped.
  pub(crate) fn keep(&self, p_source: Subscription) {
    self.subject.keep(p_source);
  }

  /// Holds `p_upstream`, a source this subject was made from, until this subject is dropped.
  pub(crate) fn retain(&self, p_upstream: Box<dyn Any + Send + Sync>) {
    self.subject.retain(p_upstream);
  }

  /// The current value. If something is tracking what is read (see [`tracking`](crate::tracking)), it is told, so it can be
  /// run again when this value changes.
  pub fn value(&self) -> T
  where
    T: Clone + Send + Sync,
  {
    let current = Arc::clone(&self.current.lock().unwrap());
    crate::tracking::read(Arc::as_ptr(&self.current) as usize, &|notify| self.changes_after(current.version, notify));
    current.value.clone()
  }

  /// The current value, without telling anything that is tracking reads.
  pub fn peek(&self) -> T
  where
    T: Clone,
  {
    self.current.lock().unwrap().value.clone()
  }

  /// Calls `p_notify` for every change after `p_version`, including one that already happened.
  fn changes_after(&self, p_version: u64, p_notify: Arc<dyn Fn() + Send + Sync>) -> Subscription
  where
    T: Send + Sync,
  {
    // Subscribed before the version is checked, so no change can slip between the two.
    let subscription = {
      let notify = Arc::clone(&p_notify);
      self.subject.subscribe(move |_| notify())
    };
    if self.current.lock().unwrap().version != p_version {
      p_notify();
    }
    subscription
  }
}

/// A [`BehaviorSubject`] that does not keep itself alive.
pub(crate) struct WeakBehaviorSubject<T> {
  current: Weak<Mutex<Arc<Versioned<T>>>>,
  subject: WeakSubject<Arc<Versioned<T>>>,
}

impl<T> WeakBehaviorSubject<T> {
  pub(crate) fn upgrade(&self) -> Option<BehaviorSubject<T>> {
    Some(BehaviorSubject {
      current: self.current.upgrade()?,
      subject: self.subject.upgrade()?,
    })
  }
}

impl<T: Send + Sync + 'static> Observable<T> for BehaviorSubject<T> {
  fn upstream(&self) -> Box<dyn Any + Send + Sync> {
    Box::new(self.clone())
  }

  /// Calls `p_listener` with the current value now, then with every change, until the returned [`Subscription`] is
  /// dropped. The first call runs on the calling thread.
  fn subscribe(&self, p_listener: impl Fn(&T) + Send + Sync + 'static) -> Subscription {
    let newest = AtomicU64::new(0);
    let deliver = Arc::new(move |p_versioned: &Versioned<T>| {
      if newest.fetch_max(p_versioned.version, Ordering::SeqCst) < p_versioned.version {
        p_listener(&p_versioned.value);
      }
    });
    // Subscribing before reading the current value means no change can slip between the two.
    let subscription = self.subject.subscribe({
      let deliver = Arc::clone(&deliver);
      move |versioned| deliver(versioned)
    });
    let current = Arc::clone(&self.current.lock().unwrap());
    deliver(&current);
    subscription
  }
}

impl<T: 'static> Observer<T> for BehaviorSubject<T> {
  /// Makes `p_value` the current value and sends it to every listener, in the order they subscribed.
  fn next(&self, p_value: T) {
    let versioned = {
      let mut current = self.current.lock().unwrap();
      *current = Arc::new(Versioned {
        version: current.version + 1,
        value: p_value,
      });
      Arc::clone(&current)
    };
    self.subject.next(versioned);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn recorder() -> (Arc<Mutex<Vec<i32>>>, impl Fn(&i32) + Send + Sync + 'static) {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let listener = {
      let heard = Arc::clone(&heard);
      move |value: &i32| heard.lock().unwrap().push(*value)
    };
    (heard, listener)
  }

  #[test]
  fn a_new_subscriber_hears_the_current_value_then_changes() {
    let subject = BehaviorSubject::new(1);
    subject.next(2);
    let (heard, listener) = recorder();
    let _subscription = subject.subscribe(listener);
    subject.next(3);
    assert_eq!(*heard.lock().unwrap(), vec![2, 3]);
  }

  #[test]
  fn value_is_the_latest() {
    let subject = BehaviorSubject::new(1);
    assert_eq!(subject.value(), 1);
    subject.next(5);
    assert_eq!(subject.value(), 5);
  }

  #[test]
  fn dropping_a_subscription_stops_it() {
    let subject = BehaviorSubject::new(0);
    let (heard, listener) = recorder();
    let subscription = subject.subscribe(listener);
    drop(subscription);
    subject.next(1);
    assert_eq!(*heard.lock().unwrap(), vec![0]);
  }

  #[test]
  fn clones_share_the_value_and_the_listeners() {
    let subject = BehaviorSubject::new(0);
    let (heard, listener) = recorder();
    let _subscription = subject.subscribe(listener);
    let other = subject.clone();
    other.next(4);
    assert_eq!(subject.value(), 4);
    assert_eq!(*heard.lock().unwrap(), vec![0, 4]);
  }

  #[test]
  fn a_listener_can_set_the_value_while_it_runs() {
    let subject = BehaviorSubject::new(0);
    let inner = subject.clone();
    let (heard, record) = recorder();
    let _subscription = subject.subscribe(move |value| {
      record(value);
      if *value == 1 {
        inner.next(2);
      }
    });
    subject.next(1);
    assert_eq!(*heard.lock().unwrap(), vec![0, 1, 2]);
  }

  /// Takes the roles, not the types: any `Observable` works, whichever subject it is.
  fn hear_all(p_source: &impl Observable<i32>) -> (Arc<Mutex<Vec<i32>>>, Subscription) {
    let (heard, listener) = recorder();
    (heard, p_source.subscribe(listener))
  }

  #[test]
  fn code_can_take_any_observable() {
    let behavior = BehaviorSubject::new(1);
    let plain = Subject::new();
    let (from_behavior, _a) = hear_all(&behavior);
    let (from_plain, _b) = hear_all(&plain);
    behavior.next(2);
    plain.next(2);
    assert_eq!(*from_behavior.lock().unwrap(), vec![1, 2]);
    assert_eq!(*from_plain.lock().unwrap(), vec![2]);
  }

  #[test]
  fn filter_works_on_a_behavior_subject() {
    let subject = BehaviorSubject::new(0);
    let even = subject.filter(|value| value % 2 == 0);
    let (heard, listener) = recorder();
    let _subscription = even.subscribe(listener);
    for value in 1..=4 {
      subject.next(value);
    }
    assert_eq!(*heard.lock().unwrap(), vec![2, 4]);
  }

  #[test]
  fn a_listener_never_hears_an_older_value_after_a_newer_one() {
    let subject = BehaviorSubject::new(0);
    let in_order = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let writer = {
      let subject = subject.clone();
      std::thread::spawn(move || (1..=2000).for_each(|value| subject.next(value)))
    };
    let subscriptions: Vec<_> = (0..50)
      .map(|_| {
        let in_order = Arc::clone(&in_order);
        let seen = AtomicU64::new(0);
        subject.subscribe(move |value| {
          if seen.swap(*value as u64, Ordering::SeqCst) > *value as u64 {
            in_order.store(false, Ordering::SeqCst);
          }
        })
      })
      .collect();
    writer.join().unwrap();
    drop(subscriptions);
    assert!(in_order.load(Ordering::SeqCst));
  }
}
