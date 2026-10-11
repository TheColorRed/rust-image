//! An effect that has a shader should give the same pixels on the GPU as on the CPU. These run each effect both ways
//! with `apply_on_cpu` and `apply_on_gpu` and compare. They need the `gpu` feature and a GPU; without a GPU they pass
//! without checking anything.
#![cfg(feature = "gpu")]

use abra as _; // links the library, whose start-up code registers the GPU
use abra_core::image::gpu::Hardware;
use abra_core::{Channels, Image};
use options::{Effect, ApplyOptions};

const SIZE: u32 = 256;

/// Every red and green pair appears, with blue spread across them.
fn picture() -> Image {
  let pixels: Vec<u8> =
    (0..SIZE * SIZE).flat_map(|i| [(i % SIZE) as u8, (i / SIZE) as u8, ((i % SIZE) * 3 + (i / SIZE) * 5) as u8, 255]).collect();
  Image::new_from_pixels(SIZE, SIZE, pixels, Channels::RGBA)
}

/// The largest difference in any channel between the CPU and GPU result, or `None` when there is no GPU.
fn worst_difference<E: Effect>(p_effect: &E) -> Option<u8> {
  let mut on_cpu = picture();
  p_effect.apply_on_cpu(&mut on_cpu);
  let mut on_gpu = picture();
  p_effect.apply_on_gpu(&mut on_gpu).ok()?;
  Some(on_cpu.to_rgba_vec().iter().zip(on_gpu.to_rgba_vec()).map(|(a, b)| a.abs_diff(b)).max().unwrap_or(0))
}

/// Fails when the CPU and GPU differ by more than `p_allowed` levels.
fn agree<E: Effect>(p_name: &str, p_effect: E, p_allowed: u8) {
  if let Some(worst) = worst_difference(&p_effect) {
    assert!(worst <= p_allowed, "{p_name}: the GPU differs from the CPU by up to {worst} levels (allowed {p_allowed})");
  }
}

#[test]
fn saturation_is_exactly_the_same_on_the_gpu() {
  for amount in [-100, -75, -60, -25, -1, 0, 1, 25, 40, 60, 75, 99, 100] {
    agree(&format!("saturation {amount}"), adjustments::levels::saturation(amount), 0);
  }
}

/// The lens-filter tint multiplies in linear light with a `pow`, which the CPU and GPU round differently now and then,
/// so a pixel can be one level apart.
#[test]
fn photo_filter_is_within_a_level_of_the_cpu_on_the_gpu() {
  use adjustments::levels::{FilterType, PhotoFilter, photo_filter};
  let presets = [
    FilterType::WarmingLight,
    FilterType::CoolingLight,
    FilterType::Sepia,
    FilterType::Underwater,
    FilterType::DeepEmerald,
    FilterType::CoolingDark,
    FilterType::Magenta,
  ];
  for preset in presets {
    for density in [0.0, 0.2, 0.35, 0.45, 1.0] {
      agree("photo filter preset", photo_filter(PhotoFilter::Preset(preset)).with_density(density), 1);
    }
  }
  // A plain color tinting like a lens filter, which is how the editor's moods use it.
  let plain = photo_filter(abra_core::Color::from_rgb(235, 177, 19)).with_density(0.35).with_preserve_luminosity(false);
  agree("photo filter plain color", plain, 1);
}

/// The mode that keeps the photo's brightness works in hue and saturation and has no shader, so it must not claim one.
#[test]
fn photo_filter_that_preserves_luminosity_stays_on_the_cpu() {
  use adjustments::levels::photo_filter;
  let preserving = photo_filter(abra_core::Color::from_rgb(235, 177, 19)).with_density(0.35);
  assert!(preserving.gpu_processor().is_none());
  let lens = photo_filter(abra_core::Color::from_rgb(235, 177, 19)).with_preserve_luminosity(false);
  assert!(lens.gpu_processor().is_some());
}

/// The shader does the same whole-number steps as the CPU, so the pixels are exactly the same.
#[test]
fn surface_blur_is_exactly_the_same_on_the_gpu() {
  for (radius, threshold) in [(1, 10), (2, 30), (5, 30), (8, 24), (10, 30), (3, 0), (4, 255)] {
    agree(&format!("surface blur {radius} {threshold}"), filters::blur::surface_blur(radius, threshold), 0);
  }
}

/// With a step the blur looks at pixels spaced apart, so it reaches further for the same cost. The shader does the same.
#[test]
fn surface_blur_with_a_step_is_exactly_the_same_on_the_gpu() {
  for (radius, threshold, step) in [(4, 30, 2), (8, 60, 3), (10, 30, 5)] {
    agree(
      &format!("surface blur {radius} {threshold} step {step}"),
      filters::blur::surface_blur(radius, threshold).with_step(step),
      0,
    );
  }
}

/// A radius the CPU speeds up by blurring a half-size copy has no shader, so it must not claim one.
#[test]
fn surface_blur_with_a_huge_radius_stays_on_the_cpu() {
  assert!(filters::blur::surface_blur(50, 30).gpu_processor().is_some());
  assert!(filters::blur::surface_blur(51, 30).gpu_processor().is_none());
}

#[test]
fn grayscale_is_exactly_the_same_on_the_gpu() {
  agree("grayscale", adjustments::color::grayscale(), 0);
}

#[test]
fn brightness_contrast_and_exposure_are_exactly_the_same_on_the_gpu() {
  for amount in [-100, -45, -1, 0, 1, 30, 100, 250] {
    agree(&format!("brightness {amount}"), adjustments::levels::brightness(amount), 0);
  }
  for amount in [-100, -40, 0, 40, 100] {
    agree(&format!("contrast {amount}"), adjustments::levels::contrast(amount), 0);
  }
  for exposure in [-3.0, -0.5, 0.0, 1.0, 4.0] {
    agree("exposure", adjustments::levels::exposure(exposure).with_offset(0.05).with_gamma(1.3), 0);
  }
}

/// The CPU downsamples a radius of 24 or more on an image 128 or more pixels on a side, for speed, and the GPU does
/// not, so those give slightly different pixels. Below that they are exactly the same.
#[test]
fn gaussian_blur_is_exactly_the_same_on_the_gpu() {
  for radius in [1, 2, 3, 7, 12, 23] {
    agree(&format!("gaussian blur {radius}"), filters::blur::gaussian_blur(radius), 0);
  }
}

#[test]
fn linear_gradient_is_exactly_the_same_on_the_gpu() {
  use abra_core::{Color, ColorStop, Gradient};
  let gradient = Gradient::new(vec![
    ColorStop::new(Color::from_rgba(255, 0, 0, 255), 0.0),
    ColorStop::new(Color::from_rgba(0, 200, 90, 180), 0.45),
    ColorStop::new(Color::from_rgba(20, 30, 255, 255), 1.0),
  ]);
  // A line between two points, a very short one, and one that ends past the image.
  for (start, end) in [((10.0, 20.0), (240.0, 200.0)), ((100.0, 100.0), (101.0, 100.0)), ((-50.0, 0.0), (900.0, 40.0))] {
    agree(
      &format!("linear gradient from {start:?} to {end:?}"),
      adjustments::color::LinearGradientEffect::between(&gradient, start, end).with_opacity(0.7),
      0,
    );
  }
  for (angle, opacity) in [(0.0, 1.0), (33.0, 0.5), (90.0, 0.8), (135.0, 1.0), (271.0, 0.25)] {
    agree(
      &format!("linear gradient {angle} degrees at {opacity}"),
      adjustments::color::LinearGradientEffect::angle(&gradient, angle).with_opacity(opacity),
      0,
    );
  }
}

/// A positioned effect limited to an area runs over the whole image on the GPU too, so its coordinates do not move.
#[test]
fn a_positioned_effect_limited_to_an_area_is_the_same_on_the_gpu() {
  use abra_core::{Area, Color, Gradient};
  let gradient = Gradient::from_to(Color::from_rgba(255, 0, 0, 255), Color::from_rgba(0, 0, 255, 255));
  let area = || ApplyOptions::new().with_area(Area::rect((100.0, 40.0), (120.0, 90.0)).with_feather(6));
  agree(
    "linear gradient in an area",
    adjustments::color::LinearGradientEffect::between(&gradient, (0.0, 0.0), (SIZE as f32, 0.0)).with_options(area()),
    0,
  );
}

#[test]
fn the_gpu_only_entry_refuses_an_effect_that_has_no_shader() {
  let mut image = picture();
  let untouched = image.to_rgba_vec();
  assert!(adjustments::color::invert().apply_on_gpu(&mut image).is_err());
  assert_eq!(image.to_rgba_vec(), untouched, "a refused effect leaves the image alone");
}

#[test]
fn the_router_follows_the_hardware_setting() {
  // Asking for the CPU never uses the GPU, so the router gives exactly what `apply_on_cpu` does.
  let cpu = || adjustments::levels::brightness(30).with_options(ApplyOptions::new().with_hardware(Hardware::Cpu));
  let mut routed = picture();
  cpu().apply(&mut routed);
  let mut direct = picture();
  cpu().apply_on_cpu(&mut direct);
  assert_eq!(routed.to_rgba_vec(), direct.to_rgba_vec());
}
