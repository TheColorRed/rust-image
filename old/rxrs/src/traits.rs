use std::{any::Any, sync::Arc};

use crate::{observer::Observable, subscription::Subscription};

pub enum TeardownLogic<T>
where
  T: Any + Send + Sync,
{
  Subscription(Subscription<T>),
  Function(Box<dyn Fn() + Send + Sync>),
}

pub type UnaryFn<T, R> = fn(T) -> R;
pub type OperatorFn<T, R> = UnaryFn<Observable<T>, Observable<R>>;

pub trait Observer<T>
where
  T: Any + Send + Sync,
{
  fn next(&self, value: T);
  fn error(&self, error: &str);
  fn complete(&self);
}

pub trait Subscribable<T>
where
  T: Any + Send + Sync,
{
  /// Subscribes to the subject or observable and returns a subscription.
  fn subscribe<U>(&self, next: U) -> Arc<Subscription<T>>
  where
    U: Fn(T) + Send + Sync + 'static;

  // fn pipe<In, Out>(&mut self, operators: Vec<UnaryFunction<In, Out>>) -> &mut Self
  // where
  //   In: Any + Send + 'static,
  //   Out: Subscribable<In>;
}
