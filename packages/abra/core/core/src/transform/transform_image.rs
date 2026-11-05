use crate::image::gpu::{GpuPass, GpuProcessor, Hardware};
use crate::transform::resize::target_dimensions;
use crate::{Image, IntoNumber, Rect};

use super::resize::{resample_passes, resample_to};
use super::rotate::{turn, turn_passes};
use super::{FlipAxis, ResizeTarget, TransformAlgorithm, TransformFit, crop, flip};

/// One thing a transform does to the pixels, in the order it is done.
#[derive(Clone, Copy)]
enum Step {
  /// Resamples to this size.
  Resize { width: u32, height: u32 },
  /// Mirrors across the axis.
  Flip(FlipAxis),
  /// Rotates, scaling by this much per axis in the same pass.
  Turn { scale: (f32, f32) },
}

/// Everything that moves an image's pixels, described together and run in one go: a crop, a size or scale, a rotation, and flips.
/// Create one with [`transform`], set what it should do with the `with_*` methods, then run it with [`TransformImage::apply`].
///
/// The order is chosen here, not by the caller, to keep the result sharp and the work small:
/// - A crop is an exact copy of pixels, so it is done first and the rest works on fewer pixels.
/// - A shrink is made before a rotation, with the resize, which looks at all the pixels it replaces. The rotation then reads about as
///   many pixels as it writes.
/// - An enlargement is made together with the rotation, each pixel read once from the original, so the picture is resampled a single
///   time. Without a rotation it is an ordinary resize.
/// - A flip is an exact mirror, so it is done on whichever picture is smaller.
#[derive(Clone)]
pub struct TransformImage {
  crop: Option<Rect>,
  size: Option<ResizeTarget>,
  rotation: f32,
  flip_horizontal: bool,
  flip_vertical: bool,
  fit: TransformFit,
  algorithm: Option<TransformAlgorithm>,
  hardware: Hardware,
}

impl TransformImage {
  /// Keeps only the area, clipped to the image. A [`Size`] keeps that size from the top-left corner. An area that does not overlap the
  /// image is ignored.
  pub fn with_crop(mut self, p_area: impl Into<Rect>) -> Self {
    self.crop = Some(p_area.into());
    self
  }

  /// Resizes to the target, after any crop and before any rotation. See [`ResizeTarget`].
  pub fn with_size(mut self, p_target: ResizeTarget) -> Self {
    self.size = Some(p_target);
    self
  }

  /// Resizes both dimensions by `p_scale`, after any crop and before any rotation.
  pub fn with_scale(self, p_scale: f32) -> Self {
    self.with_size(ResizeTarget::Scale(p_scale))
  }

  /// Rotates clockwise by `p_degrees` about the middle. Positive values turn clockwise, negative values counter-clockwise.
  pub fn with_rotation(mut self, p_degrees: impl IntoNumber) -> Self {
    self.rotation = p_degrees.into::<f32>();
    self
  }

  /// Mirrors across the axis, before any rotation. Flipping the same axis twice puts the image back.
  pub fn with_flip(mut self, p_axis: FlipAxis) -> Self {
    match p_axis {
      FlipAxis::Horizontal => self.flip_horizontal = !self.flip_horizontal,
      FlipAxis::Vertical => self.flip_vertical = !self.flip_vertical,
    }
    self
  }

  /// Sets how the canvas is sized after a rotation. Defaults to [`TransformFit::Expand`].
  pub fn with_fit(mut self, p_fit: TransformFit) -> Self {
    self.fit = p_fit;
    self
  }

  /// Sets the interpolation algorithm. When `None` (the default), the best one is picked from how much the image is scaled.
  pub fn with_algorithm(mut self, p_algorithm: impl Into<Option<TransformAlgorithm>>) -> Self {
    self.algorithm = p_algorithm.into();
    self
  }

  /// Chooses the hardware that does the resampling: the CPU, the GPU (the CPU when there is no GPU, or it fails), or `Auto`, which
  /// follows the `gpu.enabled` setting.
  pub fn with_hardware(mut self, p_hardware: Hardware) -> Self {
    self.hardware = p_hardware;
    self
  }

  /// The steps that make this transform of an image of `p_width` x `p_height` (after any crop), in the order they are run. The order is
  /// the one described on [`TransformImage`]: a shrink, the flips, then the rotation, or the enlargement when there is none.
  fn plan(&self, p_width: u32, p_height: u32) -> Vec<Step> {
    let target = self.size.and_then(|target| target_dimensions(p_width, p_height, target)).filter(|&(w, h)| w > 0 && h > 0);
    let (width, height) = target.unwrap_or((p_width, p_height));
    let turning = self.rotation.rem_euclid(360.0) > f32::EPSILON;
    let shrinking = width < p_width || height < p_height;

    let mut steps = Vec::new();
    if shrinking {
      steps.push(Step::Resize { width, height });
    }
    if self.flip_horizontal {
      steps.push(Step::Flip(FlipAxis::Horizontal));
    }
    if self.flip_vertical {
      steps.push(Step::Flip(FlipAxis::Vertical));
    }
    if turning {
      // The enlargement, if any, is made by the rotation, which reads the original pixels once.
      let scale = if shrinking { (1.0, 1.0) } else { (width as f32 / p_width as f32, height as f32 / p_height as f32) };
      steps.push(Step::Turn { scale });
    } else if !shrinking && (width, height) != (p_width, p_height) {
      steps.push(Step::Resize { width, height });
    }
    steps
  }

  /// Runs the transform on the image in place. Nothing happens for what was not asked for, or does not change the image.
  pub fn apply(&self, p_image: &mut Image) {
    if let Some(area) = self.crop {
      let (left, top, right, bottom) = area.edges::<u32>();
      crop(left, top, right.saturating_sub(left), bottom.saturating_sub(top)).apply(p_image);
    }

    let (width, height) = p_image.dimensions::<u32>();
    for step in self.plan(width, height) {
      match step {
        Step::Resize { width, height } => resample_to(p_image, width, height, self.algorithm, self.hardware),
        Step::Flip(axis) => flip(axis).apply(p_image),
        Step::Turn { scale } => turn(p_image, self.rotation, scale, self.fit, self.algorithm, self.hardware),
      }
    }
  }

  /// Returns the transformed copy of the image and leaves the original as it was.
  pub fn applied(&self, p_image: &Image) -> Image {
    let mut copy = p_image.clone();
    self.apply(&mut copy);
    copy
  }
}

impl GpuProcessor for TransformImage {
  /// The passes that resize and rotate an image of the given size, in order, for the GPU to run. A crop and flips are exact copies of
  /// pixels that are done on the CPU, so they are not part of the passes and the size is the one after the crop.
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let mut passes = Vec::new();
    let mut size = (p_width, p_height);
    for step in self.plan(p_width, p_height) {
      match step {
        Step::Resize { width, height } => {
          passes.extend(resample_passes(size, (width, height), self.algorithm));
          size = (width, height);
        }
        Step::Flip(_) => {}
        Step::Turn { scale } => passes.extend(turn_passes(size, self.rotation, scale, self.fit, self.algorithm)),
      }
    }
    passes
  }
}

/// Starts a transform that does nothing yet. Add a crop, a size or scale, a rotation or flips with the `with_*` methods, then run it with
/// [`TransformImage::apply`].
/// ```ignore
/// transform().with_scale(2.0).with_rotation(90.0).with_algorithm(TransformAlgorithm::Bicubic).apply(&mut image);
/// ```
pub fn transform() -> TransformImage {
  TransformImage {
    crop: None,
    size: None,
    rotation: 0.0,
    flip_horizontal: false,
    flip_vertical: false,
    fit: TransformFit::Expand,
    algorithm: None,
    hardware: Hardware::Auto,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Channels;

  fn image(p_width: u32, p_height: u32) -> Image {
    Image::new_from_pixels(p_width, p_height, &[10, 20, 30, 255].repeat((p_width * p_height) as usize), Channels::RGBA)
  }

  #[test]
  fn a_transform_with_nothing_in_it_changes_nothing() {
    let mut img = image(6, 4);
    let before = img.rgba().to_vec();
    transform().apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (6, 4));
    assert_eq!(img.rgba(), before.as_slice());
  }

  #[test]
  fn the_size_is_that_of_the_crop_then_the_scale_then_the_rotation() {
    let mut img = image(40, 20);
    transform().with_crop(Rect::new((0, 0), (20, 10))).with_scale(2.0).with_rotation(90.0).with_hardware(Hardware::Cpu).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (20, 40));
  }

  #[test]
  fn a_shrink_with_a_rotation_ends_at_the_size_of_the_rotated_target() {
    let mut img = image(40, 20);
    transform().with_scale(0.5).with_rotation(90.0).with_hardware(Hardware::Cpu).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (10, 20));
  }

  #[test]
  fn an_enlargement_with_a_rotation_ends_at_the_size_of_the_rotated_target() {
    let mut img = image(6, 4);
    transform().with_scale(2.0).with_rotation(90.0).with_hardware(Hardware::Cpu).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (8, 12));
  }

  #[test]
  fn flipping_before_a_rotation_mirrors_the_picture() {
    let pixels: Vec<u8> = (0..6u8).flat_map(|i| [i, 0, 0, 255]).collect();
    let mut img = Image::new_from_pixels(3, 2, pixels, Channels::RGBA);
    transform().with_flip(FlipAxis::Horizontal).apply(&mut img);
    assert_eq!(img.get_pixel(0, 0), Some((2, 0, 0, 255)));
    transform().with_flip(FlipAxis::Horizontal).apply(&mut img);
    assert_eq!(img.get_pixel(0, 0), Some((0, 0, 0, 255)));
  }

  #[test]
  fn a_crop_off_the_image_is_ignored() {
    let mut img = image(8, 8);
    transform().with_crop(Rect::new((50, 50), (5, 5))).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (8, 8));
  }
}
