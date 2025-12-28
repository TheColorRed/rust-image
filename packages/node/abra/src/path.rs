use crate::common::*;
use abra::abra_core::geometry::Path as AbraPath;

#[napi]
#[derive(Clone)]
pub struct Path {
  pub(crate) inner: AbraPath,
}

#[napi]
impl Path {
  #[napi(constructor)]
  pub fn new() -> Self {
    AbraPath::default().into()
  }
}

impl From<AbraPath> for Path {
  fn from(inner: AbraPath) -> Self {
    Self { inner }
  }
}
