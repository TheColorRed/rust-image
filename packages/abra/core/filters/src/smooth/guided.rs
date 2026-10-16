//! The guided filter: smooths a mask while keeping its edges where the edges of a photo are.
//!
//! A mask made by a model is rough: it was found on a small copy of the photo, so its edge is soft and can sit a few pixels off.
//! The guided filter fixes that by looking at the photo. Where the photo has a real edge (a strand of hair against a wall) the
//! mask is made to change there, and where the photo is flat the mask is smoothed. It works for any mask and any photo, such as
//! a region found by a model, a selection drawn by hand or a color range.
//!
//! This is the color version of the filter of He, Sun and Tang ("Guided Image Filtering"), which uses all three channels of the
//! guide, so two things of the same brightness but different color still count as different.

use abra_core::image::gpu::{GpuAux, GpuPass, GpuProcessor};
use abra_core::{Image, ResizeTarget, Size, Transform, TransformAlgorithm};
use mask::Mask;
use options::{Effect, Options};
use rayon::prelude::*;
use std::sync::Arc;

/// The mean of each pixel's square window of `p_radius` pixels either way, with fewer pixels at the edges. Done as one running
/// sum along the rows and one down the columns, so it costs the same whatever the radius.
fn box_mean(p_plane: &[f32], p_width: usize, p_height: usize, p_radius: usize) -> Vec<f32> {
  let mut across = vec![0.0f32; p_plane.len()];
  across.par_chunks_mut(p_width).zip(p_plane.par_chunks(p_width)).for_each(|(out, row)| {
    let mut sums = Vec::with_capacity(row.len() + 1);
    sums.push(0.0f64);
    for value in row {
      sums.push(sums.last().copied().unwrap_or(0.0) + *value as f64);
    }
    for (x, slot) in out.iter_mut().enumerate() {
      let (from, to) = (x.saturating_sub(p_radius), (x + p_radius + 1).min(row.len()));
      *slot = ((sums[to] - sums[from]) / (to - from) as f64) as f32;
    }
  });
  // Down the columns, a row at a time: add the row that enters the window and take away the one that leaves it.
  let mut out = vec![0.0f32; p_plane.len()];
  let mut sums = vec![0.0f64; p_width];
  let row_at = |p_y: usize| &across[p_y * p_width..(p_y + 1) * p_width];
  for y in 0..(p_radius + 1).min(p_height) {
    row_at(y).iter().zip(sums.iter_mut()).for_each(|(value, sum)| *sum += *value as f64);
  }
  for y in 0..p_height {
    if y > 0 {
      if y + p_radius < p_height {
        row_at(y + p_radius).iter().zip(sums.iter_mut()).for_each(|(value, sum)| *sum += *value as f64);
      }
      if y > p_radius {
        row_at(y - p_radius - 1).iter().zip(sums.iter_mut()).for_each(|(value, sum)| *sum -= *value as f64);
      }
    }
    let count = ((y + p_radius + 1).min(p_height) - y.saturating_sub(p_radius)) as f64;
    out[y * p_width..(y + 1) * p_width].iter_mut().zip(&sums).for_each(|(slot, sum)| *slot = (*sum / count) as f32);
  }
  out
}

/// A mask made to follow the edges of a photo.
///
/// Within each window of `p_radius` pixels the result is the best straight-line fit of the mask to the photo's colors, so it
/// rises where the photo's color changes and is smooth where the photo is flat. A mask of another size than the photo is first
/// scaled to it, so the result is the size of the photo: a rough mask found on a small copy can be sharpened at full size.
///
/// - `p_mask`: The mask to refine.
/// - `p_guide`: The photo whose edges the result follows. Its alpha is ignored.
/// - `p_radius`: How far in pixels the filter looks around each pixel. The mask's edge can move this far to meet an edge of the
///   photo, and a larger radius smooths more.
/// - `p_epsilon`: How strong an edge in the photo has to be, with colors counted from `0` to `1`, before the mask follows it. Small
///   values (`0.0001`) follow even faint edges; larger ones (`0.01`) follow only strong ones and smooth the rest.
pub fn guided_filter(p_mask: &Mask, p_guide: &Image, p_radius: u32, p_epsilon: f32) -> Mask {
  let (width, height) = p_guide.dimensions::<u32>();
  let (w, h) = (width as usize, height as usize);
  if w == 0 || h == 0 {
    return Mask::from_values(width, height, Vec::new());
  }
  let resized;
  let source = if p_mask.dimensions::<u32>() == (width, height) {
    p_mask
  } else {
    resized = p_mask.resized(width, height, TransformAlgorithm::Bilinear);
    &resized
  };
  let radius = p_radius as usize;
  let count = w * h;
  let rgba = p_guide.rgba();
  let channel = |p_offset: usize| (0..count).map(|i| rgba[i * 4 + p_offset] as f32 / 255.0).collect::<Vec<f32>>();
  let (red, green, blue) = (channel(0), channel(1), channel(2));
  let p: Vec<f32> = source.values().iter().map(|value| *value as f32 / 255.0).collect();
  let times = |p_a: &[f32], p_b: &[f32]| p_a.iter().zip(p_b).map(|(a, b)| a * b).collect::<Vec<f32>>();

  // The means over every window of the photo's colors, the mask, and how they vary together.
  let planes = [
    red.clone(),
    green.clone(),
    blue.clone(),
    p.clone(),
    times(&red, &p),
    times(&green, &p),
    times(&blue, &p),
    times(&red, &red),
    times(&red, &green),
    times(&red, &blue),
    times(&green, &green),
    times(&green, &blue),
    times(&blue, &blue),
  ];
  let means: Vec<Vec<f32>> = planes.par_iter().map(|plane| box_mean(plane, w, h, radius)).collect();
  let [m_r, m_g, m_b, m_p, m_rp, m_gp, m_bp, m_rr, m_rg, m_rb, m_gg, m_gb, m_bb] = &means[..] else {
    unreachable!("there are thirteen planes");
  };

  // In each window the mask is fit as `a · color + b`: `a` is how the mask moves with each color, `b` is what is left.
  let mut fit: Vec<[f32; 4]> = vec![[0.0; 4]; count];
  fit.par_iter_mut().enumerate().for_each(|(i, slot)| {
    // Done in double precision: the matrix is the size of the epsilon, and its inverse is the difference of small products.
    let f = |p_value: f32| p_value as f64;
    let (r, g, b) = (f(m_r[i]), f(m_g[i]), f(m_b[i]));
    let epsilon = p_epsilon as f64;
    let covariance = [f(m_rp[i]) - r * f(m_p[i]), f(m_gp[i]) - g * f(m_p[i]), f(m_bp[i]) - b * f(m_p[i])];
    let (rr, rg, rb) = (f(m_rr[i]) - r * r + epsilon, f(m_rg[i]) - r * g, f(m_rb[i]) - r * b);
    let (gg, gb, bb) = (f(m_gg[i]) - g * g + epsilon, f(m_gb[i]) - g * b, f(m_bb[i]) - b * b + epsilon);
    // The inverse of the 3 x 3 matrix of how the colors vary together, from its cofactors.
    let (c00, c01, c02) = (gg * bb - gb * gb, rb * gb - rg * bb, rg * gb - rb * gg);
    let (c11, c12, c22) = (rr * bb - rb * rb, rg * rb - rr * gb, rr * gg - rg * rg);
    let determinant = rr * c00 + rg * c01 + rb * c02;
    let a = if determinant > 0.0 && determinant.is_finite() {
      [
        (c00 * covariance[0] + c01 * covariance[1] + c02 * covariance[2]) / determinant,
        (c01 * covariance[0] + c11 * covariance[1] + c12 * covariance[2]) / determinant,
        (c02 * covariance[0] + c12 * covariance[1] + c22 * covariance[2]) / determinant,
      ]
    } else {
      [0.0; 3]
    };
    *slot = [a[0] as f32, a[1] as f32, a[2] as f32, (f(m_p[i]) - a[0] * r - a[1] * g - a[2] * b) as f32];
  });

  // Each pixel is in many windows, so it averages the fits of them all.
  let fit_planes: Vec<Vec<f32>> = (0..4).map(|k| fit.iter().map(|each| each[k]).collect()).collect();
  let smoothed: Vec<Vec<f32>> = fit_planes.par_iter().map(|plane| box_mean(plane, w, h, radius)).collect();
  let values = (0..count)
    .into_par_iter()
    .map(|i| {
      let q = smoothed[0][i] * red[i] + smoothed[1][i] * green[i] + smoothed[2][i] * blue[i] + smoothed[3][i];
      (q.clamp(0.0, 1.0) * 255.0).round() as u8
    })
    .collect();
  Mask::from_values(width, height, values)
}

/// A mask made to follow the edges of a photo, using only the photo's brightness (0.299 red + 0.587 green + 0.114 blue). This is
/// [`guided_filter`] with a one-number guide instead of three: it is what [`Guided`] does on the CPU and the GPU, so the two give the
/// same pixels, and it is cheaper. Two things of the same brightness and different color count as the same.
pub(crate) fn guided_brightness(p_mask: &Mask, p_guide: &Image, p_radius: u32, p_epsilon: f32) -> Mask {
  let (width, height) = p_guide.dimensions::<u32>();
  let (w, h) = (width as usize, height as usize);
  if w == 0 || h == 0 {
    return Mask::from_values(width, height, Vec::new());
  }
  let resized;
  let source = if p_mask.dimensions::<u32>() == (width, height) {
    p_mask
  } else {
    resized = p_mask.resized(width, height, TransformAlgorithm::Bilinear);
    &resized
  };
  let radius = p_radius as usize;
  let brightness: Vec<f32> = p_guide
    .rgba()
    .chunks_exact(4)
    .map(|pixel| pixel[0] as f32 / 255.0 * 0.299 + pixel[1] as f32 / 255.0 * 0.587 + pixel[2] as f32 / 255.0 * 0.114)
    .collect();
  let p: Vec<f32> = source.values().iter().map(|value| *value as f32 / 255.0).collect();
  let planes = [
    brightness.clone(),
    p.clone(),
    brightness.iter().zip(&p).map(|(i, p)| i * p).collect(),
    brightness.iter().map(|i| i * i).collect(),
  ];
  let means: Vec<Vec<f32>> = planes.par_iter().map(|plane| box_mean(plane, w, h, radius)).collect();
  let [m_i, m_p, m_ip, m_ii] = &means[..] else {
    unreachable!("there are four planes");
  };
  let epsilon = p_epsilon as f64;
  let fit: Vec<[f32; 2]> = (0..w * h)
    .into_par_iter()
    .map(|k| {
      let variance = m_ii[k] as f64 - m_i[k] as f64 * m_i[k] as f64;
      let covariance = m_ip[k] as f64 - m_i[k] as f64 * m_p[k] as f64;
      let a = covariance / (variance + epsilon);
      [a as f32, (m_p[k] as f64 - a * m_i[k] as f64) as f32]
    })
    .collect();
  let (a, b): (Vec<f32>, Vec<f32>) = fit.iter().map(|each| (each[0], each[1])).unzip();
  let (a, b) = rayon::join(|| box_mean(&a, w, h, radius), || box_mean(&b, w, h, radius));
  let values =
    (0..w * h).into_par_iter().map(|k| ((a[k] * brightness[k] + b[k]).clamp(0.0, 1.0) * 255.0).round() as u8).collect();
  Mask::from_values(width, height, values)
}

/// The guided filter as an effect on a picture of a mask (white where it reaches; the red channel is read), so it runs on the
/// GPU as well as the CPU and gives the same pixels on both to within a level. The result is an opaque gray picture of the refined
/// mask. Made by [`guided`].
///
/// The guide is the brightness of a photo, as in `guided_brightness`. For a guide that counts color, use [`guided_filter`], which
/// runs on the CPU.
///
/// On the GPU the work is held in `rgba32float` textures, 16 bytes a pixel for each of two, so use a picture of the mask no larger
/// than it needs to be (the size the model looked at, or the screen's).
#[derive(Clone)]
pub struct Guided {
  guide: Image,
  /// The guide's pixels, kept once so each frame hands the GPU the same buffer.
  guide_rgba: Arc<[u8]>,
  radius: u32,
  epsilon: f32,
  options: Options,
}

/// A guided filter that makes a picture of a mask follow the edges of `p_guide`.
/// - `p_guide`: The photo whose edges the mask follows. A guide of another size than the picture it is applied to is scaled to it.
/// - `p_radius`: How far in pixels the filter looks around each pixel. `0` changes nothing.
/// - `p_epsilon`: How strong an edge in the photo has to be, with brightness counted from `0` to `1`, before the mask follows it.
///   `0.0001` follows faint edges and `0.01` only strong ones.
pub fn guided(p_guide: &Image, p_radius: u32, p_epsilon: f32) -> Guided {
  Guided {
    guide: p_guide.clone(),
    guide_rgba: Arc::from(p_guide.rgba()),
    radius: p_radius,
    epsilon: p_epsilon,
    options: None,
  }
}

impl Guided {
  /// The guide at the size of the picture the filter is applied to.
  fn guide_at(&self, p_width: u32, p_height: u32) -> Image {
    let mut guide = self.guide.clone();
    if guide.dimensions::<u32>() != (p_width, p_height) {
      guide.resize(ResizeTarget::Exact(Size::new(p_width, p_height)), None);
    }
    guide
  }
}

/// `p_pass` with the guide photo bound at binding 3.
fn with_guide(mut p_pass: GpuPass, p_guide: &GpuAux) -> GpuPass {
  p_pass.aux = Some(p_guide.clone());
  p_pass
}

impl GpuProcessor for Guided {
  /// Seven passes: the guide's brightness and the mask are put together into numbers, which are averaged over each window (along the
  /// rows, then down the columns); the line that fits them is worked out for each window; the lines are averaged the same way;
  /// and each pixel reads its line at the guide's brightness. The numbers in between are `rgba32float`, and only the last pass is 8-bit.
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    if self.radius == 0 {
      return Vec::new();
    }
    let rgba = if self.guide.dimensions::<u32>() == (p_width, p_height) {
      self.guide_rgba.clone()
    } else {
      Arc::from(self.guide_at(p_width, p_height).rgba())
    };
    let aux = GpuAux { width: p_width, height: p_height, rgba };
    let bytes = |p_values: [u32; 4]| p_values.iter().flat_map(|value| value.to_le_bytes()).collect::<Vec<u8>>();
    let box_pass = |direction: u32| {
      GpuPass::new(include_str!("./guided_box.wgsl"), bytes([self.radius, direction, 0, 0]))
        .with_reach(self.radius as f32)
        .with_pixel_exact()
    };
    let mut fit_bytes = self.epsilon.to_le_bytes().to_vec();
    fit_bytes.resize(16, 0);
    vec![
      with_guide(GpuPass::new(include_str!("./guided_stats.wgsl"), Vec::new()).with_pixel_exact(), &aux),
      box_pass(0),
      box_pass(1),
      GpuPass::new(include_str!("./guided_fit.wgsl"), fit_bytes).with_pixel_exact(),
      box_pass(0),
      box_pass(1),
      with_guide(GpuPass::new(include_str!("./guided_apply.wgsl"), Vec::new()).with_pixel_exact(), &aux),
    ]
  }
}

impl Effect for Guided {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  /// The window reaches this far twice (once to find the line and once to average it).
  fn padding(&self) -> i32 {
    2 * self.radius as i32
  }

  /// The guide is a picture of the whole photo, so the filter needs the whole picture and not a part of it.
  fn positional(&self) -> bool {
    true
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    if self.radius == 0 {
      return;
    }
    let (width, height) = p_image.dimensions::<u32>();
    let values = p_image.rgba().chunks_exact(4).map(|pixel| pixel[0]).collect();
    let refined =
      guided_brightness(&Mask::from_values(width, height, values), &self.guide_at(width, height), self.radius, self.epsilon);
    let pixels = refined.values().iter().flat_map(|value| [*value, *value, *value, 255]).collect::<Vec<u8>>();
    p_image.set_rgba(pixels);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Color;

  /// A photo `width` wide that is black to the left of `edge` and white from it, and a mask that is full from `mask_edge`.
  fn split(width: u32, edge: u32, mask_edge: u32) -> (Image, Mask) {
    let mut photo = Image::new_from_color(width, 4, Color::from_rgba(0, 0, 0, 255));
    let mut values = vec![0u8; width as usize * 4];
    for y in 0..4 {
      for x in 0..width {
        if x >= edge {
          photo.set_pixel(x, y, (255, 255, 255, 255));
        }
        if x >= mask_edge {
          values[(y * width + x) as usize] = 255;
        }
      }
    }
    (photo, Mask::from_values(width, 4, values))
  }

  #[test]
  fn a_misplaced_mask_edge_is_pulled_towards_the_edge_of_the_photo() {
    // The mask reaches two pixels onto the black side of the photo's edge at x = 8.
    let (photo, mask) = split(16, 8, 6);
    let flat = Image::new_from_color(16, 4, Color::from_rgba(90, 90, 90, 255));
    let followed = guided_filter(&mask, &photo, 4, 0.0001);
    let blurred = guided_filter(&mask, &flat, 4, 0.0001);
    let row = |p_mask: &Mask| p_mask.values()[16..32].to_vec();
    // The black side, which the mask should not reach, gets less of it than a plain blur gives, and the white side no less.
    assert!(followed.get(7, 1).unwrap() + 10 < blurred.get(7, 1).unwrap(), "{:?} against {:?}", row(&followed), row(&blurred));
    assert!(followed.get(9, 1).unwrap() >= blurred.get(9, 1).unwrap(), "{:?} against {:?}", row(&followed), row(&blurred));
  }

  #[test]
  fn a_mask_that_is_all_or_nothing_stays_that_way() {
    let photo = split(8, 4, 0).0;
    assert!(guided_filter(&Mask::from_values(8, 4, vec![255; 32]), &photo, 3, 0.001).values().iter().all(|v| *v >= 254));
    assert!(guided_filter(&Mask::from_values(8, 4, vec![0; 32]), &photo, 3, 0.001).values().iter().all(|v| *v <= 1));
  }

  #[test]
  fn the_result_is_the_size_of_the_photo() {
    let (photo, _) = split(16, 8, 8);
    let small = Mask::from_values(4, 1, vec![0, 0, 255, 255]);
    assert_eq!(guided_filter(&small, &photo, 2, 0.001).dimensions::<u32>(), (16, 4));
  }

  /// A photo of values that change from pixel to pixel, and a picture of a rough mask over it.
  fn varied(p_width: u32, p_height: u32) -> (Vec<u8>, Vec<u8>) {
    let photo = (0..p_width * p_height)
      .flat_map(|i| {
        let x = i.wrapping_mul(2_654_435_761) >> 13;
        [(x % 251) as u8, ((x >> 3) % 241) as u8, ((x >> 5) % 239) as u8, 255]
      })
      .collect();
    let mask = (0..p_width * p_height)
      .flat_map(|i| {
        let (x, y) = (i % p_width, i / p_width);
        let value = if (x / 7 + y / 5) % 2 == 0 { 230u8 } else { 20 };
        [value, value, value, 255]
      })
      .collect();
    (photo, mask)
  }

  #[test]
  fn the_guided_effect_on_the_gpu_matches_the_cpu_within_a_level() -> anyhow::Result<()> {
    use abra_core::Channels;
    use gpu::{GpuContext, LiveRenderer};
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let (width, height) = (41u32, 27u32);
    let (photo, mask) = varied(width, height);
    let photo = Image::new_from_pixels(width, height, photo, Channels::RGBA);
    for (radius, epsilon) in [(1u32, 0.01f32), (3, 0.001), (8, 0.0001)] {
      let mut on_cpu = Image::new_from_pixels(width, height, mask.clone(), Channels::RGBA);
      guided(&photo, radius, epsilon).apply_on_cpu(&mut on_cpu);
      let on_gpu = renderer.process(&[guided(&photo, radius, epsilon).passes(width, height)], width, height, &mask)?;
      let worst = on_gpu.chunks_exact(4).zip(on_cpu.rgba().chunks_exact(4)).map(|(g, c)| g[0].abs_diff(c[0])).max().unwrap_or(0);
      assert!(worst <= 2, "radius {radius}, epsilon {epsilon}: the GPU is {worst} levels from the CPU");
      assert!(on_cpu.rgba() != mask, "radius {radius}: the filter changed nothing");
    }
    Ok(())
  }

  #[test]
  fn the_guided_effect_with_a_radius_of_zero_changes_nothing() {
    use abra_core::Channels;
    let (photo, mask) = varied(9, 5);
    let photo = Image::new_from_pixels(9, 5, photo, Channels::RGBA);
    let mut image = Image::new_from_pixels(9, 5, mask.clone(), Channels::RGBA);
    guided(&photo, 0, 0.001).apply_on_cpu(&mut image);
    assert_eq!(image.rgba(), &mask[..]);
    assert!(guided(&photo, 0, 0.001).passes(9, 5).is_empty());
  }

  #[test]
  fn a_flat_photo_just_smooths_the_mask() {
    let photo = Image::new_from_color(9, 1, Color::from_rgba(90, 90, 90, 255));
    let mask = Mask::from_values(9, 1, vec![0, 0, 0, 0, 255, 0, 0, 0, 0]);
    let refined = guided_filter(&mask, &photo, 1, 0.001);
    // Nothing in the photo to follow, so the point spreads over its neighbors like a blur.
    assert!(refined.get(4, 0).unwrap() < 255 && refined.get(3, 0).unwrap() > 0);
  }
}
