mod auto_color;
mod auto_tone;
mod black_white;
mod channel_mix;
mod gradient_map;
mod grayscale;
mod invert;
mod linear_gradient;
mod multiply;
mod opacity;
mod posterize;
mod quantize;
mod radial_gradient;
mod selective_color;
mod threshold;

pub use auto_color::{AutoColor, auto_color};
pub use auto_tone::{AutoTone, auto_tone};
pub use black_white::{BlackWhite, BlackWhitePreset, black_white};
pub use channel_mix::{ChannelMix, channel_mix};
pub use gradient_map::{GradientMap, gradient_map};
pub use grayscale::{Grayscale, grayscale};
pub use invert::{Invert, invert};
pub use linear_gradient::LinearGradientEffect;
pub use multiply::{ColorMultiply, color_multiply};
pub use opacity::{Opacity, reduce_opacity};
pub use posterize::{Posterize, posterize};
pub use quantize::{Quantize, quantize};
pub use radial_gradient::{radial_gradient, radial_gradient_deferred, vignette};
pub use selective_color::{ColorRange, SelectiveColor, SelectiveMethod, selective_color};
pub use threshold::{Threshold, threshold};

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Channels, Image, image::gpu::GpuProcessor};
  use options::Effect;

  #[test]
  fn gpu_opacity_and_posterize_match_the_cpu() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};

    let (width, height) = (31, 19);
    let pixels: Vec<u8> = (0..width * height).flat_map(|pixel| [(pixel * 31) as u8, (pixel * 17) as u8, (pixel * 7) as u8, pixel as u8]).collect();
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    for effect in [reduce_opacity(0.43), reduce_opacity(1.8), reduce_opacity(-0.2)] {
      let gpu = renderer.process(&[effect.passes(width, height)], width, height, &pixels)?;
      let mut cpu = Image::new_from_pixels(width, height, pixels.clone(), Channels::RGBA);
      effect.apply_on_cpu(&mut cpu);
      assert_eq!(gpu, cpu.to_rgba_vec());
    }
    for effect in [posterize(0), posterize(2), posterize(4), posterize(17), posterize(255)] {
      let gpu = renderer.process(&[effect.passes(width, height)], width, height, &pixels)?;
      let mut cpu = Image::new_from_pixels(width, height, pixels.clone(), Channels::RGBA);
      effect.apply_on_cpu(&mut cpu);
      assert_eq!(gpu, cpu.to_rgba_vec());
    }
    Ok(())
  }
}
