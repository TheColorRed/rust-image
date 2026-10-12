use crate::Image;
use crate::fs::writer_options::WriterOptions;
use imgref::Img;
use rgb::RGBA8;

/// Writes the image data to an AVIF file.
///
/// With `p_options`, the image is encoded at its `quality` (0 to 100). Without options it is encoded at the highest
/// quality, which is close to lossless but still much smaller than PNG.
pub fn write_avif(p_file: impl Into<String>, p_img: &Image, p_options: &Option<WriterOptions>) -> Result<(), String> {
  let (width, height) = p_img.dimensions::<usize>();
  let pixels: Vec<RGBA8> =
    p_img.rgba().chunks_exact(4).map(|pixel| RGBA8::new(pixel[0], pixel[1], pixel[2], pixel[3])).collect();
  let quality = p_options.as_ref().map_or(100, |options| options.quality.min(100));

  let encoded = ravif::Encoder::new()
    .with_quality(quality as f32)
    .with_alpha_quality(quality as f32)
    .with_speed(6)
    .encode_rgba(Img::new(pixels.as_slice(), width, height))
    .map_err(|e| format!("AVIF encoding failed: {e}"))?;
  std::fs::write(p_file.into(), encoded.avif_file).map_err(|e| e.to_string())
}
