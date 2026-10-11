use crate::{Channels, Image};
use avif_decode::Image as Decoded;

/// Reads an AVIF file and returns the image data. Deeper than 8 bit images are scaled down to 8 bit.
pub fn read_avif(p_file: impl Into<String>) -> Result<Image, String> {
  let bytes = std::fs::read(p_file.into()).map_err(|e| format!("Failed to open file: {e}"))?;
  let decoded = avif_decode::Decoder::from_avif(&bytes)
    .and_then(|decoder| decoder.to_image())
    .map_err(|e| format!("Failed to decode AVIF image: {e}"))?;

  let (width, height, pixels, channels): (usize, usize, Vec<u8>, Channels) = match decoded {
    Decoded::Rgb8(img) => {
      (img.width(), img.height(), img.pixels().flat_map(|p| [p.r, p.g, p.b]).collect(), Channels::RGB)
    }
    Decoded::Rgb16(img) => (
      img.width(),
      img.height(),
      img.pixels().flat_map(|p| [(p.r >> 8) as u8, (p.g >> 8) as u8, (p.b >> 8) as u8]).collect(),
      Channels::RGB,
    ),
    Decoded::Rgba8(img) => {
      (img.width(), img.height(), img.pixels().flat_map(|p| [p.r, p.g, p.b, p.a]).collect(), Channels::RGBA)
    }
    Decoded::Rgba16(img) => (
      img.width(),
      img.height(),
      img.pixels().flat_map(|p| [(p.r >> 8) as u8, (p.g >> 8) as u8, (p.b >> 8) as u8, (p.a >> 8) as u8]).collect(),
      Channels::RGBA,
    ),
    Decoded::Gray8(img) => {
      (img.width(), img.height(), img.pixels().flat_map(|p| [p.value(); 3]).collect(), Channels::RGB)
    }
    Decoded::Gray16(img) => {
      (img.width(), img.height(), img.pixels().flat_map(|p| [(p.value() >> 8) as u8; 3]).collect(), Channels::RGB)
    }
  };

  Ok(Image::new_from_pixels(width as u32, height as u32, pixels, channels))
}
