use std::{any::Any, sync::RwLock};

use crate::traits::{Observer, TeardownLogic};

pub struct Subscription<T>
where
  T: Any + Send + Sync + 'static,
{
  /// A flag to indicate whether this subscription has already been unsubscribed.
  pub closed: RwLock<bool>,

  finalizers: Vec<TeardownLogic<T>>,

  destination: RwLock<Option<Box<dyn Observer<T> + Send + Sync + 'static>>>,
}

impl<T> Default for Subscription<T>
where
  T: Any + Send + Sync,
{
  fn default() -> Self {
    Self {
      closed: RwLock::new(false),
      finalizers: Vec::new(),
      destination: RwLock::new(None),
    }
  }
}

impl<T> Subscription<T>
where
  T: Any + Send + Sync,
{
  pub fn new<U>(destination: U) -> Self
  where
    U: Observer<T> + Send + Sync + 'static,
  {
    Self {
      closed: RwLock::new(false),
      finalizers: Vec::new(),
      destination: RwLock::new(Some(Box::new(destination))),
    }
  }

  pub fn add(&mut self, finalizer: TeardownLogic<T>) {
    if *self.closed.read().unwrap() {
      match finalizer {
        TeardownLogic::Function(f) => f(),
        TeardownLogic::Subscription(subscription) => subscription.unsubscribe(),
      }
    } else {
      self.finalizers.push(finalizer);
    }
  }

  /// Unsubscribes from the subscription.
  pub fn unsubscribe(&self) {
    // If the subscription is already closed, then return early.
    if self.closed.read().unwrap().clone() {
      return;
    }

    // Set the closed flag to true to indicate that the subscription has been unsubscribed.
    *self.closed.write().unwrap() = true;

    // Execute the initial teardown logic.
    match self.destination.read().unwrap().as_ref() {
      Some(destination) => destination.complete(),
      None => {}
    }

    for finalizer in self.finalizers.iter() {
      match finalizer {
        TeardownLogic::Function(f) => f(),
        TeardownLogic::Subscription(subscription) => subscription.unsubscribe(),
      }
    }
  }

  pub fn next(&self, value: T) {
    if *self.closed.read().unwrap() {
      return;
    }

    match self.destination.read().unwrap().as_ref() {
      Some(destination) => destination.next(value),
      None => {}
    }
  }

  pub fn error(&self, error: &str) {
    if *self.closed.read().unwrap() {
      return;
    }

    *self.closed.write().unwrap() = true;
    match self.destination.read().unwrap().as_ref() {
      Some(destination) => destination.error(error),
      None => {}
    }
  }

  pub fn complete(&self) {
    if *self.closed.read().unwrap() {
      return;
    }

    *self.closed.write().unwrap() = true;

    match self.destination.read().unwrap().as_ref() {
      Some(destination) => destination.complete(),
      None => {}
    }

    self.destination.write().unwrap().take();
  }
}
