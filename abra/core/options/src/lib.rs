mod apply_options;

pub use abra_core::image::gpu::Hardware;
pub use apply_options::Apply;
pub use apply_options::ApplyTarget;
pub use apply_options::ApplyOptions;
pub use apply_options::Options;
pub use apply_options::get_ctx;

#[doc(hidden)]
pub use abra_core::{Image as __Image, image::gpu::CpuProcessor as __CpuProcessor};

/// Makes an effect that only runs on the CPU usable everywhere effects are used, such as in a live image's chain.
///
/// The effect's [`Apply::apply_to_image`] holds its CPU implementation, with its area and mask handling, and this
/// makes [`CpuProcessor::process`](abra_core::image::gpu::CpuProcessor::process) call it. An effect that also has a
/// shader implements `CpuProcessor` and `GpuProcessor` by hand instead, to return its shader from `gpu()`.
///
/// ```ignore
/// options::cpu_processor!(Sharpen);
/// ```
#[macro_export]
macro_rules! cpu_processor {
  ($effect:ty) => {
    impl $crate::__CpuProcessor for $effect {
      fn process(&self, p_image: &mut $crate::__Image) {
        $crate::Apply::apply_to_image(self, p_image);
      }
    }
  };
}
