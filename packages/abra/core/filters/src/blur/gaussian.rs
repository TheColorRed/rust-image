use crate::common::*;
use abra_core::image::gpu::{CpuProcessor, GpuAux, GpuPass, GpuProcessor};
use abra_core::image::levels::scaled_to_level;
use abra_core::{Channels, ResizeTarget, Size, Transform};
use abra_core::{IntoNumber, if_pick};
use primitives::gaussian::gaussian_weights;

#[cfg(test)]
fn gaussian_kernel_1d(p_radius: u32) -> Vec<f32> {
  let mut kernel = vec![0.0; (2 * p_radius + 1) as usize];
  let sigma = p_radius as f32 / 2.0;
  let pi = std::f32::consts::PI;

  // Fill kernel symmetrically
  for x in 0..=p_radius {
    let value = (-(x as f32 * x as f32) / (2.0 * sigma * sigma)).exp() / (2.0 * pi * sigma * sigma);
    kernel[p_radius as usize + x as usize] = value;
    kernel[p_radius as usize - x as usize] = value;
  }

  // Compute full kernel sum and normalize so the kernel sums to 1.0.
  let sum = kernel.iter().copied().sum::<f32>();
  if sum > 0.0 {
    kernel.iter_mut().for_each(|value| *value /= sum);
  }

  kernel
}

/// One pass of the blur along a line of `p_length` pixels: each output is the weights times the pixels around it,
/// rounded. `p_at` gives the index of the pixel `offset` steps from the one being computed, clamped to the line.
fn blur_line(p_source: &[u8], p_weights: &[u32], p_index: impl Fn(i32) -> usize, p_length: usize, p_out: &mut [u8]) {
  let radius = (p_weights.len() / 2) as i32;
  for position in 0..p_length {
    let mut sums = [0u32; 4];
    for (offset, &weight) in (-radius..=radius).zip(p_weights) {
      let neighbour = (position as i32 + offset).clamp(0, p_length as i32 - 1);
      let source = p_index(neighbour) * 4;
      for channel in 0..4 {
        sums[channel] += p_source[source + channel] as u32 * weight;
      }
    }
    for channel in 0..4 {
      p_out[position * 4 + channel] = ((sums[channel] + 32768) >> 16) as u8;
    }
  }
}

/// Applies a Gaussian blur to an image using separable convolution.
/// Uses two passes: horizontal and vertical for O(r) complexity instead of O(r²). The math is whole numbers (see
/// [`gaussian_weights`]) and `gaussian.wgsl` does exactly the same steps.
/// * `p_pixels` - The RGBA pixels of the image to be blurred.
/// * `p_radius` - The radius of the Gaussian kernel.
fn separable_gaussian_blur_pixels(p_pixels: &[u8], p_width: usize, p_height: usize, p_radius: u32) -> Vec<u8> {
  let weights = gaussian_weights(p_radius);

  // Horizontal pass (parallel per row), then vertical pass reading its result.
  let mut horizontal = vec![0u8; p_width * p_height * 4];
  horizontal.par_chunks_mut(p_width * 4).enumerate().for_each(|(y, row)| {
    blur_line(&p_pixels[y * p_width * 4..(y + 1) * p_width * 4], &weights, |x| x as usize, p_width, row);
  });

  // The vertical pass works down each column, then the columns are put back into rows.
  let mut columns = vec![0u8; p_width * p_height * 4];
  columns.par_chunks_mut(p_height * 4).enumerate().for_each(|(x, column)| {
    blur_line(&horizontal, &weights, |y| y as usize * p_width + x, p_height, column);
  });
  let mut vertical = vec![0u8; p_width * p_height * 4];
  for x in 0..p_width {
    for y in 0..p_height {
      let from = (x * p_height + y) * 4;
      let to = (y * p_width + x) * 4;
      vertical[to..to + 4].copy_from_slice(&columns[from..from + 4]);
    }
  }
  vertical
}
/// Applies a Gaussian blur to an image.
/// - `p_image`: The image to be blurred.
/// - `p_radius`: The radius of the Gaussian kernel.
/// - `p_options`: Additional options for applying the blur.
fn apply_gaussian_blur(p_image: &mut Image, p_radius: u32) {
  if p_radius == 0 {
    return;
  }

  let (width, height) = p_image.dimensions::<u32>();
  let pixels = p_image.to_rgba_vec();

  // For large radii on sufficiently large areas, downsample, blur, then upsample for speed.
  let vertical = if p_radius >= 24 && (width >= 128 || height >= 128) {
    let scale = if_pick!(p_radius >= 96 => 8, p_radius >= 48 => 4, else => 2);
    let down_w = (width / scale).max(1);
    let down_h = (height / scale).max(1);
    let new_radius = (p_radius as f32 / scale as f32).max(1.0).round() as u32;

    let mut tmp_img = Image::new_from_pixels(width, height, pixels.clone(), Channels::RGBA);
    tmp_img.resize(ResizeTarget::Exact(Size::new(down_w, down_h)), None);
    let blurred_small = separable_gaussian_blur_pixels(tmp_img.rgba(), down_w as usize, down_h as usize, new_radius);
    tmp_img.set_rgba(blurred_small);
    tmp_img.resize(ResizeTarget::Exact(Size::new(width, height)), None);
    tmp_img.into_rgba_vec()
  } else {
    separable_gaussian_blur_pixels(&pixels, width as usize, height as usize, p_radius)
  };

  p_image.set_rgba(vertical);
}

#[derive(Clone)]
pub struct GaussianBlur {
  radius: u32,
  options: Options,
}

impl GpuProcessor for GaussianBlur {
  /// A horizontal pass followed by a vertical pass, in the same whole-number math as the CPU, so the pixels are exactly
  /// the same. The one difference is that the CPU downsamples a large radius (24 or more) on a large image (128 pixels
  /// or more on a side) for speed, and the GPU does not, so those two give slightly different pixels.
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    if self.radius == 0 {
      return Vec::new();
    }
    // The weights go to the shader as a table, three bytes each (they never need more than 17 bits).
    let table: Vec<u8> = gaussian_weights(self.radius)
      .iter()
      .flat_map(|&weight| [weight as u8, (weight >> 8) as u8, (weight >> 16) as u8, 255])
      .collect();
    let table: std::sync::Arc<[u8]> = table.into();
    let pass = |direction: u32| {
      let uniforms: Vec<u8> = [self.radius, direction, 0, 0].iter().flat_map(|value| value.to_le_bytes()).collect();
      GpuPass::new(include_str!("./gaussian.wgsl"), uniforms).with_window().with_reach(self.radius as f32).with_aux(GpuAux {
        width: 2 * self.radius + 1,
        height: 1,
        rgba: table.clone(),
      })
    };
    vec![pass(0), pass(1)]
  }
}

impl Effect for GaussianBlur {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    self.radius as i32
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_gaussian_blur(p_image, self.radius);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }

  /// A blur that is half as wide for a picture that is half as wide, rounded to a whole pixel.
  fn for_level(&self, p_level: u32) -> Option<std::sync::Arc<dyn CpuProcessor>> {
    let radius = scaled_to_level(self.radius as f32, p_level).round() as u32;
    Some(std::sync::Arc::new(GaussianBlur { radius, ..self.clone() }))
  }
}

pub fn gaussian_blur(p_radius: impl IntoNumber) -> GaussianBlur {
  GaussianBlur {
    radius: p_radius.into(),
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use options::{ApplyOptions, Effect};

  use super::gaussian_blur;
  use abra_core::{Area, Image, image::gpu::GpuProcessor};

  fn test_pixels(p_width: u32, p_height: u32) -> Vec<u8> {
    let mut pixels = Vec::new();
    for y in 0..p_height {
      for x in 0..p_width {
        // Hard edges and a bright dot, so blurring visibly matters and clamped borders are exercised.
        let value = if (x / 5 + y / 4) % 2 == 0 { 230 } else { 20 };
        pixels.extend_from_slice(&[
          value,
          (x * 7) as u8,
          (y * 9) as u8,
          if x == 3 && y == 3 { 40 } else { 255 },
        ]);
      }
    }
    pixels
  }

  #[test]
  fn gpu_blur_matches_the_cpu_blur() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let (width, height) = (41u32, 27u32);
    let pixels = test_pixels(width, height);

    for radius in [1u32, 2, 5, 12, 23, 40] {
      let expected = super::separable_gaussian_blur_pixels(&pixels, width as usize, height as usize, radius);
      let actual = renderer.process(&[gaussian_blur(radius).passes(width, height)], width, height, &pixels)?;
      assert_eq!(actual.len(), expected.len());
      for (index, (a, e)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(a, e, "radius {radius}, byte {index}: gpu {a} vs cpu {e}");
      }
    }
    Ok(())
  }

  #[test]
  fn the_weights_add_up_to_exactly_65536_and_are_symmetric() {
    for radius in [1u32, 2, 3, 7, 12, 40, 100] {
      let weights = super::gaussian_weights(radius);
      assert_eq!(weights.len() as u32, 2 * radius + 1);
      assert_eq!(weights.iter().map(|&weight| weight as u64).sum::<u64>(), 65536, "radius {radius}");
      let (left, right) = weights.split_at(radius as usize);
      assert!(left.iter().zip(right[1..].iter().rev()).all(|(a, b)| a.abs_diff(*b) <= 1), "radius {radius}");
    }
  }

  #[test]
  fn a_flat_area_stays_exactly_as_it_was() {
    // A float kernel that adds up to a hair under 1 and then truncates darkens flat areas by a level.
    for value in [1u8, 100, 200, 255] {
      let pixels: Vec<u8> = (0..30 * 30).flat_map(|_| [value, value, value, 255]).collect();
      for radius in [1u32, 3, 12] {
        assert_eq!(
          super::separable_gaussian_blur_pixels(&pixels, 30, 30, radius),
          pixels,
          "value {value}, radius {radius}"
        );
      }
    }
  }

  /// Viewing a blurred photo at another zoom gives what blurring the whole photo and then looking at it gives, so a slider
  /// moved while zoomed shows the same blur as at the photo's own size.
  #[test]
  fn a_blur_in_a_window_at_another_zoom_is_the_whole_photo_blurred_then_viewed() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer, TileOptions, TiledRenderer, Window};
    let context = GpuContext::new_default_blocking()?;
    let (width, height) = (200u32, 200u32);
    // A white square in the middle of black: a blur of it is smooth, and wrong radii or places show.
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
      for x in 0..width {
        let at = ((y * width + x) * 4) as usize;
        let inside = (80..120).contains(&x) && (80..120).contains(&y);
        pixels[at..at + 4].copy_from_slice(&[if inside { 255 } else { 0 }, 0, 0, 255]);
      }
    }
    let passes = vec![gaussian_blur(16).passes(width, height)];
    let reference = LiveRenderer::new(context.clone()).process(&passes, width, height, &pixels)?;
    // The whole photo's blur, read between its pixels like the screen would.
    let blurred_at = |x: f32, y: f32| {
      let (x, y) = ((x - 0.5).clamp(0.0, 199.0), (y - 0.5).clamp(0.0, 199.0));
      let (x0, y0) = (x.floor() as usize, y.floor() as usize);
      let (x1, y1) = ((x0 + 1).min(199), (y0 + 1).min(199));
      let (fx, fy) = (x - x0 as f32, y - y0 as f32);
      let at = |x: usize, y: usize| reference[(y * 200 + x) * 4] as f32;
      let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
      let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
      top * (1.0 - fy) + bottom * fy
    };

    let mut photo = TiledRenderer::with_options(context, width, height, pixels.into(), TileOptions::default().with_tile_size(64))?;
    for out in [100u32, 400] {
      let scale = 200.0 / out as f32;
      let window = Window { x: 0.0, y: 0.0, width: 200.0, height: 200.0, out_width: out, out_height: out, pad: 0, angle: 0.0 };
      photo.render(&window, &passes)?;
      let drawn = photo.read_pixels()?;
      let side = ((drawn.len() / 4) as f32).sqrt() as u32;
      let pad = (side - out) / 2;
      assert!(pad as f32 * scale >= 15.0, "the blur's reach is padded for: {pad} pixels at {scale} photo pixels each");
      let mut worst = 0.0f32;
      for y in 0..out {
        for x in 0..out {
          let got = drawn[(((y + pad) * side + x + pad) * 4) as usize] as f32;
          worst = worst.max((got - blurred_at((x as f32 + 0.5) * scale, (y as f32 + 0.5) * scale)).abs());
        }
      }
      assert!(worst <= 8.0, "at {out} pixels across the blur is off by {worst}");
    }
    Ok(())
  }

  #[test]
  fn gpu_blur_radius_zero_is_a_no_op() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let pixels = test_pixels(9, 9);
    assert_eq!(renderer.process(&[gaussian_blur(0).passes(9, 9)], 9, 9, &pixels)?, pixels);
    Ok(())
  }

  #[test]
  fn gaussian_blur_without_area_does_not_panic() {
    let mut img = Image::new(8, 8);
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }

    gaussian_blur(2).apply(&mut img);

    assert_eq!(img.dimensions::<u32>(), (8, 8));
  }

  #[test]
  fn gaussian_blur_area_writes_back_only_area() {
    let mut img = Image::new(8, 8);
    // Single bright pixel in center
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 0u8, 0u8, 255));
    // Snapshot original values
    let orig = img.to_rgba_vec();

    // Apply blur to center 4x4 area (white pixel should spread)
    gaussian_blur(2).with_options(ApplyOptions::new().with_area(Area::rect((2.0, 2.0), (4.0, 4.0)))).apply(&mut img);

    // Ensure dimensions unchanged
    assert_eq!(img.dimensions::<u32>(), (8, 8));

    // Check outside area unchanged
    let mut changed_count = 0usize;
    // Diagnostic code removed
    for y in 0..8u32 {
      for x in 0..8u32 {
        let idx = ((y * 8 + x) * 4) as usize;
        if x < 2 || x >= 6 || y < 2 || y >= 6 {
          // Outside area should remain unchanged
          assert_eq!(img.rgba()[idx], orig[idx]);
          assert_eq!(img.rgba()[idx + 1], orig[idx + 1]);
          assert_eq!(img.rgba()[idx + 2], orig[idx + 2]);
          assert_eq!(img.rgba()[idx + 3], orig[idx + 3]);
        } else {
          // Count changes in blurred region; at least one pixel should differ
          if img.rgba()[idx] != orig[idx]
            || img.rgba()[idx + 1] != orig[idx + 1]
            || img.rgba()[idx + 2] != orig[idx + 2]
            || img.rgba()[idx + 3] != orig[idx + 3]
          {
            changed_count += 1;
          }
        }
      }
    }
    // debug removed
    assert!(changed_count > 0, "No pixels in the blurred area changed");
  }

  #[test]
  fn separable_blur_changes_pixels() {
    let mut img = Image::new(8, 8);
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 0u8, 0u8, 255));
    let pixels = img.to_rgba_vec();
    let out = super::separable_gaussian_blur_pixels(&pixels, 8, 8, 2);
    // Ensure center changed
    let idx = ((2 * 8 + 2) * 4) as usize;
    assert!(out[idx] != pixels[idx] || out[idx + 1] != pixels[idx + 1] || out[idx + 2] != pixels[idx + 2]);
  }

  #[test]
  fn horizontal_pass_changes_pixels() {
    let mut img = Image::new(8, 8);
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 0u8, 0u8, 255));
    let pixels = img.to_rgba_vec();
    let kernel = super::gaussian_kernel_1d(2);
    let width = 8usize;
    let y = 3usize;
    let mut horiz = vec![0u8; width * 4];
    let kernel_radius = 2i32;
    for x in 0..width {
      let mut r = 0.0f32;
      let mut g = 0.0f32;
      let mut b = 0.0f32;
      let mut a = 0.0f32;
      for kx in -kernel_radius..=kernel_radius {
        let px = (x as i32 + kx).clamp(0, width as i32 - 1) as usize;
        let src_idx = (y * width + px) * 4;
        let weight = kernel[(kx + kernel_radius) as usize];
        r += pixels[src_idx] as f32 * weight;
        g += pixels[src_idx + 1] as f32 * weight;
        b += pixels[src_idx + 2] as f32 * weight;
        a += pixels[src_idx + 3] as f32 * weight;
      }
      let rr = r.clamp(0.0, 255.0) as u8;
      let gg = g.clamp(0.0, 255.0) as u8;
      let bb = b.clamp(0.0, 255.0) as u8;
      let aa = a.clamp(0.0, 255.0) as u8;
      let off = x * 4;
      horiz[off] = rr;
      horiz[off + 1] = gg;
      horiz[off + 2] = bb;
      horiz[off + 3] = aa;
    }
    let idx = (3 * 4) as usize;
    assert!(horiz[idx] != pixels[idx] || horiz[idx + 1] != pixels[idx + 1] || horiz[idx + 2] != pixels[idx + 2]);
  }

  #[test]
  fn vertical_pass_changes_pixels() {
    let mut img = Image::new(8, 8);
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 0u8, 0u8, 255));
    let pixels = img.to_rgba_vec();
    let kernel = super::gaussian_kernel_1d(2);
    let width = 8usize;
    let height = 8usize;
    let kernel_radius = 2i32;
    // Horizontal
    let mut horizontal = vec![0u8; width * height * 4];
    for y in 0..height {
      for x in 0..width {
        let mut r = 0.0f32;
        let mut g = 0.0f32;
        let mut b = 0.0f32;
        let mut a = 0.0f32;
        for kx in -kernel_radius..=kernel_radius {
          let px = (x as i32 + kx).clamp(0, width as i32 - 1) as usize;
          let src_idx = (y * width + px) * 4;
          let weight = kernel[(kx + kernel_radius) as usize];
          r += pixels[src_idx] as f32 * weight;
          g += pixels[src_idx + 1] as f32 * weight;
          b += pixels[src_idx + 2] as f32 * weight;
          a += pixels[src_idx + 3] as f32 * weight;
        }
        let idx = (y * width + x) * 4;
        horizontal[idx] = r.clamp(0.0, 255.0) as u8;
        horizontal[idx + 1] = g.clamp(0.0, 255.0) as u8;
        horizontal[idx + 2] = b.clamp(0.0, 255.0) as u8;
        horizontal[idx + 3] = a.clamp(0.0, 255.0) as u8;
      }
    }
    // Vertical
    let mut vertical = vec![0u8; width * height * 4];
    for y in 0..height {
      for x in 0..width {
        let mut r = 0.0f32;
        let mut g = 0.0f32;
        let mut b = 0.0f32;
        let mut a = 0.0f32;
        for ky in -kernel_radius..=kernel_radius {
          let py = (y as i32 + ky).clamp(0, height as i32 - 1) as usize;
          let src_idx = (py * width + x) * 4;
          let weight = kernel[(ky + kernel_radius) as usize];
          r += horizontal[src_idx] as f32 * weight;
          g += horizontal[src_idx + 1] as f32 * weight;
          b += horizontal[src_idx + 2] as f32 * weight;
          a += horizontal[src_idx + 3] as f32 * weight;
        }
        let idx = (y * width + x) * 4;
        vertical[idx] = r.clamp(0.0, 255.0) as u8;
        vertical[idx + 1] = g.clamp(0.0, 255.0) as u8;
        vertical[idx + 2] = b.clamp(0.0, 255.0) as u8;
        vertical[idx + 3] = a.clamp(0.0, 255.0) as u8;
      }
    }
    let idx = ((2 * 8 + 2) * 4) as usize;
    assert!(
      vertical[idx] != pixels[idx] || vertical[idx + 1] != pixels[idx + 1] || vertical[idx + 2] != pixels[idx + 2]
    );
  }
  /// A `p_width` x `p_height` image with detail in every channel, so a blur that read the wrong pixels shows.
  fn busy_image(p_width: u32, p_height: u32) -> Image {
    let mut pixels = Vec::new();
    for y in 0..p_height {
      for x in 0..p_width {
        pixels.extend_from_slice(&[(x * 37 + y * 11) as u8, (x * 5 ^ y * 29) as u8, (x * y) as u8, 255]);
      }
    }
    Image::new_from_pixels(p_width, p_height, pixels, abra_core::Channels::RGBA)
  }

  #[test]
  fn a_part_of_a_deferred_blur_is_the_same_as_the_blur_of_the_whole_there_even_at_the_edges() {
    use abra_core::image::image_ext::ImageExt;
    let image = busy_image(64, 48);
    let mut whole = image.clone();
    gaussian_blur(5).apply_on_cpu(&mut whole);
    let mut later = image.clone();
    gaussian_blur(5).apply(options::deferred(&mut later));
    assert!(later.deferred().is_some());
    for (left, top, width, height) in [(20, 10, 16, 12), (0, 0, 9, 9), (55, 40, 9, 8), (-3, 20, 8, 6), (60, 44, 10, 10)] {
      assert_eq!(
        later.region(0, left, top, width, height),
        whole.region(0, left, top, width, height),
        "the part at {left},{top} {width}x{height}"
      );
    }
    assert!(later.deferred().is_some(), "only the parts were made, not the picture");
  }

  #[test]
  fn a_zoomed_out_part_of_a_deferred_blur_is_a_narrower_blur_of_the_zoomed_out_picture() {
    use abra_core::image::image_ext::ImageExt;
    let image = busy_image(64, 48);
    let mut later = image.clone();
    gaussian_blur(8).apply(options::deferred(&mut later));
    // A picture a quarter as wide is blurred a quarter as much: radius 2.
    let quarter = image.region(2, 0, 0, 16, 12);
    let mut expected = Image::new_from_pixels(16, 12, quarter, abra_core::Channels::RGBA);
    gaussian_blur(2).apply_on_cpu(&mut expected);
    assert_eq!(later.region(2, 3, 2, 8, 6), expected.region(0, 3, 2, 8, 6));
    // And a radius that rounds away at that zoom leaves the picture as it is.
    let mut small = image.clone();
    gaussian_blur(1).apply(options::deferred(&mut small));
    assert_eq!(small.region(2, 0, 0, 16, 12), image.region(2, 0, 0, 16, 12));
  }
}
