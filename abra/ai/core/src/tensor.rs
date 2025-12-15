//! Tensor conversion utilities for AI models.
//!
//! This module provides helpers for converting between `Image` and tensor formats
//! commonly used by neural networks (NCHW layout).

use abra_core::Image;
use ndarray::Array4;

/// Converts an `Image` to an NCHW tensor (batch=1, channels=3, height, width).
///
/// Pixel values are normalized to [0.0, 1.0] range.
///
/// # Example
///
/// ```ignore
/// use abra_ai_core::tensor::image_to_nchw;
///
/// let tensor = image_to_nchw(&image);
/// // tensor shape: [1, 3, height, width]
/// ```
pub fn image_to_nchw(image: &Image) -> Array4<f32> {
  let (width, height) = image.dimensions::<u32>();
  let total_pixels = (width * height) as usize;

  let mut values = Vec::with_capacity(total_pixels * 3);

  // Organize as CHW: all R, then all G, then all B
  for channel in 0..3 {
    for y in 0..height {
      for x in 0..width {
        if let Some((r, g, b, _)) = image.get_pixel(x, y) {
          let val = match channel {
            0 => r as f32 / 255.0,
            1 => g as f32 / 255.0,
            2 => b as f32 / 255.0,
            _ => 0.0,
          };
          values.push(val);
        } else {
          values.push(0.0);
        }
      }
    }
  }

  Array4::from_shape_vec((1, 3, height as usize, width as usize), values).expect("Failed to create ndarray from image")
}

/// Converts NCHW tensor data back to an `Image`.
///
/// Expects float data in [0.0, 1.0] range, laid out as [R..., G..., B...]
/// (all red values, then all green, then all blue).
///
/// # Arguments
///
/// - `width`: Output image width
/// - `height`: Output image height
/// - `data`: Float data in NCHW layout (C=3 channels)
///
/// # Example
///
/// ```ignore
/// use abra_ai_core::tensor::nchw_to_image;
///
/// let image = nchw_to_image(256, 256, &tensor_data);
/// ```
pub fn nchw_to_image(width: u32, height: u32, data: &[f32]) -> Image {
  let num_pixels = (width * height) as usize;
  let hw = num_pixels;

  let mut rgba_data = vec![0u8; num_pixels * 4];

  for i in 0..num_pixels {
    // NCHW layout: R at [0..hw], G at [hw..2*hw], B at [2*hw..3*hw]
    let r = data.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
    let g = data.get(hw + i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
    let b = data.get(2 * hw + i).copied().unwrap_or(0.0).clamp(0.0, 1.0);

    rgba_data[i * 4] = (r * 255.0) as u8;
    rgba_data[i * 4 + 1] = (g * 255.0) as u8;
    rgba_data[i * 4 + 2] = (b * 255.0) as u8;
    rgba_data[i * 4 + 3] = 255;
  }

  let mut image = Image::new(width, height);
  image.set_new_pixels(&rgba_data, width, height);
  image
}

/// Configuration for tensor preprocessing.
#[derive(Clone, Debug)]
pub struct TensorConfig {
  /// Whether to normalize pixel values to [0, 1] (default: true)
  pub normalize: bool,
  /// Mean values to subtract per channel (RGB order)
  pub mean: Option<[f32; 3]>,
  /// Std values to divide by per channel (RGB order)
  pub std: Option<[f32; 3]>,
}

impl Default for TensorConfig {
  fn default() -> Self {
    Self {
      normalize: true,
      mean: None,
      std: None,
    }
  }
}

impl TensorConfig {
  /// Creates a new config with ImageNet normalization (mean/std).
  pub fn imagenet() -> Self {
    Self {
      normalize: true,
      mean: Some([0.485, 0.456, 0.406]),
      std: Some([0.229, 0.224, 0.225]),
    }
  }
}
