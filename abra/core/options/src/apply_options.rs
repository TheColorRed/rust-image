//! Options used when applying effects (filters, adjustments, or other image operations).
//!
//! This module provides a small builder struct used to configure optional parameters
//! that control how an operation is applied to an image. The two main options are:
//! - `Mask`: controls the per-pixel strength of the operation (black = no effect,
//!   white = full effect, grayscale = partial effect).
//! - `Area`: restricts the operation to a particular region (optionally feathered).

use abra_core::image::apply_area::ApplyContext;
use abra_core::image::gpu::Hardware;
use abra_core::{Area, ImageRef};
use mask::Mask;
use std::sync::Arc;

pub type Options = Option<ApplyOptions>;

/// Options for applying an effect (filter, adjustment, etc.) to an image.
/// ```ignore
/// use abra::{Area, Image, Heart, mask::Mask, options::ApplyOptions};
///
/// let mut image = Image::read("images/input.png")?;
/// let mask = Area::shape(Shape::Heart).fit(200, 200);
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
  mask: Option<Arc<Mask>>,
  /// Optional area to be applied by the filter.
  /// If set, the filter will only be applied within this area.
  /// If an area has a feather on its edges, then the filter will be applied
  /// gradually from the edge of the area to the feathered region.
  area: Option<Arc<[Area]>>,
  /// Optional hardware preference for applying the effect.
  /// If set, the filter will attempt to use the specified hardware (CPU or GPU).
  hardware: Option<Hardware>,
}

/// Contract for configurable operations that apply in place to an image.
///
/// Implementors must provide both configuration and application, so builder
/// APIs cannot accidentally omit either operation.
pub trait Apply: Sized {
  /// Returns the options (area and mask) this operation was given, so a target that applies it later, such as a live
  /// image, can keep them with the operation.
  fn options(&self) -> &Options;

  /// Returns the builder's application options storage.
  /// Returns a mutable reference to the current `Options` for this builder.
  #[doc(hidden)]
  fn options_mut(&mut self) -> &mut Options;

  /// Restricts the operation with an optional area or mask.
  /// - `p_options`: The `ApplyOptions` containing the area and/or mask to use.
  fn with_options(mut self, p_options: impl Into<Options>) -> Self {
    *self.options_mut() = p_options.into();
    self
  }

  /// Applies the operation to an image. This is the one place an effect defines its own behavior; it picks the GPU or
  /// the CPU itself.
  /// - `p_image`: The image to apply the operation to.
  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>);

  /// Applies the operation to `p_target`: an image (edited now) or a live preview (re-rendered on every call).
  /// The call is the same either way; only the target differs.
  /// - `p_target`: What to apply the operation to.
  fn apply<T: ApplyTarget<Self>>(&self, p_target: T) {
    p_target.receive(self);
  }
}

/// Something an effect `E` can be applied to. Images and image references are targets automatically; other targets,
/// such as a live preview, implement this to take the effect in their own way.
pub trait ApplyTarget<E> {
  /// Takes `p_effect`.
  fn receive(self, p_effect: &E);
}

impl<'a, E: Apply, T: Into<ImageRef<'a>>> ApplyTarget<E> for T {
  fn receive(self, p_effect: &E) {
    p_effect.apply_to_image(self);
  }
}

impl Default for ApplyOptions {
  fn default() -> Self {
    Self {
      mask: None,
      area: None,
      hardware: Some(Hardware::Auto),
    }
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
      hardware: self.hardware.unwrap_or_default(),
    }
  }
  /// Sets a mask to be used by the filter.
  /// - `p_mask`: The `Mask` to apply; Black = no effect, White = full effect, grayscale = partial effect.
  pub fn with_mask(mut self, p_mask: impl Into<Mask>) -> Self {
    self.mask = Some(Arc::new(p_mask.into()));
    self
  }
  /// Sets an area to be used by the filter.
  /// - `p_area`: The `Area` to apply; if set, the operation is restricted to this area and may be feathered.
  pub fn with_area(mut self, p_area: impl Into<Area>) -> Self {
    self.area = Some(Arc::from(vec![p_area.into()]));
    self
  }
  /// Sets multiple areas to be used by the filter.
  /// - `p_area`: A vector of `Area` to apply; if set, the operation is restricted to these areas and may be feathered.
  pub fn with_areas(mut self, p_area: impl Into<Vec<Area>>) -> Self {
    self.area = Some(Arc::from(p_area.into()));
    self
  }
  /// Returns a reference to the mask if set.
  pub fn mask(&self) -> Option<&Mask> {
    self.mask.as_deref()
  }
  /// Returns a reference to the area if set.
  pub fn area(&self) -> Option<&[Area]> {
    self.area.as_deref()
  }
  /// Whether `p_other` is a copy of these options, made by cloning them. Cheap, and never true for two options that
  /// were built separately, even from equal values, so a `false` only means "may differ".
  pub fn is_copy_of(&self, p_other: &ApplyOptions) -> bool {
    fn same<T: ?Sized>(a: &Option<Arc<T>>, b: &Option<Arc<T>>) -> bool {
      match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        _ => false,
      }
    }
    same(&self.mask, &p_other.mask) && same(&self.area, &p_other.area) && self.hardware == p_other.hardware
  }
  /// Returns the hardware preference if set.
  pub fn hardware(&self) -> Option<&Hardware> {
    self.hardware.as_ref()
  }
  /// Sets the hardware preference for applying the effect.
  /// - `p_hardware`: The hardware preference to use (CPU, GPU, or Auto).
  pub fn with_hardware(mut self, p_hardware: impl Into<Hardware>) -> Self {
    self.hardware = Some(p_hardware.into());
    self
  }
}

/// Convert an optional ApplyOptions into the lightweight core ApplyContext used by core helpers.
/// This helper lives in the `options` crate to avoid a circular dependency (core -> options -> core).
pub fn get_ctx<'a>(p_opts: Option<&'a ApplyOptions>) -> Option<ApplyContext<'a>> {
  p_opts.map(|o| ApplyContext {
    area: o.area().map(|v| v.iter().collect()),
    mask_image: o.mask().map(|m| m.image().rgba()),
    hardware: o.hardware().copied().unwrap_or_default(),
  })
}
