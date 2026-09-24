use abra::options::prelude::ApplyOptions as AbraApplyOptions;

use crate::Mask;
use crate::area::Area;
use crate::common::*;

#[napi]
pub struct ApplyOptions {
  area: Option<Vec<Area>>,
  mask: Option<Mask>,
}

impl Default for ApplyOptions {
  fn default() -> Self {
    Self { area: None, mask: None }
  }
}

#[napi]
impl ApplyOptions {
  #[napi(constructor)]
  pub fn new(area: Option<Vec<&Area>>, mask: Option<&Mask>) -> Self {
    Self {
      area: area.map(|areas| areas.into_iter().cloned().collect()),
      mask: mask.cloned(),
    }
  }

  #[napi(getter)]
  pub fn area(&self) -> Option<Vec<Area>> {
    self.area.clone()
  }

  #[napi(getter)]
  pub fn mask(&self) -> Option<Mask> {
    self.mask.clone()
  }

  #[napi]
  pub fn set_area(&mut self, area: Option<Vec<&Area>>) -> &Self {
    self.area = area.map(|areas| areas.into_iter().cloned().collect());
    self
  }

  #[napi]
  pub fn set_mask(&mut self, mask: Option<&Mask>) -> &Self {
    self.mask = mask.cloned();
    self
  }

  pub(crate) fn to_apply_options(&self) -> AbraApplyOptions {
    let mut apply_opts = AbraApplyOptions::new();
    if let Some(area) = &self.area {
      let areas: Vec<abra::prelude::Area> = area.iter().map(|a| a.inner.clone()).collect();
      apply_opts = apply_opts.with_areas(areas);
    }
    if let Some(mask) = &self.mask {
      apply_opts = apply_opts.with_mask(mask.inner.clone());
    }
    apply_opts
  }
}
