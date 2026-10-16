mod blur;
mod r#box;
mod focus;
mod gaussian;
mod lens;
mod motion;
mod surface;

pub use blur::{Blur, blur};
pub use r#box::{BoxBlur, box_blur};
pub use focus::{BlurType, FocusBlur, FocusGeometry, FocusShape, focus_blur};
pub use gaussian::{GaussianBlur, gaussian_blur};
pub use lens::{ApertureShape, LensBlur, lens_blur};
pub use motion::{MotionBlur, motion_blur};
pub use surface::{SurfaceBlur, surface_blur};

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::image::gpu::GpuProcessor;

  /// Every blur with a GPU implementation creates a pipeline and dispatches successfully.
  #[test]
  fn gpu_blur_shaders_compile_and_dispatch() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};

    let (width, height) = (17, 13);
    let pixels: Vec<u8> = (0..width * height).flat_map(|pixel| [(pixel * 13) as u8, (pixel * 7) as u8, pixel as u8, 255]).collect();
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let effects: Vec<Box<dyn GpuProcessor>> = vec![
      Box::new(blur()),
      Box::new(box_blur(3)),
      Box::new(motion_blur(27.0, 7)),
      Box::new(lens_blur(4).with_samples(8)),
      Box::new(focus_blur().with_blur(gaussian_blur(3))),
    ];
    for effect in effects {
      let result = renderer.process(&[effect.passes(width, height)], width, height, &pixels)?;
      assert_eq!(result.len(), pixels.len());
    }
    Ok(())
  }
}
