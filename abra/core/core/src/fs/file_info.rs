use crate::Channels;

#[derive(Clone)]
/// Contains the image data and metadata from a file
pub struct FileInfo {
  /// The width of the source image.
  pub width: u32,
  /// The height of the source image.
  pub height: u32,
  /// The number of channels in the source image.
  #[allow(dead_code)]
  pub channels: Channels,
  /// The pixel data of the source image.
  pub pixels: Vec<u8>,
}
impl FileInfo {
  /// Creates a new FileInfo with the given dimensions, channels, and pixel data
  pub fn new(p_width: u32, p_height: u32, p_channels: Channels, p_pixels: Vec<u8>) -> FileInfo {
    FileInfo {
      width: p_width,
      height: p_height,
      channels: p_channels,
      pixels: p_pixels,
    }
  }
}
