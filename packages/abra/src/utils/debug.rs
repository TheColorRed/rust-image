use crate::{
  canvas::{DropShadowOptions, StrokeOptions},
  color::Gradient,
  geometry::path::Path,
  transform::ResizeAlgorithm,
};

#[cfg(debug_assertions)]
use crate::combine::blend::blend_mode_name;

use std::time::Duration;

#[cfg(debug_assertions)]
macro_rules! debug_println {
    ($($arg:tt)*) => { println!($($arg)*) }
}

#[cfg(not(debug_assertions))]
macro_rules! debug_println {
  ($($arg:tt)*) => {
    ()
  };
}

/// Enum representing different types of debug information to print.
pub enum DebugInfo {
  /// Image opened information.
  /// - `file_path`: The path of the image file opened
  /// - `width`: Width of the opened image
  /// - `height`: Height of the opened image
  /// - `duration`: Time taken to open the image
  ImageOpened(String, u32, u32, Duration),
  /// Image saved information.
  /// - `file_path`: The path of the image file saved
  /// - `width`: Width of the saved image
  /// - `height`: Height of the saved image
  /// - `duration`: Time taken to save the image
  ImageSaved(String, u32, u32, Duration),
}
/// Enum representing different transform debug entries.
pub enum DebugTransform {
  /// Resizing information.
  /// - `algorithm`: The resize algorithm used
  /// - `old_width`: Original width of the image
  /// - `old_height`: Original height of the image
  /// - `new_width`: New width after resizing
  /// - `new_height`: New height after resizing
  /// - `duration`: Time taken to perform the resize
  Resize(ResizeAlgorithm, u32, u32, u32, u32, Duration),
  /// Cropping information.
  /// - `old_width`: Original width of the image
  /// - `old_height`: Original height of the image
  /// - `new_width`: New width after cropping
  /// - `new_height`: New height after cropping
  /// - `duration`: Time taken to perform the crop
  Crop(u32, u32, u32, u32, Duration),
  /// Flipping information.
  /// - `direction`: The direction of the flip ("horizontal" or "vertical")
  /// - `width`: Width of the image after flipping
  /// - `height`: Height of the image after flipping
  /// - `duration`: Time taken to perform the flip
  Flip(String, u32, u32, Duration),
  /// Rotation information.
  /// - `algorithm`: The interpolation algorithm used
  /// - `degrees`: The degrees of the rotation
  /// - `old_width`: Width of the image before rotation
  /// - `old_height`: Height of the image before rotation
  /// - `new_width`: Width of the image after rotation
  /// - `new_height`: Height of the image after rotation
  /// - `duration`: Time taken to perform the rotation
  Rotate(ResizeAlgorithm, f32, u32, u32, u32, u32, Duration),
}
/// Enum representing different effect debug entries.
pub enum DebugEffects {
  /// Drop shadow effect applied.
  DropShadow(DropShadowOptions, Duration),
  /// Stroke effect applied.
  Stroke(StrokeOptions, Duration),
}
/// Enum representing different filter debug entries.
pub enum DebugFilters {
  /// Gaussian blur filter applied.
  /// - `radius`: The blur radius in pixels
  /// - `duration`: Time taken to perform the blur
  GaussianBlur(f32, Duration),
}
/// Enum representing different drawing debug entries.
pub enum DebugDrawing {
  /// Gradient drawing applied.
  /// - `gradient`: The gradient used for drawing
  /// - `path`: The path along which the gradient was drawn
  /// - `duration`: Time taken to perform the drawing
  Gradient(Gradient, Path, Duration),
}

/// Implementations for general logging debug information.
impl DebugInfo {
  /// Logs the debug information to the console for the given PrintInfo variant.
  #[allow(unused_variables)]
  pub fn log(self) {
    match self {
      // Primary outputs; these do not contain indentation
      DebugInfo::ImageOpened(file_path, width, height, duration) => {
        debug_println!("Image Opened: filename={}; dimensions={}x{}; time={:?}", file_path, width, height, duration)
      }
      DebugInfo::ImageSaved(file_path, width, height, duration) => {
        debug_println!("Image Saved: filename={}; dimensions={}x{}; time={:?}", file_path, width, height, duration)
      }
    }
  }
}
/// Implementations for logging transform debug information.
impl DebugTransform {
  /// Logs the debug information to the console for the given Transform variant.
  #[allow(unused_variables)]
  pub fn log(self) {
    match self {
      // Secondary outputs; these contain indentation
      // #region: Transform Outputs
      DebugTransform::Resize(algorithm, old_width, old_height, new_width, new_height, duration) => debug_println!(
        "    Transform::Resize: algorithm={} original={}x{}; new={}x{}; time={:?}",
        algorithm,
        old_width,
        old_height,
        new_width,
        new_height,
        duration
      ),
      DebugTransform::Crop(old_width, old_height, new_width, new_height, duration) => debug_println!(
        "    Transform::Crop: original={}x{}; new={}x{}; time={:?}",
        old_width,
        old_height,
        new_width,
        new_height,
        duration
      ),
      DebugTransform::Flip(direction, width, height, duration) => {
        debug_println!(
          "    Transform::Flip: direction={}; dimensions={}x{}; time={:?}",
          direction,
          width,
          height,
          duration
        )
      }
      DebugTransform::Rotate(algorithm, degrees, old_width, old_height, new_width, new_height, duration) => {
        debug_println!(
          "    Transform::Rotate: algorithm={}; degrees={}; original={}x{}; new={}x{}; time={:?}",
          algorithm,
          degrees,
          old_width,
          old_height,
          new_width,
          new_height,
          duration
        )
      }
    }
  }
}
/// Implementations for logging effect debug information.
impl DebugEffects {
  /// Logs the debug information to the console for the given Effects variant.
  #[allow(unused_variables)]
  pub fn log(self) {
    match self {
      // #endregion: Transform Outputs
      DebugEffects::DropShadow(options, duration) => debug_println!(
        "    Effect::DropShadow: distance={}; opacity={}; angle={}; size={}; spread={}; blend_mode={}; fill={}; time={:?}",
        options.distance,
        options.opacity,
        options.angle,
        options.size,
        options.spread,
        blend_mode_name(options.blend_mode),
        options.fill,
        duration
      ),
      DebugEffects::Stroke(options, duration) => {
        debug_println!("    Effect::Stroke: fill={}; size={}; time={:?}", options.fill, options.size, duration)
      }
    }
  }
}
/// Implementations for logging filter debug information.
impl DebugFilters {
  /// Logs the debug information to the console for the given Filters variant.
  #[allow(unused_variables)]
  pub fn log(self) {
    match self {
      DebugFilters::GaussianBlur(radius, duration) => {
        debug_println!("    Filter::GaussianBlur: radius={}; time={:?}", radius, duration)
      }
    }
  }
}
impl DebugDrawing {
  /// Logs the debug information to the console for the given Drawing variant.
  #[allow(unused_variables)]
  pub fn log(self) {
    match self {
      DebugDrawing::Gradient(gradient, path, duration) => {
        debug_println!("    Drawing::Gradient: gradient={{{}}}; path={{{}}}; time={:?}", gradient, path, duration)
      }
    }
  }
}
