//! Filters module contains all the filters that can be applied to an image.

pub mod blur;
pub mod edges;
pub mod noise;
pub mod sharpen;
pub mod smooth;
pub mod sobel;

mod kernel;

/// A macro to apply a filter. This will apply the given function to the specified area of the image,
/// or the entire image if no area is specified via `None` within the `ApplyOptions` object.
/// - `$func`: The primary function to apply to the image within the specified area.
///   If the area is not provided, the function is applied to the entire image.
/// - `$image`: The image to which the filter is applied.
/// - `$apply_opts`: Options that specify the area and mask.
/// - `$kernel_padding`: The padding around the kernel.
/// - `..$rest`: Additional arguments to pass `$func`.
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
macro_rules! apply_filter {
  ($func:ident, $image:ident, $apply_opts:ident, $kernel_padding:expr $(, $rest:expr )* ) => {
    let options = $apply_opts.into();
    let ctx = options::get_ctx(options.as_ref());
    abra_core::image::apply_area::process_image($image, ctx, $kernel_padding, |img| {
      $func(img $(, $rest )*);
    });
  };
}
