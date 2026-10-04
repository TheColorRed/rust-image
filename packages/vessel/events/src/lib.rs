//! A small event system in the style of RxJS.
//!
//! - [`Subject`]: a stream of events. Anything can send an event with `next`, and anything can `subscribe` to hear every
//!   event sent from then on.
//! - [`BehaviorSubject`]: a `Subject` that always holds a current value. A new subscriber hears the current value at once,
//!   then every change.
//!
//! Operators turn one stream into another, the way RxJS operators do: `filter`, `map`, `filter_map`, `scan` (which
//! returns a [`BehaviorSubject`] holding the running state), `merge`, `distinct_until_changed`, `combine_latest` and
//! `take_until`. They are methods of [`Observable`].
//!
//! `subscribe` and `filter` belong to the [`Observable`] trait and `next` to [`Observer`]; `use pub_sub::prelude::*`
//! brings them in. Subscribing returns a [`Subscription`]; dropping it stops the listening. Cloning a subject gives
//! another handle to the same stream. The crate knows nothing about images or rendering, so any part of the library can
//! use it.

mod behavior_subject;
#[cfg(test)]
mod operator_tests;
mod subject;
mod subscription;
pub mod tracking;
mod traits;

pub use self::{
  behavior_subject::BehaviorSubject,
  subject::Subject,
  subscription::Subscription,
  traits::{Observable, Observer},
};

/// Everything needed to use the crate. `subscribe`, `next` and `filter` are trait methods, so the traits must be in
/// scope.
pub mod prelude {
  pub use crate::{BehaviorSubject, Observable, Observer, Subject, Subscription};
}
