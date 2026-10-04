//! Tests for the operators on [`Observable`](crate::Observable).

use std::sync::{Arc, Mutex};

use crate::prelude::*;

/// A listener that records what it hears, and the list it records into.
fn recorder<T: Clone + Send + 'static>() -> (Arc<Mutex<Vec<T>>>, impl Fn(&T) + Send + Sync + 'static) {
  let heard = Arc::new(Mutex::new(Vec::new()));
  let listener = {
    let heard = Arc::clone(&heard);
    move |value: &T| heard.lock().unwrap().push(value.clone())
  };
  (heard, listener)
}

#[test]
fn map_changes_each_value() {
  let source = Subject::new();
  let doubled = source.map(|value: &i32| value * 2);
  let (heard, listener) = recorder();
  let _subscription = doubled.subscribe(listener);
  source.next(1);
  source.next(2);
  assert_eq!(*heard.lock().unwrap(), vec![2, 4]);
}

#[test]
fn filter_map_keeps_only_what_it_turns_into_something() {
  let keys = Subject::new();
  let commands = keys.filter_map(|key: &&str| match *key {
    "up" => Some(1),
    "down" => Some(-1),
    _ => None,
  });
  let (heard, listener) = recorder();
  let _subscription = commands.subscribe(listener);
  for key in ["up", "x", "down", "y", "up"] {
    keys.next(key);
  }
  assert_eq!(*heard.lock().unwrap(), vec![1, -1, 1]);
}

#[test]
fn scan_folds_changes_into_state_you_can_read() {
  let changes = Subject::new();
  let total = changes.scan(10, |state: &i32, change: &i32| state + change);
  assert_eq!(total.value(), 10);

  let (heard, listener) = recorder();
  let _subscription = total.subscribe(listener);
  changes.next(1);
  changes.next(2);
  assert_eq!(total.value(), 13);
  // A new listener hears the state it starts from, then every change, like any BehaviorSubject.
  assert_eq!(*heard.lock().unwrap(), vec![10, 11, 13]);
}

#[test]
fn merge_passes_on_values_from_both_in_arrival_order() {
  let (left, right) = (Subject::new(), Subject::new());
  let merged = left.merge(&right);
  let (heard, listener) = recorder();
  let _subscription = merged.subscribe(listener);
  left.next(1);
  right.next(2);
  left.next(3);
  assert_eq!(*heard.lock().unwrap(), vec![1, 2, 3]);
}

#[test]
fn distinct_until_changed_skips_repeats() {
  let source = Subject::new();
  let distinct = source.distinct_until_changed();
  let (heard, listener) = recorder();
  let _subscription = distinct.subscribe(listener);
  for value in [1, 1, 2, 2, 2, 1] {
    source.next(value);
  }
  assert_eq!(*heard.lock().unwrap(), vec![1, 2, 1]);
}

#[test]
fn combine_latest_waits_for_both_then_sends_a_pair_for_each_change() {
  let (width, height) = (Subject::new(), Subject::new());
  let size = width.combine_latest(&height);
  let (heard, listener) = recorder();
  let _subscription = size.subscribe(listener);

  width.next(10);
  assert!(heard.lock().unwrap().is_empty(), "nothing until both have sent a value");
  height.next(20);
  width.next(11);
  height.next(21);
  assert_eq!(*heard.lock().unwrap(), vec![(10, 20), (11, 20), (11, 21)]);
}

#[test]
fn take_until_stops_for_good_when_the_notifier_sends() {
  let (source, stop) = (Subject::new(), Subject::new());
  let limited = source.take_until(&stop);
  let (heard, listener) = recorder();
  let _subscription = limited.subscribe(listener);
  source.next(1);
  source.next(2);
  stop.next(());
  source.next(3);
  source.next(4);
  assert_eq!(*heard.lock().unwrap(), vec![1, 2]);
}

#[test]
fn operators_chain_like_the_planned_input_example() {
  // Key presses become commands, and the commands fold into a brightness.
  let keys = Subject::new();
  let brightness = keys
    .filter_map(|key: &&str| match *key {
      "ArrowUp" => Some(1),
      "ArrowDown" => Some(-1),
      _ => None,
    })
    .scan(8, |brightness: &i32, step: &i32| (brightness + step).clamp(1, 10));

  for key in ["ArrowUp", "x", "ArrowUp", "ArrowUp", "ArrowDown"] {
    keys.next(key);
  }
  // 8 -> 9 -> 10 -> 10 (clamped) -> 9
  assert_eq!(brightness.value(), 9);
}

#[test]
fn a_dropped_result_stops_listening_to_its_source() {
  let source = Subject::new();
  let marker = Arc::new(());
  let mapped = {
    let marker = Arc::clone(&marker);
    source.map(move |value: &i32| {
      let _keep_alive = &marker;
      *value
    })
  };
  assert_eq!(Arc::strong_count(&marker), 2, "the operator holds a handle while it is alive");
  drop(mapped);
  assert_eq!(Arc::strong_count(&marker), 1, "dropping the result frees the operator and its source subscription");
}

#[test]
fn scan_does_not_keep_its_state_alive_after_it_is_dropped() {
  let source = Subject::new();
  let marker = Arc::new(());
  let state = {
    let marker = Arc::clone(&marker);
    source.scan(0, move |state: &i32, change: &i32| {
      let _keep_alive = &marker;
      state + change
    })
  };
  assert_eq!(Arc::strong_count(&marker), 2);
  drop(state);
  assert_eq!(Arc::strong_count(&marker), 1);
}

#[test]
fn a_subscription_keeps_a_chain_that_was_never_stored_alive() {
  let keys = Subject::new();
  let (heard, listener) = recorder();
  // The filtered stream in the middle is a temporary, and only the subscription is kept.
  let _subscription = keys.filter(|key: &&str| *key == "Escape").subscribe(listener);
  keys.next("a");
  keys.next("Escape");
  assert_eq!(*heard.lock().unwrap(), vec!["Escape"]);
}

#[test]
fn a_persistent_subject_keeps_its_listeners_when_the_subscription_is_dropped_and_only_unsubscribe_stops_one() {
  let subject = Subject::persistent();
  let (kept, kept_listener) = recorder::<i32>();
  let (stopped, stopped_listener) = recorder::<i32>();

  subject.subscribe(kept_listener); // the subscription is not held at all
  let subscription = subject.subscribe(stopped_listener);
  subject.next(1);
  subscription.unsubscribe();
  subject.next(2);

  assert_eq!(*kept.lock().unwrap(), vec![1, 2]);
  assert_eq!(*stopped.lock().unwrap(), vec![1]);
}

#[test]
fn an_ordinary_subject_still_stops_when_the_subscription_is_dropped() {
  let subject = Subject::new();
  let (heard, listener) = recorder::<i32>();
  let subscription = subject.subscribe(listener);
  subject.next(1);
  drop(subscription);
  subject.next(2);
  assert_eq!(*heard.lock().unwrap(), vec![1]);
}

#[test]
fn listeners_of_a_persistent_subject_go_with_the_subject() {
  let marker = Arc::new(());
  let subject = Subject::<i32>::persistent();
  subject.subscribe({
    let marker = Arc::clone(&marker);
    move |_| {
      let _keep_alive = &marker;
    }
  });
  assert_eq!(Arc::strong_count(&marker), 2);
  drop(subject);
  assert_eq!(Arc::strong_count(&marker), 1);
}

#[test]
fn what_a_persistent_stream_is_filtered_into_lives_as_long_as_the_stream_without_anyone_holding_it() {
  let events = Subject::<i32>::persistent();
  let (heard, listener) = recorder();
  // Neither the filtered stream nor the subscription is stored anywhere.
  events.filter(|value| *value > 1).map(|value| value * 10).subscribe(listener);
  events.next(1);
  events.next(2);
  assert_eq!(*heard.lock().unwrap(), vec![20]);
}
