pub mod levels;
pub use levels::FilterType;
pub use options::Apply;

/// Adjustments that affect an image's color.
pub mod color;

/// A macro to apply a filter. This will apply the given function to the specified area of the image,
/// or the entire image if no area is specified via `None` within the `ApplyOptions` object.
/// - `$func`: The primary function to apply to the image within the specified area.
///   If the area is not provided, the function is applied to the entire image.
/// - `$image`: The image to which the filter is applied.
/// - `$apply_opts`: Options that specify the area and mask.
/// - `$kernel_padding`: The padding around the kernel.
/// - `..$rest`: Additional arguments to pass `$func`.
///
/// Start with `gpu = <&impl GpuEffect>;` to give the adjustment a GPU version, which runs instead of `$cpu_func` when the
/// GPU is enabled and available.
///
/// ## Example
///
/// ```ignore
/// use abra_core::Image;
/// use options::Options;
/// use crate::apply_filter;
///
/// fn apply_example_filter(image: &mut Image, intensity: u32) {
///     // filter logic here
/// }
///
/// pub fn example_filter(image: &mut Image, intensity: u32, apply_options: impl Into<Options>) {
///     apply_filter!(apply_example_filter, image, apply_options, 1, intensity);
/// }
/// ```
#[macro_export]
macro_rules! apply_adjustment {
  (gpu = $gpu:expr; $cpu_func:ident, $image:ident, $apply_opts:expr, $kernel_padding:expr $(, $rest:expr )* ) => {
    let ctx = options::get_ctx($apply_opts);
    abra_core::image::apply_area::apply_in_area($image, ctx, $kernel_padding, Some($gpu as &dyn abra_core::image::gpu::GpuEffect), |img| {
      $cpu_func(img $(, $rest )*);
    });
  };
  ($cpu_func:ident, $image:ident, $apply_opts:expr, $kernel_padding:expr $(, $rest:expr )* ) => {
    let ctx = options::get_ctx($apply_opts);
    abra_core::image::apply_area::apply_in_area($image, ctx, $kernel_padding, None, |img| {
      $cpu_func(img $(, $rest )*);
    });
  };
}
