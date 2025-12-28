//! Options used when applying effects (filters, adjustments, or other image operations).
//!
//! This module provides a small builder struct used to configure optional parameters
//! that control how an operation is applied to an image. The two main options are:
//! - `Mask`: controls the per-pixel strength of the operation (black = no effect,
//!   white = full effect, grayscale = partial effect).
//! - `Area`: restricts the operation to a particular region (optionally feathered).

use abra_core::Area;
use abra_core::image::apply_area::ApplyContext;
use mask::Mask;

pub type Options = Option<ApplyOptions>;

/// Options for applying an effect (filter, adjustment, etc.) to an image.
/// ```ignore
/// use abra::{Area, Image, Heart, mask::Mask, options::ApplyOptions};
///
/// let mut image = Image::new_from_path("images/input.png");
/// let mask = Heart::new().fit(200, 200);
/// let area = Area::rect(10, 10, 100, 50);
///
/// let opts = ApplyOptions::new()
///   .with_mask(mask)
///   .with_area(area);
///
/// blur::gaussian_blur(&mut image, 5.0, opts);
/// ```
#[derive(Clone, Debug)]
pub struct ApplyOptions {
  /// Optional mask to be applied by the filter.
  /// If set, the filter will use this mask to determine how strong to apply the effect.
  /// Black areas will have no effect, white areas will have full effect,
  /// and grayscale will represent partial effect.
  mask: Option<Mask>,
  /// Optional area to be applied by the filter.
  /// If set, the filter will only be applied within this area.
  /// If an area has a feather on its edges, then the filter will be applied
  /// gradually from the edge of the area to the feathered region.
  area: Option<Vec<Area>>,
}

impl Default for ApplyOptions {
  fn default() -> Self {
    Self { mask: None, area: None }
  }
}

impl ApplyOptions {
  /// Create a new default set of apply options.
  pub fn new() -> Self {
    Self::default()
  }
  /// Gets the context representation of the options for use by core image helpers.
  pub fn ctx(&self) -> ApplyContext<'_> {
    ApplyContext {
      area: self.area.as_ref().map(|v| v.iter().collect()),
      mask_image: self.mask.as_ref().map(|m| m.image().rgba()),
    }
  }
  /// Sets a mask to be used by the filter.
  /// - `p_mask`: The `Mask` to apply; Black = no effect, White = full effect, grayscale = partial effect.
  pub fn with_mask(mut self, p_mask: impl Into<Mask>) -> Self {
    self.mask = Some(p_mask.into());
    self
  }
  /// Sets an area to be used by the filter.
  /// - `p_area`: The `Area` to apply; if set, the operation is restricted to this area and may be feathered.
  pub fn with_area(mut self, p_area: impl Into<Area>) -> Self {
    self.area = Some(vec![p_area.into()]);
    self
  }
  /// Sets multiple areas to be used by the filter.
  /// - `p_area`: A vector of `Area` to apply; if set, the operation is restricted to these areas and may be feathered.
  pub fn with_areas(mut self, p_area: impl Into<Vec<Area>>) -> Self {
    self.area = Some(p_area.into());
    self
  }
  /// Returns a reference to the mask if set.
  pub fn mask(&self) -> Option<&Mask> {
    self.mask.as_ref()
  }
  /// Returns a reference to the area if set.
  pub fn area(&self) -> Option<&[Area]> {
    self.area.as_deref()
  }
}

/// Convert an optional ApplyOptions into the lightweight core ApplyContext used by core helpers.
/// This helper lives in the `options` crate to avoid a circular dependency (core -> options -> core).
pub fn get_ctx<'a>(opts: Option<&'a ApplyOptions>) -> Option<ApplyContext<'a>> {
  opts.map(|o| ApplyContext {
    area: o.area().map(|v| v.iter().collect()),
    mask_image: o.mask().map(|m| m.image().rgba()),
  })
}
