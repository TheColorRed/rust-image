use crate::{transform::ResizeAlgorithm, Image, Layer};

use std::time::Duration;

pub enum RotateMessage {
  String(String),
  Degrees(f32),
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
  /// - `degrees`: The degrees of the rotation
  /// - `old_width`: Width of the image before rotation
  /// - `old_height`: Height of the image before rotation
  /// - `new_width`: Width of the image after rotation
  /// - `new_height`: Height of the image after rotation
  /// - `duration`: Time taken to perform the rotation
  Rotate(f32, u32, u32, u32, u32, Duration),
}

impl DebugInfo {
  /// Logs the debug information to the console for the given PrintInfo variant.
  pub fn log(self) {
    match self {
      // Primary outputs; these do not contain indentation
      DebugInfo::ImageOpened(file_path, width, height, duration) => println!(
        "Image Opened: filename={}; dimensions={}x{}; time={:?}",
        file_path, width, height, duration
      ),
      DebugInfo::ImageSaved(file_path, width, height, duration) => println!(
        "Image Saved: filename={}; dimensions={}x{}; time={:?}",
        file_path, width, height, duration
      ),
      // Secondary outputs; these contain indentation
      // #region: Transform Outputs
      DebugInfo::Resize(algorithm, old_width, old_height, new_width, new_height, duration) => println!(
        "    Transform::Resize: algorithm={} original={}x{}; new={}x{}; time={:?}",
        algorithm, old_width, old_height, new_width, new_height, duration
      ),
      DebugInfo::Crop(old_width, old_height, new_width, new_height, duration) => println!(
        "    Transform::Crop: original={}x{}; new={}x{}; time={:?}",
        old_width, old_height, new_width, new_height, duration
      ),
      DebugInfo::Flip(direction, width, height, duration) => println!(
        "    Transform::Flip: direction={}; dimensions={}x{}; time={:?}",
        direction, width, height, duration
      ),
      DebugInfo::Rotate(degrees, old_width, old_height, new_width, new_height, duration) => println!(
        "    Transform::Rotate: degrees={}; original={}x{}; new={}x{}; time={:?}",
        degrees, old_width, old_height, new_width, new_height, duration
      ),
      // #endregion: Transform Outputs
    }
  }
}
