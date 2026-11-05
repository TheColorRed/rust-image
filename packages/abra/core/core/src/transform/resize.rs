use crate::image::gpu::{GpuPass, Hardware, ready_gpu_provider};
use crate::transform::TransformAlgorithm;
use crate::transform::interpolation;
use crate::{Channels, Image, Size};

/// The shader that resizes on the GPU, with the samplers it shares with rotating in front. See the comments in the files.
const RESIZE_SHADER: &str = concat!(include_str!("./sampling.wgsl"), include_str!("./resize.wgsl"));

/// Describes how an image should be resized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResizeTarget {
  /// Resize to exact dimensions without preserving aspect ratio.
  Exact(Size),
  /// Resize to a width while preserving aspect ratio.
  FitWidth(u32),
  /// Resize to a height while preserving aspect ratio.
  FitHeight(u32),
  /// Resize to the largest size that fits inside the given size while preserving aspect ratio.
  Fit(Size),
  /// Change the width by a number of pixels while preserving aspect ratio.
  RelativeWidth(i32),
  /// Change the height by a number of pixels while preserving aspect ratio.
  RelativeHeight(i32),
  /// Resize both dimensions by a positive scale factor.
  Scale(f32),
}

/// Determine the best resize algorithm based on the original and target dimensions.
/// If no algorithm is specified, this function selects an appropriate algorithm:
/// - If the target size is less than half the original size, Lanczos is chosen for high quality downscaling.
/// - If the target size is larger than the original size, Bicubic is chosen for quality upscaling.
/// - If the target size is smaller than the original size but not less than half, Bilinear is chosen for a good balance.
/// - If the target size is the same as the original size, Bilinear is used as a default.
pub(crate) fn get_resize_algorithm(
  p_algorithm: impl Into<Option<TransformAlgorithm>>, p_old_width: u32, p_old_height: u32, p_width: u32, p_height: u32,
) -> TransformAlgorithm {
  match p_algorithm.into() {
    Some(TransformAlgorithm::Auto) | None => {
      // Uses the Lanczos algorithm when downscaling more than half for best quality.
      if p_width < p_old_width / 2 || p_height < p_old_height / 2 {
        TransformAlgorithm::Lanczos
      }
      // Uses Bicubic when upscaling for better quality.
      else if p_width > p_old_width || p_height > p_old_height {
        TransformAlgorithm::Bicubic
      }
      // Uses Bilinear for moderate downscaling.
      else if p_width < p_old_width || p_height < p_old_height {
        TransformAlgorithm::Bilinear
      } else {
        TransformAlgorithm::Bicubic
      }
    }
    Some(algo) => algo,
  }
}

/// Resolves an image resize target to exact pixel dimensions, for an image of `p_old_width` x `p_old_height`.
pub(crate) fn target_dimensions(p_old_width: u32, p_old_height: u32, p_target: ResizeTarget) -> Option<(u32, u32)> {
  let (old_width, old_height) = (p_old_width, p_old_height);
  match p_target {
    ResizeTarget::Exact(size) => Some((size.width.max(0.0) as u32, size.height.max(0.0) as u32)),
    ResizeTarget::FitWidth(width) => {
      let height = ((old_height as f32 / old_width as f32 * width as f32) as u32).max(1);
      Some((width, height))
    }
    ResizeTarget::FitHeight(height) => {
      let width = (old_width as f32 / old_height as f32 * height as f32) as u32;
      Some((width, height))
    }
    ResizeTarget::Fit(size) => {
      let scale = (size.width / old_width as f32).min(size.height / old_height as f32);
      target_dimensions(p_old_width, p_old_height, ResizeTarget::Scale(scale))
    }
    ResizeTarget::RelativeWidth(amount) => {
      let width = (old_width as i32 + amount).max(1) as u32;
      target_dimensions(p_old_width, p_old_height, ResizeTarget::FitWidth(width))
    }
    ResizeTarget::RelativeHeight(amount) => {
      let height = (old_height as i32 + amount).max(1) as u32;
      target_dimensions(p_old_width, p_old_height, ResizeTarget::FitHeight(height))
    }
    ResizeTarget::Scale(scale) if scale > 0.0 => {
      Some(((old_width as f32 * scale).max(1.0) as u32, (old_height as f32 * scale).max(1.0) as u32))
    }
    ResizeTarget::Scale(_) => None,
  }
}

/// The resampled pixels of `p_image` at `p_width` x `p_height`, from the GPU when `p_hardware` allows it, and from the CPU otherwise,
/// including when the GPU fails.
fn resample_pixels(
  p_image: &Image, p_width: u32, p_height: u32, p_algorithm: TransformAlgorithm, p_hardware: Hardware,
) -> Vec<u8> {
  let (old_width, old_height) = p_image.dimensions::<u32>();
  if let Some(provider) = ready_gpu_provider(p_hardware)
    && let Ok(pixels) = (provider.process)(
      &[resample_passes((old_width, old_height), (p_width, p_height), Some(p_algorithm))],
      old_width,
      old_height,
      p_image.rgba(),
    )
  {
    return pixels;
  }
  interpolation::resample(p_image, p_width, p_height, p_algorithm.interpolation())
}

/// Resamples the image to `p_width` x `p_height` on the hardware `p_hardware` names. `None` picks the algorithm from how much the image
/// is scaled. Nothing happens when the size is the same as the image's.
pub(crate) fn resample_to(
  p_image: &mut Image, p_width: u32, p_height: u32, p_algorithm: Option<TransformAlgorithm>, p_hardware: Hardware,
) {
  let (old_width, old_height) = p_image.dimensions::<u32>();
  if (p_width == old_width && p_height == old_height) || p_width == 0 || p_height == 0 {
    return;
  }
  let algorithm = get_resize_algorithm(p_algorithm, old_width, old_height, p_width, p_height);
  let pixels = resample_pixels(p_image, p_width, p_height, algorithm, p_hardware);
  p_image.set_pixels(p_width, p_height, pixels, Channels::RGBA);
}

/// The one pass that resamples an image of `p_from` to `p_to` on the GPU (see [`GpuPass::output`]), or none when the size is the same.
pub(crate) fn resample_passes(
  p_from: (u32, u32), p_to: (u32, u32), p_algorithm: Option<TransformAlgorithm>,
) -> Vec<GpuPass> {
  if p_from == p_to || p_to.0 == 0 || p_to.1 == 0 {
    return Vec::new();
  }
  let algorithm = get_resize_algorithm(p_algorithm, p_from.0, p_from.1, p_to.0, p_to.1);
  // The shader finds the spot an output pixel reads from the sizes of its two textures, so all it is told is the algorithm.
  let uniforms: Vec<u8> = [algorithm.interpolation().shader_mode().to_le_bytes(), [0; 4], [0; 4], [0; 4]].concat();
  vec![GpuPass::new(RESIZE_SHADER, uniforms).with_output(p_to.0, p_to.1).with_pixel_exact()]
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::transform;

  fn image(p_width: u32, p_height: u32) -> Image {
    Image::new_from_pixels(p_width, p_height, &[10, 20, 30, 255].repeat((p_width * p_height) as usize), Channels::RGBA)
  }

  #[test]
  fn builder_resizes_to_the_target() {
    let mut img = image(20, 10);
    transform().with_hardware(Hardware::Cpu).with_size(ResizeTarget::Exact(Size::new(8, 4))).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (8, 4));

    transform().with_hardware(Hardware::Cpu).with_size(ResizeTarget::FitWidth(16)).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (16, 8));
  }

  #[test]
  fn resized_returns_a_copy_and_leaves_the_source() {
    let img = image(40, 20);
    let copy = transform().with_hardware(Hardware::Cpu).with_size(ResizeTarget::Fit(Size::new(10, 10))).applied(&img);
    assert_eq!(copy.dimensions::<u32>(), (10, 5));
    assert_eq!(img.dimensions::<u32>(), (40, 20));
    // An interior pixel: resampling blends edge pixels with transparent samples outside the image.
    let interior = (2 * 10 + 5) * 4;
    assert_eq!(&copy.rgba()[interior..interior + 4], &[10, 20, 30, 255]);
  }

  #[test]
  fn fit_preserves_aspect_ratio_inside_the_box() {
    let mut img = image(40, 20);
    transform().with_hardware(Hardware::Cpu).with_size(ResizeTarget::Fit(Size::new(10, 10))).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (10, 5));

    let mut img = image(20, 40);
    transform().with_hardware(Hardware::Cpu).with_size(ResizeTarget::Fit(Size::new(10, 10))).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (5, 10));
  }

  #[test]
  fn builder_accepts_an_algorithm() {
    for algorithm in [TransformAlgorithm::NearestNeighbor, TransformAlgorithm::Lanczos] {
      let mut img = image(20, 10);
      transform().with_hardware(Hardware::Cpu).with_size(ResizeTarget::Scale(2.0)).with_algorithm(algorithm).apply(&mut img);
      assert_eq!(img.dimensions::<u32>(), (40, 20));
    }
  }

  #[test]
  fn builder_leaves_an_image_alone_when_the_size_is_unchanged() {
    let mut img = image(6, 3);
    let before = img.rgba().to_vec();
    transform().with_hardware(Hardware::Cpu).with_size(ResizeTarget::Exact(Size::new(6, 3))).apply(&mut img);
    assert_eq!(img.dimensions::<u32>(), (6, 3));
    assert_eq!(img.rgba(), before.as_slice());
  }

  #[test]
  fn edge_directed_algorithms_resize() {
    for algorithm in [TransformAlgorithm::EdgeDirectNEDI, TransformAlgorithm::EdgeDirectEDI] {
      let mut img = image(10, 6);
      transform().with_hardware(Hardware::Cpu).with_size(ResizeTarget::Scale(2.0)).with_algorithm(algorithm).apply(&mut img);
      assert_eq!(img.dimensions::<u32>(), (20, 12), "{algorithm}");
      // A flat color stays flat away from the transparent border.
      assert_eq!(img.get_pixel(10, 6), Some((10, 20, 30, 255)), "{algorithm}");
    }
  }
}
