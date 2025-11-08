use crate::Channels;

#[derive(Clone)]
/// Contains the image data and metadata from a file
pub struct FileInfo {
  /// The width of the source image.
  pub width: u32,
  /// The height of the source image.
  pub height: u32,
  /// The number of channels in the source image.
  pub channels: Channels,
  /// The pixel data of the source image.
  pub pixels: Vec<u8>,
}
impl FileInfo {
  /// Creates a new FileInfo with the given dimensions, channels, and pixel data
  pub fn new(width: u32, height: u32, channels: Channels, pixels: Vec<u8>) -> FileInfo {
    FileInfo {
      width,
      height,
      channels,
      pixels,
    }
  }
}
