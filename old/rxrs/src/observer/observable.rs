use std::{
  any::Any,
  sync::{Arc, RwLock},
};

use crate::{
  subscription::Subscription,
  traits::{Observer, Subscribable},
};

pub struct Observable<T>
where
  T: Any + Send + Sync,
{
  subscriber: Option<Box<dyn Fn(T) + Send + Sync>>,
  pub subscriptions: RwLock<Vec<Arc<Subscription<T>>>>,
  pub closed: bool,
}

impl<T> Default for Observable<T>
where
  T: Any + Send + Sync,
{
  fn default() -> Self {
    Self {
      subscriber: None,
      subscriptions: RwLock::new(Vec::new()),
      closed: false,
    }
  }
}

impl<T> Subscribable<T> for Observable<T>
where
  T: Any + Send + Sync,
{
  fn subscribe<U>(&self, next: U) -> Arc<Subscription<T>>
  where
    U: Fn(T) + Send + Sync + 'static,
  {
    let observable = Observable::new(Some(Box::new(next)));
    let subscription = Subscription::new(observable);
    {
      let mut borrow = self.subscriptions.write().unwrap();
      borrow.push(Arc::new(subscription));
    }
    let last = self.subscriptions.read().unwrap();
    let last = last.last().unwrap();
    Arc::clone(last)
  }
}

impl<T> Observer<T> for Observable<T>
where
  T: Any + Send + Sync,
{
  fn next(&self, value: T) {
    // let next = &self.next;
    // next(value);

    match self.subscriber.as_ref() {
      Some(cb) => cb(value),
      None => {}
    }

    // let x = Box::new(|_| {});
    // self.subscribe.as_ref().unwrap_or(&x)(value);
  }

  fn error(&self, error: &str) {
    // let err = &self.error;
    // err(String::from(error));
  }

  fn complete(&self) {
    // let complete = &self.complete;
    // complete();
  }
}

impl<T> Observable<T>
where
  T: Any + Send + Sync,
{
  pub fn new(subscribe: Option<Box<dyn Fn(T) + Send + Sync + 'static>>) -> Self {
    Self {
      subscriber: subscribe,
      subscriptions: RwLock::new(Vec::new()),
      closed: false,
    }
  }
}
