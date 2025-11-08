use std::{
  any::Any,
  sync::{Arc, RwLock},
};

use crate::{
  observer::Observable,
  subscription::Subscription,
  traits::{Observer, Subscribable},
};

pub struct BehaviorSubject<T>
where
  T: Any + Send + Sync,
{
  /// The base observable.
  base: Observable<T>,
  /// The current value of the subject.
  value: RwLock<T>,
}

impl<T> AsRef<BehaviorSubject<T>> for BehaviorSubject<T>
where
  T: Any + Send + Sync,
{
  fn as_ref(&self) -> &BehaviorSubject<T> {
    &self
  }
}

impl<T> Subscribable<T> for BehaviorSubject<T>
where
  T: Any + Send + Sync + Clone,
{
  fn subscribe<U>(&self, next: U) -> Arc<Subscription<T>>
  where
    U: Fn(T) + Send + Sync + 'static,
  {
    let observable = Observable::new(Some(Box::new(next)));
    let subscription = Subscription::new(observable);
    subscription.next(self.value.read().unwrap().clone());
    {
      let mut borrow = self.base.subscriptions.write().unwrap();
      borrow.push(Arc::new(subscription));
    }
    let last = self.base.subscriptions.read().unwrap();
    let last = last.last().unwrap();
    Arc::clone(last)
  }
}

impl<T> BehaviorSubject<T>
where
  T: Any + Send + Sync + Clone,
{
  pub fn new(value: T) -> Self {
    Self {
      base: Observable::default(),
      value: RwLock::new(value),
    }
  }

  /// Returns the current value of the subject.
  pub fn value(&self) -> T {
    self.value.read().unwrap().clone()
  }
}

impl<T> Observer<T> for BehaviorSubject<T>
where
  T: Any + Send + Sync + Clone,
{
  fn next(&self, value: T) {
    if !self.base.closed {
      *self.value.write().unwrap() = value.clone();
      for observer in self.base.subscriptions.read().unwrap().iter() {
        observer.next(value.clone());
      }
    }
  }

  fn error(&self, error: &str) {
    for subscription in self.base.subscriptions.read().unwrap().iter() {
      subscription.error(error);
    }
  }

  fn complete(&self) {
    for subscription in self.base.subscriptions.read().unwrap().iter() {
      subscription.complete();
    }
  }
}
