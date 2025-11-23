use std::{
  any::Any,
  sync::{Arc, RwLock},
};

use crate::{
  observer::Observable,
  subscription::Subscription,
  traits::{Observer, Subscribable},
};

pub struct ReplaySubject<T>
where
  T: Any + Send + Sync,
{
  /// The base observable.
  base: Observable<T>,
  /// The buffer size which determines how many values to store and replay upon subscription.
  buffer_size: u32,
  /// The values that have been observed.
  values: RwLock<Vec<T>>,
}

impl<T> Subscribable<T> for ReplaySubject<T>
where
  T: Any + Send + Sync + Clone,
{
  fn subscribe<U>(&self, next: U) -> Arc<Subscription<T>>
  where
    U: Fn(T) + Send + Sync + 'static,
  {
    let observable = Observable::new(Some(Box::new(next) as Box<dyn Fn(T) + Send + Sync>));
    let subscription = Subscription::new(observable);

    // Send the buffered values to the new observer.
    for value in self.values.read().unwrap().iter() {
      subscription.next(value.clone());
    }

    {
      let mut borrow = self.base.subscriptions.write().unwrap();
      borrow.push(Arc::new(subscription));
    }
    let last = self.base.subscriptions.read().unwrap();
    let last = last.last().unwrap();
    Arc::clone(last)
  }
}

impl<T> ReplaySubject<T>
where
  T: Any + Send + Sync + Clone,
{
  pub fn new(buffer_size: u32) -> Self {
    Self {
      buffer_size,
      base: Observable::default(),
      values: RwLock::new(Vec::new()),
    }
  }
}

impl<T> Observer<T> for ReplaySubject<T>
where
  T: Any + Send + Sync + Clone,
{
  fn next(&self, value: T) {
    if !self.base.closed {
      self.values.write().unwrap().push(value.clone());

      // Remove the oldest value if the buffer is full.
      if self.values.read().unwrap().len() > self.buffer_size as usize {
        self.values.write().unwrap().remove(0);
      }

      for observer in self.base.subscriptions.read().unwrap().iter() {
        observer.next(value.clone());
      }
    }
  }

  fn error(&self, error: &str) {
    for subscriptions in self.base.subscriptions.read().unwrap().iter() {
      subscriptions.error(error);
    }
  }

  fn complete(&self) {
    for subscriptions in self.base.subscriptions.read().unwrap().iter() {
      subscriptions.complete();
    }
  }
}
