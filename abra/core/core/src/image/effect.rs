//! Options used when applying effects (filters, adjustments, or other image operations).
//!
//! This module provides a small builder struct used to configure optional parameters
//! that control how an operation is applied to an image. The two main options are:
//! - `Mask`: controls the per-pixel strength of the operation (black = no effect,
//!   white = full effect, grayscale = partial effect).
//! - `Area`: restricts the operation to a particular region (optionally feathered).

use crate::image::apply_area::{
  ApplyContext, apply_in_area, apply_in_area_gpu, apply_to_whole_image_in_area, apply_to_whole_image_in_area_gpu,
};
use crate::image::gpu::{CpuProcessor, GpuEffect, GpuProcessor, Hardware, ready_gpu_provider};
use crate::{Area, GrayPlane, Image, ImageRef};
use std::sync::Arc;

pub type Options = Option<ApplyOptions>;

/// Options for applying an effect (filter, adjustment, etc.) to an image.
/// ```ignore
/// use abra::{Area, Image, Heart, mask::Mask, options::ApplyOptions};
///
/// let mut image = Image::read("images/input.png")?;
/// let mask = Mask::from_image(Image::read("images/mask.png")?);
/// let area = Area::rect((10, 10), (100, 50));
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
  mask: Option<GrayPlane>,
  /// Optional area to be applied by the filter.
  /// If set, the filter will only be applied within this area.
  /// If an area has a feather on its edges, then the filter will be applied
  /// gradually from the edge of the area to the feathered region.
  area: Option<Arc<[Area]>>,
  /// Optional hardware preference for applying the effect.
  /// If set, the filter will attempt to use the specified hardware (CPU or GPU).
  hardware: Option<Hardware>,
}

/// What an effect (an adjustment, filter or other image operation) is. An effect declares only what is its own: where
/// it keeps its options, how much of the pixels around an area it needs, what it does to pixels on the CPU, and its
/// shader if it has one. Everything else is provided: limiting it to an area and mask, choosing the GPU or the CPU,
/// and working as a step in a live image's chain.
///
/// ```ignore
/// impl Apply for Sharpen {
///   fn options(&self) -> &ApplyOptions { &self.options }
///   fn options_mut(&mut self) -> &mut ApplyOptions { &mut self.options }
///   fn padding(&self) -> i32 { 1 }
///   fn cpu_processor(&self, p_image: &mut Image) { apply_sharpen(p_image); }
/// }
/// ```
///
/// An effect with a shader also implements [`GpuProcessor`] and returns itself from
/// [`gpu_processor`](Self::gpu_processor).
pub trait Effect: Sized + Send + Sync {
  /// Returns the options (area and mask) this operation was given.
  fn options(&self) -> &Options;

  /// Returns a mutable reference to the options for this operation.
  #[doc(hidden)]
  fn options_mut(&mut self) -> &mut Options;

  /// How many pixels around an area the operation reads to work out a pixel inside it, such as a blur's radius.
  /// Defaults to `0`, for operations that only look at the pixel they change.
  fn padding(&self) -> i32 {
    0
  }

  /// Whether the operation depends on where a pixel is in the image, or on the image's size, such as a gradient, or
  /// something centered on the image. Defaults to `false`.
  ///
  /// By default an operation limited to an area is run on a crop of the image around it, which is faster but moves
  /// the origin: a gradient across the whole image would start again at the crop's corner. An operation that returns
  /// `true` runs over the whole image and is then limited to the area and mask, so it sees the same coordinates it
  /// would without an area.
  fn positional(&self) -> bool {
    false
  }

  /// What the operation does to pixels on the CPU. It gets the whole image, or the part of it inside an area plus
  /// [`padding`](Self::padding), and knows nothing about areas or masks: those are applied around it.
  fn cpu_processor(&self, p_image: &mut Image);

  /// The GPU version of the operation, or `None` (the default) when it only runs on the CPU. Override with
  /// `Some(self)` after implementing [`GpuProcessor`].
  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    None
  }

  /// Restricts the operation with an optional area or mask.
  /// - `p_options`: The `ApplyOptions` containing the area and/or mask to use.
  fn with_options(mut self, p_options: impl Into<Options>) -> Self {
    *self.options_mut() = p_options.into();
    self
  }

  /// Applies the operation to an image now, limited to its area and mask, on whichever hardware suits. This is the
  /// router: it uses the GPU when the operation has a shader and the hardware setting (the operation's, else
  /// `gpu.enabled` in settings) allows it and a GPU is ready, and the CPU in every other case, including when the GPU
  /// fails. Call [`apply_on_cpu`](Self::apply_on_cpu) or [`apply_on_gpu`](Self::apply_on_gpu) to choose yourself.
  /// - `p_image`: The image to apply the operation to.
  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    let hardware = self.options().as_ref().and_then(|options| options.hardware().copied()).unwrap_or_default();
    if self.gpu_processor().is_some() && ready_gpu_provider(hardware).is_some() && self.apply_on_gpu(image).is_ok() {
      return;
    }
    self.apply_on_cpu(image);
  }

  /// Applies the operation on the CPU, limited to its area and mask. It never uses the GPU, whatever the settings.
  /// - `p_image`: The image to apply the operation to.
  fn apply_on_cpu(&self, p_image: &mut Image) {
    let ctx = get_ctx(self.options().as_ref());
    if self.positional() {
      apply_to_whole_image_in_area(p_image, ctx, |p_whole_image| self.cpu_processor(p_whole_image));
    } else {
      apply_in_area(p_image, ctx, self.padding(), None, |p_area_image| self.cpu_processor(p_area_image));
    }
  }

  /// Applies the operation on the GPU, limited to its area and mask. It never uses the CPU: it returns an error when
  /// the operation has no GPU version, no GPU is available, or the GPU fails, and leaves the image as it was.
  /// - `p_image`: The image to apply the operation to.
  fn apply_on_gpu(&self, p_image: &mut Image) -> Result<(), String> {
    if self.gpu_processor().is_none() {
      return Err("this operation has no GPU version".to_string());
    }
    let ctx = get_ctx(self.options().as_ref());
    if self.positional() {
      apply_to_whole_image_in_area_gpu(p_image, ctx, self as &dyn GpuEffect)
    } else {
      apply_in_area_gpu(p_image, ctx, self.padding(), self as &dyn GpuEffect)
    }
  }

  /// Applies the operation to `p_target`: an image (edited now) or a live image (re-rendered on every call).
  /// The call is the same either way; only the target differs.
  /// - `p_target`: What to apply the operation to.
  fn apply<T: ApplyTarget<Self>>(&self, p_target: T) {
    p_target.receive(self);
  }
}

impl<T: Effect> CpuProcessor for T {
  fn process(&self, p_image: &mut Image) {
    self.apply_on_cpu(p_image);
  }

  fn gpu(&self) -> Option<&dyn GpuProcessor> {
    self.gpu_processor()
  }
}

/// Something an effect `E` can be applied to. Images and image references are targets automatically; other targets,
/// such as a live preview, implement this to take the effect in their own way.
pub trait ApplyTarget<E> {
  /// Takes `p_effect`.
  fn receive(self, p_effect: &E);
}

impl<'a, E: Effect, T: Into<ImageRef<'a>>> ApplyTarget<E> for T {
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
      mask: self.mask.as_ref().map(|m| m.values()),
      hardware: self.hardware.unwrap_or_default(),
    }
  }
  /// Sets a mask to be used by the filter.
  /// - `p_mask`: The mask to apply, as a `Mask`, a [`GrayPlane`] or an image (its brightness is used). Black = no
  ///   effect, white = full effect, grayscale = partial effect.
  pub fn with_mask(mut self, p_mask: impl Into<GrayPlane>) -> Self {
    self.mask = Some(p_mask.into());
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
  /// Returns the mask if set.
  pub fn mask(&self) -> Option<&GrayPlane> {
    self.mask.as_ref()
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
    let same_mask = match (&self.mask, &p_other.mask) {
      (None, None) => true,
      (Some(a), Some(b)) => a.shares_values_with(b),
      _ => false,
    };
    same_mask && same(&self.area, &p_other.area) && self.hardware == p_other.hardware
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

/// Convert an optional ApplyOptions into the lightweight ApplyContext used by the area helpers.
pub fn get_ctx<'a>(p_opts: Option<&'a ApplyOptions>) -> Option<ApplyContext<'a>> {
  p_opts.map(|o| ApplyContext {
    area: o.area().map(|v| v.iter().collect()),
    mask: o.mask().map(|m| m.values()),
    hardware: o.hardware().copied().unwrap_or_default(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Channels;

  /// Writes the column of each pixel, as the effect sees it, into the red channel.
  #[derive(Clone)]
  struct Columns {
    positional: bool,
    options: Options,
  }

  impl Effect for Columns {
    fn options(&self) -> &Options {
      &self.options
    }

    fn options_mut(&mut self) -> &mut Options {
      &mut self.options
    }

    fn positional(&self) -> bool {
      self.positional
    }

    fn cpu_processor(&self, p_image: &mut Image) {
      let (width, _) = p_image.dimensions::<usize>();
      let mut pixels = p_image.to_rgba_vec();
      for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
        pixel[0] = (index % width) as u8;
      }
      p_image.set_rgba(pixels);
    }
  }

  fn red_at(p_image: &Image, p_x: usize, p_y: usize) -> u8 {
    p_image.to_rgba_vec()[(p_y * 32 + p_x) * 4]
  }

  fn apply_in_a_square(p_positional: bool) -> Image {
    let mut image = Image::new_from_pixels(32, 8, [0, 0, 0, 255].repeat(32 * 8), Channels::RGBA);
    let area = Area::rect((10.0, 2.0), (4.0, 3.0));
    let effect = Columns {
      positional: p_positional,
      options: Some(ApplyOptions::new().with_area(area)),
    };
    effect.apply_on_cpu(&mut image);
    image
  }

  #[test]
  fn an_operation_limited_to_an_area_sees_a_crop_unless_it_is_positional() {
    // Without the flag it runs on a crop whose first column is image column 10, so column 12 is seen as column 2.
    assert_eq!(red_at(&apply_in_a_square(false), 12, 3), 2);
    // With it, the operation runs over the whole image and sees column 12 as column 12.
    assert_eq!(red_at(&apply_in_a_square(true), 12, 3), 12);
  }

  #[test]
  fn a_positional_operation_still_only_changes_the_area() {
    let image = apply_in_a_square(true);
    assert_eq!(red_at(&image, 5, 3), 0, "left of the area");
    assert_eq!(red_at(&image, 20, 3), 0, "right of the area");
    assert_eq!(red_at(&image, 12, 0), 0, "above the area");
    assert_eq!(red_at(&image, 12, 6), 0, "below the area");
  }
}
