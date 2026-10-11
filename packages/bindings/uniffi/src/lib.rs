//! The Abra library for other languages, over uniffi.
//!
//! The library crates know nothing about bindings. This crate gives each library type that a tool needs a handle with the
//! same name and the same functions, so a TypeScript tool reads like the Rust one: `Color.red()`, `new Gradient([...])`,
//! `blend(edges).apply(photo)`. Each handle owns the library's value and forwards to it; the work is the library's. A handle
//! is needed because Rust only lets a type's own crate add functions to it.
//!
//! One file for each part of the library: [`color`] (the `primitives` color module), [`image`] (`Image`), [`gradient`]
//! (`abra-core`'s gradients and the radial gradient) and [`blend`] (blending), [`mask`] (masks) and [`geometry`] (points, paths, areas and shapes). The top of each file says what in its part of
//! the library cannot be exported, and why. A tuple of the library is an array here, and an
//! array of the wrong length panics, as the library does for input it cannot use.
//!
//! uniffi turns this crate into one namespace (`abra_uniffi`) of the generated TypeScript bindings. A crate that builds the
//! native library depends on this one and shares its types.

mod adjustments;
mod blend;
mod color;
mod effect;
mod filters;
mod geometry;
mod gradient;
mod image;
mod mask;

pub use adjustments::{
  AutoColor, AutoTone, Brightness, ColorMultiply, Contrast, Exposure, FilterType, GradientMap, Grayscale,
  HighlightsShadows, Hue, Invert, LinearGradientEffect, Opacity, PhotoFilter, PhotoFilterAdjustment, Posterize,
  Saturation, Threshold, Vibrance, auto_color, auto_tone, brightness, color_multiply, contrast, exposure, gradient_map,
  grayscale, highlights_shadows, hue, invert, photo_filter, posterize, reduce_opacity, saturation, threshold, vibrance,
};
pub use blend::{BlendImage, BlendMode, blend, blend_mode_from_name, blend_modes, blended};
pub use color::{Bins, Color, Harmony, Histogram, luma, luma_weights};
pub use filters::{
  Blur, BlurType, BoxBlur, Despeckle, FocusBlur, FocusGeometry, GaussianBlur, GlowingEdges, LensBlur, Median,
  MotionBlur, Noise, Pinch, Ripple, Sharpen, Smooth, Sobel, SurfaceBlur, blur, box_blur, despeckle, focus_blur,
  gaussian_blur, glowing_edges, lens_blur, median, motion_blur, noise, pinch, ripple, sharpen, smooth, sobel,
  sobel_magnitude, surface_blur,
};
pub use geometry::{
  Area, AreaStroke, Homography, LineSegment, Path, PathStroke, Shape, aspect_ratio_meet, aspect_ratio_new,
  aspect_ratio_none, aspect_ratio_slice, bresenham, homography_from_points, homography_from_quads, point_f_add,
  point_f_angle, point_f_cross, point_f_distance_to, point_f_div, point_f_dot, point_f_length, point_f_length_squared,
  point_f_lerp, point_f_mul, point_f_new, point_f_normalize, point_f_perpendicular, point_f_sub, point_f_zero,
  polygon_contains, quad_bounds, quad_corners, quad_flattened_aspect, quad_flattened_size, quad_from_rect, quad_new,
  quad_rect, rect_bottom, rect_center, rect_contains, rect_corners, rect_edges, rect_from_edges, rect_from_points,
  rect_from_size, rect_intersect, rect_is_empty, rect_left, rect_map_point, rect_new, rect_origin, rect_right,
  rect_size, rect_top, segment_end, size_add, size_add_scalar, size_aspect_ratio, size_div,
  size_inscribed_after_rotation, size_mul, size_new, size_rotated_bounds, size_sub, size_sub_scalar, size_to_tuple,
};
pub use gradient::{ColorStop, Gradient, radial_gradient};
pub use image::{Image, channel_index, channels_bytes_per_pixel, resolution_new, resolution_uniform};
pub use mask::{Mask, apply_mask_to_image, apply_mask_to_pixels_rgba, mask_value_to_alpha, rgba_to_gray};

uniffi::setup_scaffolding!();
