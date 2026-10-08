use abra_core::{Channels, Gradient, Image};
use rayon::prelude::*;

use abra_core::image::gpu::{GpuAux, GpuPass};
use abra_core::image::recipe::{Generator, generated};
use std::sync::Arc;

/// Number of colors sampled from the gradient into the lookup table.
const LUT_SIZE: u32 = 1024;

/// Samples the gradient into `LUT_SIZE` RGBA texels, so a pixel needs only a lookup.
fn build_lut(p_gradient: &Gradient) -> Vec<u8> {
  (0..LUT_SIZE)
    .flat_map(|index| {
      let (r, g, b, a) = p_gradient.color_at(index as f32 / (LUT_SIZE - 1) as f32).rgba();
      [r, g, b, a]
    })
    .collect()
}

/// A radial gradient that is made when it is needed: on the CPU by [`render`](Generator::render), or by a shader.
#[derive(Debug)]
struct RadialGradient {
  lut: Arc<[u8]>,
  center: (f32, f32),
  radius: (f32, f32),
}

impl RadialGradient {
  /// The inverse of the radius. A radius of zero would divide by zero; every pixel then takes the last stop, as it is beyond
  /// the ellipse.
  fn inverse(&self) -> (f32, f32) {
    (1.0 / self.radius.0.max(f32::MIN_POSITIVE), 1.0 / self.radius.1.max(f32::MIN_POSITIVE))
  }
}

impl Generator for RadialGradient {
  fn render(&self, p_width: u32, p_height: u32) -> Image {
    let inverse = self.inverse();
    let mut pixels = vec![0u8; p_width as usize * p_height as usize * 4];
    if !pixels.is_empty() {
      pixels.par_chunks_exact_mut(p_width as usize * 4).enumerate().for_each(|(y, row)| {
        let dy = (y as f32 + 0.5 - self.center.1) * inverse.1;
        for (x, pixel) in row.chunks_exact_mut(4).enumerate() {
          let dx = (x as f32 + 0.5 - self.center.0) * inverse.0;
          let position = (dx * dx + dy * dy).sqrt().min(1.0);
          let index = (position * (LUT_SIZE - 1) as f32).round() as usize * 4;
          pixel.copy_from_slice(&self.lut[index..index + 4]);
        }
      });
    }
    Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA)
  }

  fn passes(&self, _p_width: u32, _p_height: u32) -> Option<Vec<GpuPass>> {
    let inverse = self.inverse();
    let uniforms: Vec<u8> =
      [self.center.0, self.center.1, inverse.0, inverse.1].into_iter().flat_map(f32::to_le_bytes).collect();
    Some(vec![GpuPass::new(include_str!("./radial_gradient.wgsl"), uniforms).with_aux(GpuAux {
      width: LUT_SIZE,
      height: 1,
      rgba: self.lut.clone(),
    })])
  }
}

/// An image of `p_width` x `p_height` pixels holding a radial gradient.
///
/// The first stop of `p_gradient` is at `p_center` and the last stop is on the ellipse around it with half-axes
/// `p_radius`, so a different x and y radius gives an oval. Pixels beyond the ellipse take the last stop. Colors keep their
/// own alpha, so a gradient from transparent to black is a vignette that can be placed over a photo.
///
/// `p_center` and `p_radius` are in pixels of the image.
///
/// The pixels are made on the CPU now. [`radial_gradient_deferred`] leaves them to the GPU when there is one.
pub fn radial_gradient(
  p_width: u32, p_height: u32, p_gradient: &Gradient, p_center: (f32, f32), p_radius: (f32, f32),
) -> Image {
  RadialGradient {
    lut: build_lut(p_gradient).into(),
    center: p_center,
    radius: p_radius,
  }
  .render(p_width, p_height)
}

/// [`radial_gradient`] as an image that is made by the GPU when its pixels are first read (see `abra_core::image::recipe`).
/// Without a GPU it is made now, with the same pixels either way.
pub fn radial_gradient_deferred(
  p_width: u32, p_height: u32, p_gradient: &Gradient, p_center: (f32, f32), p_radius: (f32, f32),
) -> Image {
  generated(
    p_width,
    p_height,
    Arc::new(RadialGradient {
      lut: build_lut(p_gradient).into(),
      center: p_center,
      radius: p_radius,
    }),
  )
}

/// [`radial_gradient`] as other languages call it, which hold the gradient as a handle and return the image as one.
#[cfg(feature = "uniffi")]
#[uniffi::export(name = "radial_gradient")]
pub fn radial_gradient_image(
  p_width: u32, p_height: u32, p_gradient: std::sync::Arc<Gradient>, p_center_x: f32, p_center_y: f32, p_radius_x: f32,
  p_radius_y: f32,
) -> std::sync::Arc<Image> {
  std::sync::Arc::new(radial_gradient_deferred(
    p_width,
    p_height,
    &p_gradient,
    (p_center_x, p_center_y),
    (p_radius_x, p_radius_y),
  ))
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Color;

  fn pixel(p_image: &Image, p_x: usize, p_y: usize) -> [u8; 4] {
    let width = p_image.dimensions::<u32>().0 as usize;
    let rgba = p_image.rgba();
    let start = (p_y * width + p_x) * 4;
    [rgba[start], rgba[start + 1], rgba[start + 2], rgba[start + 3]]
  }

  #[test]
  fn runs_from_the_first_color_at_the_center_to_the_last_at_the_edge() {
    let gradient = Gradient::from_to(Color::from_rgba(255, 0, 0, 255), Color::from_rgba(0, 0, 255, 255));
    let image = radial_gradient(101, 101, &gradient, (50.5, 50.5), (50.0, 50.0));
    assert_eq!(pixel(&image, 50, 50), [255, 0, 0, 255]);
    // Beyond the radius, everything is the last stop.
    assert_eq!(pixel(&image, 0, 0), [0, 0, 255, 255]);
    let halfway = pixel(&image, 75, 50);
    assert!((halfway[0] as i32 - 128).abs() <= 3 && (halfway[2] as i32 - 128).abs() <= 3, "{halfway:?}");
  }

  /// Registers the library's real GPU provider wrapped in one that counts its runs, and returns the counter (reset to zero).
  /// A shader that fails to compile makes a run fail and the picture fall back to the CPU, which would match the CPU
  /// perfectly, so the tests check the count to know the GPU really made the pixels. Hold `GPU_PROVIDER_LOCK`.
  fn counted_gpu() -> &'static std::sync::atomic::AtomicUsize {
    use abra_core::image::gpu::{GpuProvider, get_gpu_provider, register_gpu_provider};
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static REAL: OnceLock<GpuProvider> = OnceLock::new();
    static RUNS: AtomicUsize = AtomicUsize::new(0);

    gpu::register();
    let real = *REAL.get_or_init(|| get_gpu_provider().expect("the gpu crate registers a provider"));
    register_gpu_provider(GpuProvider {
      process: |effects, width, height, pixels| {
        RUNS.fetch_add(1, Ordering::SeqCst);
        (REAL.get().unwrap().process)(effects, width, height, pixels)
      },
      ..real
    });
    RUNS.store(0, Ordering::SeqCst);
    &RUNS
  }

  /// The whole GPU path through the library, on a real GPU: the same calls a tool makes, compared with the CPU's pixels. The
  /// real provider is wrapped in one that counts, so the test fails if the work quietly fell back to the CPU.
  #[test]
  fn the_gpu_gives_the_cpu_s_pixels_for_a_gradient_over_a_photo_and_a_picture_over_a_photo() -> anyhow::Result<()> {
    use abra_core::BlendMode;
    use abra_core::image::gpu::{GPU_PROVIDER_LOCK, clear_gpu_provider};
    use abra_core::image::recipe::blended;
    use std::sync::atomic::Ordering;

    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    // Fails here, loudly, on a machine with no GPU adapter.
    gpu::GpuContext::new_default_blocking()?;
    let gpu_runs = counted_gpu();

    let (width, height) = (61, 37);
    let mut photo_pixels = Vec::new();
    for y in 0..height {
      for x in 0..width {
        photo_pixels.extend_from_slice(&[(x * 4) as u8, (y * 6) as u8, 90, if x % 7 == 0 { 140 } else { 255 }]);
      }
    }
    let photo = Image::new_from_pixels(width, height, photo_pixels, Channels::RGBA);
    let gradient = Gradient::new(vec![
      abra_core::ColorStop::new(Color::transparent(), 0.0),
      abra_core::ColorStop::new(Color::transparent(), 0.5),
      abra_core::ColorStop::new(Color::from_rgba(10, 20, 30, 220), 1.0),
    ]);
    let (center, radius) = ((30.5, 18.5), (43.0, 26.0));

    // What the CPU makes: the gradient image, then a normal blend.
    for opacity in [1.0f32, 0.6] {
      let mut expected = photo.clone();
      abra_core::blend::blend(&radial_gradient(width, height, &gradient, center, radius))
        .with_opacity(opacity)
        .apply(&mut expected);

      let edges = radial_gradient_deferred(width, height, &gradient, center, radius);
      let made = blended(&photo, &edges, BlendMode::Normal, opacity);
      assert!(made.deferred().is_some(), "with a GPU the result is not made until it is read");
      let (actual, expected) = (made.rgba(), expected.rgba());
      let worst = actual.iter().zip(expected).map(|(a, e)| a.abs_diff(*e)).max().unwrap_or(0);
      assert!(worst <= 2, "opacity {opacity}: the GPU differs from the CPU by {worst}");
    }

    // The gradient is a picture like any other to the blend, so every mode works with it. Each is the gradient's own run and
    // the blend's.
    let runs_before = gpu_runs.load(Ordering::SeqCst);
    let modes = [
      BlendMode::Multiply,
      BlendMode::Overlay,
      BlendMode::SoftLight,
      BlendMode::Screen,
      BlendMode::Difference,
      BlendMode::Hue,
    ];
    for mode in modes {
      let mut expected = photo.clone();
      abra_core::blend::blend(&radial_gradient(width, height, &gradient, center, radius))
        .with_mode(mode)
        .with_opacity(0.8)
        .apply(&mut expected);
      let edges = radial_gradient_deferred(width, height, &gradient, center, radius);
      let made = blended(&photo, &edges, mode, 0.8);
      let worst = made.rgba().iter().zip(expected.rgba()).map(|(a, e)| a.abs_diff(*e)).max().unwrap_or(0);
      assert!(worst <= 2, "{}: the fused gradient differs from the CPU by {worst}", mode.name());
    }
    assert_eq!(gpu_runs.load(Ordering::SeqCst) - runs_before, modes.len() * 2, "the gradient and then the blend, each on the GPU");

    // A picture that is just pixels goes over the photo as a texture.
    let picture = Image::new_from_color(width, height, Color::from_rgba(200, 30, 60, 128));
    let mut expected = photo.clone();
    abra_core::blend::blend(&picture).with_opacity(0.75).apply(&mut expected);
    let made = blended(&photo, &picture, BlendMode::Normal, 0.75);
    let worst = made.rgba().iter().zip(expected.rgba()).map(|(a, e)| a.abs_diff(*e)).max().unwrap_or(0);
    assert!(worst <= 1, "the GPU differs from the CPU by {worst}");

    // A chain of two stays one run and still matches.
    let edges = radial_gradient_deferred(width, height, &gradient, center, radius);
    let twice = blended(&blended(&photo, &edges, BlendMode::Normal, 1.0), &picture, BlendMode::Normal, 0.3);
    let mut by_hand = photo.clone();
    abra_core::blend::blend(&radial_gradient(width, height, &gradient, center, radius)).apply(&mut by_hand);
    abra_core::blend::blend(&picture).with_opacity(0.3).apply(&mut by_hand);
    let worst = twice.rgba().iter().zip(by_hand.rgba()).map(|(a, e)| a.abs_diff(*e)).max().unwrap_or(0);
    assert!(worst <= 3, "a chain of two differs by {worst}");
    // Two gradient blends (two opacities) and six with other modes, each the gradient's run and the blend's: sixteen. The
    // picture blend: one. The chain of two: the gradient's run and then the chain as one run: two.
    assert_eq!(gpu_runs.load(Ordering::SeqCst), 19, "every one of them ran on the GPU, the chain as one run");
    clear_gpu_provider();
    Ok(())
  }

  /// Every built-in blend mode on a real GPU against the CPU, over a 256 x 256 grid in which red and green hold every
  /// destination/source pair of values (one the other way round), blue a spread of pairs, and alpha varies, so the whole
  /// of each mode's arithmetic is covered. The modes that are whole-number math on both sides must match exactly; the hue,
  /// saturation, color, luminosity, lighter/darker color and divide modes keep the CPU's float steps and may differ by a level or
  /// two where a float lands either side of a rounding edge.
  #[test]
  fn every_blend_mode_gives_the_gpu_the_cpu_s_pixels() -> anyhow::Result<()> {
    use abra_core::BlendMode;
    use abra_core::image::gpu::{GPU_PROVIDER_LOCK, clear_gpu_provider};
    use abra_core::image::recipe::blended;

    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    gpu::GpuContext::new_default_blocking()?;
    let gpu_runs = counted_gpu();

    let size = 256u32;
    let (mut base, mut top) = (Vec::new(), Vec::new());
    for y in 0..size {
      for x in 0..size {
        base.extend_from_slice(&[x as u8, y as u8, ((x * 5 + y * 3) % 256) as u8, if (x + y) % 5 == 0 { 90 } else { 255 }]);
        top.extend_from_slice(&[y as u8, x as u8, ((x * 7 + y * 11) % 256) as u8, if (x * y) % 4 == 0 { 140 } else { 220 }]);
      }
    }
    let base = Image::new_from_pixels(size, size, base, Channels::RGBA);
    let top = Image::new_from_pixels(size, size, top, Channels::RGBA);

    // The modes that use floats on both sides, and how far they may differ.
    let float_modes = ["darker-color", "lighter-color", "divide", "hue", "saturation", "color", "luminosity"];
    let mut report = Vec::new();
    for mode in BlendMode::ALL {
      for opacity in [1.0f32, 0.55] {
        let mut expected = base.clone();
        abra_core::blend::blend(&top).with_mode(mode).with_opacity(opacity).apply(&mut expected);
        let made = blended(&base, &top, mode, opacity);
        assert!(made.deferred().is_some(), "{} is made by the GPU", mode.name());
        let worst = made.rgba().iter().zip(expected.rgba()).map(|(a, e)| a.abs_diff(*e)).max().unwrap_or(0);
        let allowed = if float_modes.contains(&mode.name()) { 3 } else { 0 };
        if worst > allowed {
          report.push(format!("{} at opacity {opacity}: differs by {worst} (allowed {allowed})", mode.name()));
        }
      }
    }
    clear_gpu_provider();
    assert_eq!(
      gpu_runs.load(std::sync::atomic::Ordering::SeqCst),
      BlendMode::ALL.len() * 2,
      "every mode, at both opacities, was made by the GPU and not by the CPU after a failed run"
    );
    assert!(report.is_empty(), "modes where the GPU differs from the CPU:\n{}", report.join("\n"));
    Ok(())
  }

  /// The vignette tool as TypeScript writes it, through the functions other languages call.
  #[cfg(feature = "uniffi")]
  #[test]
  fn a_vignette_made_from_the_exported_functions_darkens_the_corners_and_keeps_the_center() {
    use abra_core::{ColorStop, blend::blend_images};
    use std::sync::Arc;

    let photo = Arc::new(Image::new_from_color(101, 101, Color::from_rgba(200, 200, 200, 255)));
    let clear = Arc::new(Color::transparent());
    let dark = Arc::new(Color::from_rgba(0, 0, 0, 255));
    let edges = Arc::new(Gradient::new(vec![
      ColorStop::new(*clear, 0.0),
      ColorStop::new(*clear, 0.5),
      ColorStop::new(*dark, 1.0),
    ]));
    let half = 50.5;
    let reach = half * std::f32::consts::SQRT_2;
    let vignette = radial_gradient_image(101, 101, edges, half, half, reach, reach);
    let result = blend_images(photo, vignette, "normal".to_string(), 1.0).unwrap();
    assert_eq!(pixel(&result, 50, 50), [200, 200, 200, 255], "the middle is untouched");
    assert!(pixel(&result, 0, 0)[0] < 30, "the corner is nearly black: {:?}", pixel(&result, 0, 0));
    assert!(blend_images(Arc::new(Image::new(1, 1)), Arc::new(Image::new(1, 1)), "nope".to_string(), 1.0).is_err());
  }

  #[test]
  fn different_radii_make_an_oval() {
    let gradient = Gradient::from_to(Color::black(), Color::white());
    let image = radial_gradient(100, 100, &gradient, (50.0, 50.0), (50.0, 25.0));
    // 25 pixels sideways is half way, and so are 12.5 pixels up.
    let side = pixel(&image, 75, 50)[0] as i32;
    let up = pixel(&image, 50, 37)[0] as i32;
    assert!((side - up).abs() <= 8, "side {side}, up {up}");
  }

  #[test]
  fn an_empty_image_or_zero_radius_does_not_panic() {
    let gradient = Gradient::from_to(Color::black(), Color::white());
    radial_gradient(0, 0, &gradient, (0.0, 0.0), (1.0, 1.0));
    let image = radial_gradient(4, 4, &gradient, (2.0, 2.0), (0.0, 0.0));
    assert_eq!(pixel(&image, 0, 0), [255, 255, 255, 255]);
  }
}
