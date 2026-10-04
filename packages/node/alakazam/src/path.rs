use crate::common::*;
use abra::abra_core::geometry::Path as AbraPath;

#[napi]
#[derive(Clone)]
pub struct GradientPath {
  pub(crate) inner: AbraPath,
}

#[napi]
impl GradientPath {
  #[napi(constructor)]
  pub fn new() -> Self {
    AbraPath::default().into()
  }

  #[napi]
  /// Creates a line path from the start point to the end point.
  /// @param start The starting point of the line.
  /// @param end The ending point of the line.
  pub fn line(start: (f64, f64), end: (f64, f64)) -> Self {
    AbraPath::line(start, end).into()
  }
}

impl From<AbraPath> for GradientPath {
  fn from(inner: AbraPath) -> Self {
    Self { inner }
  }
}
