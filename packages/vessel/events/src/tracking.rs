//! Finding out which values a piece of code read, so it can be run again when one of them changes.
//!
//! This is how a component redraws by itself: while its `draw` runs, every [`BehaviorSubject`](crate::BehaviorSubject)
//! (and so every property) that `value()` is called on tells the tracker, which then listens for that value's next
//! change. Nothing has to be declared.
//!
//! ```ignore
//! track(|read| { /* remember `read.id`; call `(read.on_change)(notify)` to hear its next change */ }, || {
//!   let level = brightness.value();   // reported to the tracker
//! });
//! ```

use std::cell::RefCell;
use std::sync::Arc;

use crate::Subscription;

/// A value that was just read.
pub struct Read<'a> {
  /// Which value: the same value always has the same id, whichever handle read it.
  pub id: usize,
  /// Asks to be told the next time the value changes. `notify` is called once for every change after the read (and at
  /// once if it already changed since). Dropping the returned [`Subscription`] stops the telling.
  pub on_change: &'a dyn Fn(Arc<dyn Fn() + Send + Sync>) -> Subscription,
}

type Tracker = Box<dyn FnMut(Read<'_>)>;

thread_local! {
  /// The tracker for the code running on this thread right now, if any.
  static TRACKER: RefCell<Option<Tracker>> = const { RefCell::new(None) };
}

/// Runs `p_run`, telling `p_tracker` about every value it reads on this thread. Trackers nest: the earlier one is back in
/// charge when `p_run` returns.
pub fn track<R>(p_tracker: impl FnMut(Read<'_>) + 'static, p_run: impl FnOnce() -> R) -> R {
  /// Puts the earlier tracker back, even if `p_run` panics.
  struct Restore(Option<Tracker>);
  impl Drop for Restore {
    fn drop(&mut self) {
      let earlier = self.0.take();
      TRACKER.with(|tracker| *tracker.borrow_mut() = earlier);
    }
  }

  let earlier = TRACKER.with(|tracker| tracker.borrow_mut().replace(Box::new(p_tracker)));
  let _restore = Restore(earlier);
  p_run()
}

/// Reports that the value `p_id` was read, if something is tracking. `p_on_change` is only called if it is.
pub(crate) fn read(p_id: usize, p_on_change: &dyn Fn(Arc<dyn Fn() + Send + Sync>) -> Subscription) {
  // The tracker is taken out while it runs, so a read it causes cannot reach it again.
  let tracker = TRACKER.with(|tracker| tracker.borrow_mut().take());
  let Some(mut tracker) = tracker else { return };
  tracker(Read {
    id: p_id,
    on_change: p_on_change,
  });
  TRACKER.with(|current| {
    let mut current = current.borrow_mut();
    if current.is_none() {
      *current = Some(tracker);
    }
  });
}

#[cfg(test)]
mod tests {
  use std::sync::Mutex;

  use super::*;
  use crate::{BehaviorSubject, Observer};

  #[test]
  fn reads_inside_track_are_reported_and_reads_outside_are_not() {
    let value = BehaviorSubject::new(1);
    let ids = Arc::new(Mutex::new(Vec::new()));

    value.value(); // no tracker yet
    let result = track(
      {
        let ids = Arc::clone(&ids);
        move |read| ids.lock().unwrap().push(read.id)
      },
      || value.value() + value.clone().value(),
    );
    assert_eq!(result, 2);
    let ids = ids.lock().unwrap();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], ids[1], "a clone is the same value");

    value.value(); // the tracker is gone again
    assert_eq!(ids.len(), 2);
  }

  #[test]
  fn the_tracker_hears_changes_after_the_read_and_ones_that_slipped_in_before_it_subscribed() {
    let value = BehaviorSubject::new(1);
    let changes = Arc::new(Mutex::new(0));
    let subscriptions = Arc::new(Mutex::new(Vec::new()));
    track(
      {
        let (changes, subscriptions) = (Arc::clone(&changes), Arc::clone(&subscriptions));
        move |read| {
          let changes = Arc::clone(&changes);
          let subscription = (read.on_change)(Arc::new(move || *changes.lock().unwrap() += 1));
          subscriptions.lock().unwrap().push(subscription);
        }
      },
      || value.value(),
    );
    assert_eq!(*changes.lock().unwrap(), 0);
    value.next(2);
    value.next(3);
    assert_eq!(*changes.lock().unwrap(), 2);

    subscriptions.lock().unwrap().clear();
    value.next(4);
    assert_eq!(*changes.lock().unwrap(), 2, "dropping the subscription stops the telling");
  }

  #[test]
  fn trackers_nest_and_the_outer_one_is_restored() {
    let (outer, inner) = (Arc::new(Mutex::new(0)), Arc::new(Mutex::new(0)));
    let value = BehaviorSubject::new(0);
    track(
      {
        let outer = Arc::clone(&outer);
        move |_| *outer.lock().unwrap() += 1
      },
      || {
        value.value();
        track(
          {
            let inner = Arc::clone(&inner);
            move |_| *inner.lock().unwrap() += 1
          },
          || value.value(),
        );
        value.value();
      },
    );
    assert_eq!((*outer.lock().unwrap(), *inner.lock().unwrap()), (2, 1));
  }
}
