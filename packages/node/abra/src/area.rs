use crate::common::*;
use abra::abra_core::geometry::Area as AbraArea;

#[napi]
#[derive(Clone)]
pub struct Area {
  pub(crate) inner: AbraArea,
}

#[napi]
impl Area {
  #[napi(constructor)]
  /// Create a new default Area (empty).
  /// ```
  /// let area = Area.new();
  /// ```
  pub fn new() -> Self {
    AbraArea::default().into()
  }

  #[napi(factory)]
  /// Create a rectangular Area.
  /// @param position - The (x, y) position of the top-left corner of the rectangle.
  /// @param size - The (width, height) size of the rectangle.
  /// ```
  /// let area = Area.rect((10, 10), (100, 100));
  /// ```
  pub fn rect(position: (u32, u32), size: (u32, u32)) -> Self {
    AbraArea::rect(position, size).into()
  }
  /// Create an Area from a list of points.
  /// @param points - An array of [x, y] points defining the area polygon.
  /// ```
  /// let area = Area.fromPoints([[10, 10], [100, 10], [100, 100], [10, 100]]);
  /// ```
  #[napi(factory)]
  pub fn from_points(points: Vec<Vec<f64>>) -> Self {
    let points: &[[f32; 2]] = &points
      .iter()
      .map(|p| [p[0] as f32, p[1] as f32])
      .collect::<Vec<[f32; 2]>>();
    AbraArea::from_points(points).into()
  }

  #[napi]
  /// Set the feather amount of the Area.
  /// @param feather The feather amount in pixels.
  pub fn set_feather(&mut self, feather: u32) {
    let inner = self.inner.clone();
    self.inner = inner.with_feather(feather as u32);
  }
}

impl From<AbraArea> for Area {
  fn from(inner: AbraArea) -> Self {
    Self { inner }
  }
}
