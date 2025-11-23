use crate::{observer::Observable, traits::UnaryFn};

pub fn map<In, Out, Fnc>(project: fn(In, i32) -> Out) -> UnaryFn<Observable<In>, Observable<Out>>
where
  In: Send + Sync + 'static,
  Out: Send + Sync + 'static,
  Fnc: Fn(In, i32) -> Out,
{
  todo!()
  // |source| {
  //   Observable::default()
  //   // Observable::new(Box::new(|destination| {
  //   //   let mut index = 0;
  //   //   // source.
  //   // }))
  // }
}
