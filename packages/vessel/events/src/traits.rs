use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::{BehaviorSubject, Subject, Subscription};

/// Something that can be listened to.
///
/// The provided methods are operators: each listens to this and returns a new stream, the way RxJS operators do. The new
/// stream listens to its source for as long as it, or a clone of it, is alive. Values a source sent before the operator
/// was applied are not replayed, except where noted (a [`BehaviorSubject`] source replays its current value to the
/// operator once, as it does to any new listener, but nobody is listening to the result yet).
pub trait Observable<T: 'static> {
  /// Calls `p_listener` with each value this sends, until the returned [`Subscription`] is dropped.
  ///
  /// - `p_listener`: Runs on the thread that sends the value.
  fn subscribe(&self, p_listener: impl Fn(&T) + Send + Sync + 'static) -> Subscription;

  /// A handle that keeps this stream alive. An operator holds the handle of each stream it listens to, so a chain such as
  /// `keys.filter_map(..).scan(..)` stays connected for as long as its last link does, even though the middle link is never
  /// stored in a variable. A stream that is already kept alive by something else can leave the default, which keeps nothing.
  fn upstream(&self) -> Box<dyn Any + Send + Sync> {
    Box::new(())
  }

  /// Whether listeners stay for as long as this stream lives (see [`Subject::persistent`]). What an operator makes from
  /// such a stream is persistent too, and lives as long as the stream does.
  fn is_persistent(&self) -> bool {
    false
  }

  /// Returns a [`Subject`] of only the values `p_predicate` accepts.
  fn filter(&self, p_predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> Subject<T>
  where
    Self: Sized,
  {
    let persistent = self.is_persistent();
    let filtered = if persistent { Subject::persistent() } else { Subject::new() };
    let weak = filtered.downgrade();
    // A persistent source keeps what it feeds alive, so nothing has to hold the result.
    let alive = persistent.then(|| filtered.clone());
    let source = self.subscribe(move |value| {
      let _alive = &alive;
      if p_predicate(value)
        && let Some(subject) = weak.upgrade()
      {
        subject.dispatch(value);
      }
    });
    if !persistent {
      filtered.keep(source);
      filtered.retain(self.upstream());
    }
    filtered
  }

  /// Returns a [`Subject`] of `p_map` applied to each value.
  fn map<U: 'static>(&self, p_map: impl Fn(&T) -> U + Send + Sync + 'static) -> Subject<U>
  where
    Self: Sized,
  {
    let persistent = self.is_persistent();
    let mapped = if persistent { Subject::persistent() } else { Subject::new() };
    let weak = mapped.downgrade();
    let alive = persistent.then(|| mapped.clone());
    let source = self.subscribe(move |value| {
      let _alive = &alive;
      if let Some(subject) = weak.upgrade() {
        subject.dispatch(&p_map(value));
      }
    });
    if !persistent {
      mapped.keep(source);
      mapped.retain(self.upstream());
    }
    mapped
  }

  /// Returns a [`Subject`] of what `p_map` returns for each value, skipping the values it returns `None` for. It is a
  /// `filter` and a `map` in one step, such as turning key presses into your own events and ignoring the other keys.
  fn filter_map<U: 'static>(&self, p_map: impl Fn(&T) -> Option<U> + Send + Sync + 'static) -> Subject<U>
  where
    Self: Sized,
  {
    let persistent = self.is_persistent();
    let mapped = if persistent { Subject::persistent() } else { Subject::new() };
    let weak = mapped.downgrade();
    let alive = persistent.then(|| mapped.clone());
    let source = self.subscribe(move |value| {
      let _alive = &alive;
      if let Some(mapped_value) = p_map(value)
        && let Some(subject) = weak.upgrade()
      {
        subject.dispatch(&mapped_value);
      }
    });
    if !persistent {
      mapped.keep(source);
      mapped.retain(self.upstream());
    }
    mapped
  }

  /// Folds the values into a running state and returns it as a [`BehaviorSubject`]: it starts at `p_seed`, becomes
  /// `p_step(state, value)` for each value, and always has its latest state available with `value()`. This is the
  /// reducer that turns a stream of changes into state, such as key presses into a brightness.
  ///
  /// Unlike RxJS, the seed is part of the result (it is the subject's first value), because a state you can read is what
  /// this is for. Values must not arrive from several threads at once, or two steps could start from the same state.
  fn scan<S: Clone + Send + Sync + 'static>(
    &self, p_seed: S, p_step: impl Fn(&S, &T) -> S + Send + Sync + 'static,
  ) -> BehaviorSubject<S>
  where
    Self: Sized,
  {
    let state = BehaviorSubject::new(p_seed);
    let weak = state.downgrade();
    let source = self.subscribe(move |value| {
      if let Some(state) = weak.upgrade() {
        let next = p_step(&state.peek(), value);
        state.next(next);
      }
    });
    state.keep(source);
    state.retain(self.upstream());
    state
  }

  /// Returns a [`Subject`] of the values of both this and `p_other`, in the order they arrive.
  fn merge(&self, p_other: &impl Observable<T>) -> Subject<T>
  where
    Self: Sized,
  {
    let merged = Subject::new();
    let forward = |weak: crate::subject::WeakSubject<T>| {
      move |value: &T| {
        if let Some(subject) = weak.upgrade() {
          subject.dispatch(value);
        }
      }
    };
    let first = self.subscribe(forward(merged.downgrade()));
    let second = p_other.subscribe(forward(merged.downgrade()));
    merged.keep(first);
    merged.keep(second);
    merged.retain(self.upstream());
    merged.retain(p_other.upstream());
    merged
  }

  /// Returns a [`Subject`] that skips a value equal to the one before it.
  fn distinct_until_changed(&self) -> Subject<T>
  where
    Self: Sized,
    T: Clone + PartialEq + Send + Sync,
  {
    let distinct = Subject::new();
    let weak = distinct.downgrade();
    let last: Mutex<Option<T>> = Mutex::new(None);
    let source = self.subscribe(move |value| {
      let changed = {
        let mut last = last.lock().unwrap();
        let changed = last.as_ref() != Some(value);
        if changed {
          *last = Some(value.clone());
        }
        changed
      };
      if changed && let Some(subject) = weak.upgrade() {
        subject.dispatch(value);
      }
    });
    distinct.keep(source);
    distinct.retain(self.upstream());
    distinct
  }

  /// Returns a [`Subject`] of `(this, other)` pairs, sent whenever either side changes once both have sent a value.
  fn combine_latest<U: Clone + Send + Sync + 'static>(&self, p_other: &impl Observable<U>) -> Subject<(T, U)>
  where
    Self: Sized,
    T: Clone + Send + Sync,
  {
    let combined = Subject::new();
    let latest: Arc<Mutex<(Option<T>, Option<U>)>> = Arc::new(Mutex::new((None, None)));

    let first = {
      let (latest, weak) = (Arc::clone(&latest), combined.downgrade());
      self.subscribe(move |value| {
        let pair = {
          let mut latest = latest.lock().unwrap();
          latest.0 = Some(value.clone());
          latest.0.clone().zip(latest.1.clone())
        };
        if let Some(pair) = pair
          && let Some(subject) = weak.upgrade()
        {
          subject.dispatch(&pair);
        }
      })
    };
    let second = {
      let (latest, weak) = (latest, combined.downgrade());
      p_other.subscribe(move |value| {
        let pair = {
          let mut latest = latest.lock().unwrap();
          latest.1 = Some(value.clone());
          latest.0.clone().zip(latest.1.clone())
        };
        if let Some(pair) = pair
          && let Some(subject) = weak.upgrade()
        {
          subject.dispatch(&pair);
        }
      })
    };
    combined.keep(first);
    combined.keep(second);
    combined.retain(self.upstream());
    combined.retain(p_other.upstream());
    combined
  }

  /// Returns a [`Subject`] that passes values on until `p_notifier` sends one, and then stops for good. Tie a stream to a
  /// lifetime with it, such as a component's input to the moment the component is removed.
  fn take_until<N: 'static>(&self, p_notifier: &impl Observable<N>) -> Subject<T>
  where
    Self: Sized,
  {
    let limited = Subject::new();
    let done = Arc::new(AtomicBool::new(false));
    let source = {
      let (done, weak) = (Arc::clone(&done), limited.downgrade());
      self.subscribe(move |value| {
        if !done.load(Ordering::SeqCst)
          && let Some(subject) = weak.upgrade()
        {
          subject.dispatch(value);
        }
      })
    };
    let notifier = p_notifier.subscribe(move |_| done.store(true, Ordering::SeqCst));
    limited.keep(source);
    limited.keep(notifier);
    limited.retain(self.upstream());
    limited.retain(p_notifier.upstream());
    limited
  }
}

/// Something that can be sent values.
pub trait Observer<T> {
  /// Sends `p_value` to whoever is listening.
  fn next(&self, p_value: T);
}
