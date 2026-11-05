use crate::{Brush, Sprite, SpriteTrail, Sprites, TrailPoint, sprites};
use abra_core::image::effect::Effect;
use abra_core::image::gpu::{GpuPass, GpuProcessor};
use abra_core::image::recipe::{Generator, generated};
use abra_core::{Channels, Fill, Image};
use std::sync::Arc;

/// Most points one stroke is drawn from. A line with more is thinned evenly (keeping both ends) before it is drawn, so a long
/// stroke still fits in the shader's uniform buffer and costs a bounded amount per pixel.
pub const MAX_STROKE_POINTS: usize = 256;

/// What a brush looks like as a line, in the terms a shader draws it: how far from the line it reaches, how wide its fade is and its
/// color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeLook {
  /// Half the brush's size, in pixels.
  pub radius: f32,
  /// How wide the fade at the edge is, in pixels: the part of the radius that the brush's softness covers, and at least one so the
  /// edge is never jagged.
  pub edge: f32,
  /// The color, straight (not multiplied by its alpha), each from 0 to 1, the last being the color's alpha times the brush's opacity at the
  /// start of a stroke, since a renderer that draws a line a piece at a time in one style has one opacity. A brush that is not a solid
  /// color is black.
  pub color: [f32; 4],
}

/// The color of a brush, straight, each from 0 to 1, with its own alpha and not its opacity. A brush that is not a solid color is black.
fn solid_color(p_brush: &Brush) -> [f32; 4] {
  let color = match p_brush.color() {
    Fill::Solid(color) => color.rgba(),
    _ => (0, 0, 0, 255),
  };
  [color.0 as f32 / 255.0, color.1 as f32 / 255.0, color.2 as f32 / 255.0, color.3 as f32 / 255.0]
}

impl StrokeLook {
  /// How `p_brush` draws a line in one style: its size, and its opacity at the start of the stroke. A width or an opacity that changes along
  /// the line needs a renderer that reads it along the line, as [`stroke_overlay`] does.
  pub fn of(p_brush: &Brush) -> StrokeLook {
    let radius = p_brush.size() / 2.0;
    let mut color = solid_color(p_brush);
    color[3] *= p_brush.opacity().opacity_at(0.0);
    StrokeLook {
      radius,
      edge: ((1.0 - p_brush.hardness()) * radius).max(1.0),
      color,
    }
  }
}

/// A line through points, drawn with a brush's width, hardness, opacity and color: a trail of the stamping engine (`sprites`), so it is
/// drawn like any other brush stroke, on the CPU by [`render`](Generator::render) or by the engine's shader.
#[derive(Debug)]
struct StrokeOverlay {
  sprites: Sprites,
}

/// A line of more than [`MAX_STROKE_POINTS`] points, thinned evenly (keeping both ends).
fn thinned(p_points: &[(f32, f32)]) -> Vec<(f32, f32)> {
  if p_points.len() <= MAX_STROKE_POINTS {
    p_points.to_vec()
  } else {
    (0..MAX_STROKE_POINTS).map(|i| p_points[i * (p_points.len() - 1) / (MAX_STROKE_POINTS - 1)]).collect()
  }
}

impl StrokeOverlay {
  fn new(p_points: &[(f32, f32)], p_brush: &Brush) -> StrokeOverlay {
    let points = thinned(p_points);
    // The width of the brush at each point, by how far along the line the point is: `0` at the first and `1` at the last, by length.
    let width = p_brush.width();
    let opacity = p_brush.opacity();
    let mut lengths = vec![0.0_f32; points.len()];
    for at in 1..points.len() {
      lengths[at] = lengths[at - 1] + ((points[at].0 - points[at - 1].0).powi(2) + (points[at].1 - points[at - 1].1).powi(2)).sqrt();
    }
    let total = lengths.last().copied().unwrap_or(0.0);
    let along = |length: &f32| if total > 0.0 { length / total } else { 0.0 };
    let [red, green, blue, alpha] = solid_color(p_brush);
    let color = ((red * 255.0 + 0.5) as u8, (green * 255.0 + 0.5) as u8, (blue * 255.0 + 0.5) as u8);
    let sprites = match points.as_slice() {
      [] => sprites(),
      // A single point is a dot.
      [(x, y)] => sprites().with_sprites(vec![
        Sprite::new(*x, *y, width.width_at(0.0) / 2.0).with_hardness(p_brush.hardness()).with_color(color).with_opacity(opacity.opacity_at(0.0) * alpha),
      ]),
      _ => {
        let trail = points.iter().zip(&lengths).map(|((x, y), length)| {
          TrailPoint::new(*x, *y, width.width_at(along(length))).with_opacity(opacity.opacity_at(along(length)) * alpha).with_color(color)
        });
        sprites().with_trails(vec![SpriteTrail::new(trail.collect()).with_hardness(p_brush.hardness())])
      }
    };
    StrokeOverlay { sprites }
  }
}

impl Generator for StrokeOverlay {
  fn render(&self, p_width: u32, p_height: u32) -> Image {
    let mut image = Image::new_from_pixels(p_width, p_height, vec![0u8; p_width as usize * p_height as usize * 4], Channels::RGBA);
    self.sprites.cpu_processor(&mut image);
    image
  }

  fn has_gpu(&self) -> bool {
    self.sprites.gpu_supported()
  }

  fn passes(&self, p_width: u32, p_height: u32) -> Option<Vec<GpuPass>> {
    self.sprites.gpu_supported().then(|| self.sprites.passes(p_width, p_height))
  }
}

/// An image of `p_width` x `p_height` pixels holding a line drawn through `p_points` (in pixels of the image) with `p_brush`'s
/// width, hardness, opacity and solid color. Pixels away from the line are transparent, so the image is made to be blended over a
/// photo; a brush that is not a solid color draws in black. A line of more than [`MAX_STROKE_POINTS`] points is thinned evenly.
///
/// The brush's width and opacity are read along the line, `0` at its first point and `1` at its last, by length (see `Brush::with_width` and
/// `Brush::with_opacity`), so a line can taper, swell or fade. A brush with only a size is that wide all the way, and one with only a number
/// for its opacity is that opaque all the way. A line that gains points changes the places the width is read at,
/// so a taper to a point follows the end of the line as it grows.
///
/// The line is drawn the way a brush stroke is, by the stamping engine, so its edge is the brush's hardness.
///
/// The pixels are made on the CPU now. [`stroke_overlay_deferred`] leaves them to the GPU when there is one.
pub fn stroke_overlay(p_width: u32, p_height: u32, p_points: &[(f32, f32)], p_brush: &Brush) -> Image {
  StrokeOverlay::new(p_points, p_brush).render(p_width, p_height)
}

/// [`stroke_overlay`] as an image that is made by the GPU when its pixels are first read (see `abra_core::image::recipe`). A
/// line that gains a point is a new image that costs one shader run, so it can follow a finger frame after frame. Without a GPU
/// it is made now, with the same pixels either way.
pub fn stroke_overlay_deferred(p_width: u32, p_height: u32, p_points: &[(f32, f32)], p_brush: &Brush) -> Image {
  generated(p_width, p_height, Arc::new(StrokeOverlay::new(p_points, p_brush)))
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Color, StrokeOpacity};

  fn alpha_at(p_image: &Image, p_x: usize, p_y: usize) -> u8 {
    p_image.rgba()[(p_y * p_image.dimensions::<u32>().0 as usize + p_x) * 4 + 3]
  }

  #[test]
  fn the_line_is_solid_on_it_soft_at_its_edge_and_clear_away_from_it() {
    let red = Color::red();
    let hard = Brush::new().with_size(9).with_color(&red).with_hardness(1.0);
    let image = stroke_overlay(40, 20, &[(5.0, 10.0), (35.0, 10.0)], &hard);
    assert_eq!(alpha_at(&image, 20, 10), 255);
    assert_eq!(alpha_at(&image, 20, 2), 0);
    // A softer brush fades toward its edge, which is 4.5 pixels from the line: the middle of the pixel in row 13 is 3.5 away.
    let soft = stroke_overlay(40, 20, &[(5.0, 10.0), (35.0, 10.0)], &Brush::new().with_size(9).with_color(&red).with_hardness(0.5));
    let edge = alpha_at(&soft, 20, 13);
    assert!(edge > 0 && edge < 255, "{edge}");
    assert_eq!(alpha_at(&stroke_overlay(8, 8, &[], &hard), 4, 4), 0);
  }

  #[test]
  fn a_line_with_a_width_that_tapers_is_thinner_toward_its_end_and_ends_in_a_point() {
    let brush = Brush::new().with_size(20).with_width(abra_core::StrokeWidth::from_to(20.0, 0.0)).with_hardness(1.0);
    let image = stroke_overlay(100, 40, &[(10.0, 20.0), (90.0, 20.0)], &brush);
    let height_at = |x: usize| (0..40).filter(|y| alpha_at(&image, x, *y) > 127).count();
    assert!(height_at(15) > height_at(50) && height_at(50) > height_at(85), "{} {} {}", height_at(15), height_at(50), height_at(85));
    // A hard edge is solid over all of the radius here, so the width at 15 is 17.5 and at 50 is 10.
    assert!((16..=19).contains(&height_at(15)), "{}", height_at(15));
    // Nothing at the point itself: a radius of nothing covers nothing, not even half a pixel.
    assert_eq!(alpha_at(&image, 90, 20), 0);
  }

  #[test]
  fn a_line_with_an_opacity_that_fades_is_clearer_toward_its_end() {
    let brush = Brush::new().with_size(10).with_hardness(1.0).with_opacity(StrokeOpacity::from_to(1.0, 0.0));
    let image = stroke_overlay(100, 30, &[(10.0, 15.0), (90.0, 15.0)], &brush);
    let (start, middle, end) = (alpha_at(&image, 14, 15), alpha_at(&image, 50, 15), alpha_at(&image, 86, 15));
    assert!(start > 220 && (110..=145).contains(&middle) && end < 40, "{start} {middle} {end}");
    // A number is that opaque the whole way, and a brush without one is solid.
    let half = stroke_overlay(100, 30, &[(10.0, 15.0), (90.0, 15.0)], &Brush::new().with_size(10).with_hardness(1.0).with_opacity(0.5));
    assert!((alpha_at(&half, 50, 15) as i32 - 128).abs() <= 2);
    assert_eq!(alpha_at(&stroke_overlay(100, 30, &[(10.0, 15.0), (90.0, 15.0)], &Brush::new().with_size(10).with_hardness(1.0)), 50, 15), 255);
  }

  #[test]
  fn a_width_that_never_changes_draws_what_the_size_does() {
    let by_size = Brush::new().with_size(12).with_hardness(0.5);
    let by_width = Brush::new().with_size(3).with_width(abra_core::StrokeWidth::constant(12.0)).with_hardness(0.5);
    let points = [(5.0, 10.0), (20.0, 25.0), (45.0, 12.0)];
    assert_eq!(stroke_overlay(50, 30, &points, &by_size).rgba(), stroke_overlay(50, 30, &points, &by_width).rgba());
  }

  #[test]
  fn a_line_that_leaves_the_pixels_it_is_drawn_on_is_clipped_not_wrapped() {
    let image = stroke_overlay(30, 30, &[(-40.0, 15.0), (-10.0, 15.0), (15.0, 15.0)], &Brush::new().with_size(6).with_hardness(1.0));
    assert_eq!(alpha_at(&image, 15, 15), 255);
    assert_eq!(alpha_at(&image, 2, 25), 0);
  }

  #[test]
  fn a_long_line_is_thinned_to_the_most_points_a_line_is_drawn_from() {
    let points: Vec<(f32, f32)> = (0..1000).map(|i| (i as f32 / 10.0, 5.0)).collect();
    let thin = thinned(&points);
    assert_eq!(thin.len(), MAX_STROKE_POINTS);
    assert_eq!(thin[0], points[0]);
    assert_eq!(thin[MAX_STROKE_POINTS - 1], points[999]);
    assert_eq!(thinned(&points[..10]), points[..10].to_vec());
  }

  #[test]
  fn a_single_point_is_a_dot() {
    let image = stroke_overlay(20, 20, &[(10.5, 10.5)], &Brush::new().with_size(8).with_hardness(1.0));
    assert_eq!(alpha_at(&image, 10, 10), 255);
    assert_eq!(alpha_at(&image, 2, 2), 0);
  }

  /// The shader on a real GPU gives the CPU's pixels, for a hard and a soft brush. The real provider is wrapped in one that counts,
  /// so the test fails if the work quietly fell back to the CPU (which would match perfectly).
  #[test]
  fn the_gpu_gives_the_cpu_s_pixels() -> anyhow::Result<()> {
    use abra_core::BlendMode;
    use abra_core::image::gpu::{GPU_PROVIDER_LOCK, GpuProvider, get_gpu_provider, register_gpu_provider};
    use abra_core::image::recipe::blended;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static REAL: OnceLock<GpuProvider> = OnceLock::new();
    static RUNS: AtomicUsize = AtomicUsize::new(0);

    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    gpu::GpuContext::new_default_blocking()?;
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

    let (width, height) = (97, 61);
    let photo = Image::new_from_pixels(width, height, [90u8, 120, 150, 255].repeat((width * height) as usize), Channels::RGBA);
    let white = Color::white();
    let short: Vec<(f32, f32)> = vec![(8.0, 50.0), (30.0, 12.0), (60.5, 40.0), (90.0, 8.0)];
    // A long loop, so that the grid has many cells with a few pieces each, and a scribble that crowds one cell into checking every piece.
    let long: Vec<(f32, f32)> = (0..255).map(|i| i as f32 / 255.0 * std::f32::consts::TAU).map(|a| (48.0 + 40.0 * a.cos() * (1.0 + 0.1 * (a * 5.0).sin()), 30.0 + 24.0 * a.sin())).collect();
    let scribble: Vec<(f32, f32)> = (0..200).map(|i| (40.0 + (i % 7) as f32, 30.0 + (i % 5) as f32)).collect();
    let tapered = abra_core::StrokeWidth::from_to(20.0, 1.0);
    let fading = abra_core::StrokeOpacity::new(vec![abra_core::OpacityStop::new(0.2, 0.0), abra_core::OpacityStop::new(1.0, 0.5), abra_core::OpacityStop::new(0.0, 1.0)]);
    let swelling = abra_core::StrokeWidth::new(vec![abra_core::WidthStop::new(2.0, 0.0), abra_core::WidthStop::new(24.0, 0.4), abra_core::WidthStop::new(0.0, 1.0)]);
    for (points, size, hardness, along, fade) in [
      (&short, 9, 1.0, None, None),
      (&short, 22, 0.0, None, None),
      (&short, 14, 0.5, None, None),
      (&short, 14, 0.5, Some(tapered.clone()), None),
      (&short, 14, 1.0, Some(swelling.clone()), None),
      (&short, 12, 0.5, None, Some(fading.clone())),
      (&long, 6, 0.5, None, None),
      (&long, 8, 1.0, Some(tapered), Some(fading.clone())),
      (&scribble, 5, 0.5, Some(swelling), Some(fading)),
    ] {
      let mut brush = Brush::new().with_size(size).with_color(&white).with_hardness(hardness).with_opacity(fade.unwrap_or(StrokeOpacity::constant(0.8)));
      if let Some(along) = along {
        brush = brush.with_width(along);
      }
      let expected = blended(&photo, &stroke_overlay(width, height, points, &brush), BlendMode::Normal, 1.0);
      let made = blended(&photo, &stroke_overlay_deferred(width, height, points, &brush), BlendMode::Normal, 1.0);
      assert!(made.deferred().is_some(), "with a GPU the result is not made until it is read");
      let worst = made.rgba().iter().zip(expected.rgba()).map(|(a, e)| a.abs_diff(*e)).max().unwrap_or(0);
      assert!(worst <= 2, "size {size} hardness {hardness}: the GPU differs from the CPU by {worst}");
    }
    assert!(RUNS.load(Ordering::SeqCst) >= 9, "the GPU did not run the shader");
    abra_core::image::gpu::clear_gpu_provider();
    Ok(())
  }

  /// A zoomed-out view of a line is the line drawn in the photo's pixels and shown smaller, so a horizontal line across the middle of
  /// the photo is across the middle of the half-size view and as thick as half its width.
  #[test]
  fn a_zoomed_out_view_of_the_line_is_the_same_line_smaller() -> anyhow::Result<()> {
    use abra_core::ImageExt;
    use abra_core::image::gpu::GPU_PROVIDER_LOCK;

    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    gpu::GpuContext::new_default_blocking()?;
    gpu::register();
    let brush = Brush::new().with_size(20).with_hardness(1.0);
    let line = stroke_overlay_deferred(200, 100, &[(20.0, 50.0), (180.0, 50.0)], &brush);
    let half = line.region(1, 0, 0, 100, 50);
    let alpha = |x: usize, y: usize| half[(y * 100 + x) * 4 + 3];
    let thick = |x: usize| (0..50).filter(|y| alpha(x, *y) > 127).count();
    // The line is hard, so it is 20 pixels wide in the photo and about 10 in the half-size view.
    assert!((9..=11).contains(&thick(50)), "{}", thick(50));
    assert_eq!(alpha(50, 5), 0);
    assert_eq!(alpha(3, 25), 0, "the line starts at x 20 of the photo, which is x 10 here");
    abra_core::image::gpu::clear_gpu_provider();
    Ok(())
  }
}
