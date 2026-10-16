mod add_noise;
mod despeckle;
mod median;

pub use add_noise::{Noise, NoiseDistribution, noise};
pub use despeckle::{Despeckle, despeckle};
pub use median::{Median, median};

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::image::gpu::GpuProcessor;

  #[test]
  fn gpu_noise_shaders_compile_and_dispatch() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};

    let pixels: Vec<u8> = (0..17 * 13).flat_map(|_| [64, 96, 128, 255]).collect();
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    for effect in [noise(8.0), noise(8.0).with_distribution(NoiseDistribution::Gaussian)] {
      let result = renderer.process(&[effect.passes(17, 13)], 17, 13, &pixels)?;
      assert_eq!(result.len(), pixels.len());
      assert!(result.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }
    Ok(())
  }
}
