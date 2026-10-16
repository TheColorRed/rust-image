mod sharpen;

pub use sharpen::{Sharpen, sharpen};

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Channels, Image, image::gpu::GpuProcessor};
  use options::Effect;

  #[test]
  fn gpu_sharpen_matches_the_cpu_and_compiles_its_shader() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};

    let (width, height) = (17, 13);
    let pixels: Vec<u8> = (0..width * height).flat_map(|pixel| [(pixel * 13) as u8, (pixel * 7) as u8, pixel as u8, 255]).collect();
    let mut expected = Image::new_from_pixels(width, height, pixels.clone(), Channels::RGBA);
    sharpen().apply_on_cpu(&mut expected);
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let actual = renderer.process(&[sharpen().passes(width, height)], width, height, &pixels)?;
    assert_eq!(actual, expected.to_rgba_vec());
    Ok(())
  }
}
