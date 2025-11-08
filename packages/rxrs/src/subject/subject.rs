use std::{any::Any, sync::Arc};

use crate::{
  observer::Observable,
  subscription::Subscription,
  traits::{Observer, Subscribable},
};

/// A subject is an observable that can be subscribed to and observed by multiple observers.
pub struct Subject<T>
where
  T: Any + Send + Sync,
{
  /// The base observable.
  base: Observable<T>,
}

impl<T> Subscribable<T> for Subject<T>
where
  T: Any + Send + Sync + Clone,
{
  fn subscribe<U>(&self, next: U) -> Arc<Subscription<T>>
  where
    U: Fn(T) + Send + Sync + 'static,
  {
    let observable = Observable::new(Some(Box::new(move |value| next(value))));
    let subscription = Subscription::new(observable);
    {
      let mut borrow = self.base.subscriptions.write().unwrap();
      borrow.push(Arc::new(subscription));
    }
    let borrow_sub = self.base.subscriptions.read().unwrap();
    let subscription = borrow_sub.last().unwrap();
    Arc::clone(subscription)
  }
}

impl<T> Subject<T>
where
  T: Any + Send + Sync + Clone,
{
  pub fn new() -> Self {
    Self {
      base: Observable::default(),
    }
  }
}

impl<T> Observer<T> for Subject<T>
where
  T: Any + Send + Sync + Clone,
{
  fn next(&self, value: T) {
    if !self.base.closed {
      for observer in self.base.subscriptions.read().unwrap().iter() {
        observer.next(value.clone());
      }
    }
  }

  fn error(&self, error: &str) {
    for observer in self.base.subscriptions.read().unwrap().iter() {
      observer.error(error);
    }
  }

  fn complete(&self) {
    for observer in self.base.subscriptions.read().unwrap().iter() {
      observer.complete();
    }
  }
}
