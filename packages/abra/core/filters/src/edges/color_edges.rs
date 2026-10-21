use crate::common::*;
use abra_core::Color;
use abra_core::image::gpu::{GpuPass, GpuProcessor};

fn apply_color_edges(p_image: &mut Image, p_thickness: u32, p_tolerance: u32, p_ink: [u8; 3], p_paper: [u8; 3]) {
  let (width, height) = p_image.dimensions::<usize>();
  if width == 0 || height == 0 {
    return;
  }
  let source = p_image.rgba();
  let thickness = p_thickness.max(1) as usize;
  // The sum of the three differences is three times their mean, so the tolerance is three times as much.
  let limit = p_tolerance * 3;
  let difference = |first: usize, second: usize| -> u32 {
    (0..3).map(|channel| (source[first * 4 + channel] as i32 - source[second * 4 + channel] as i32).unsigned_abs()).sum()
  };
  let mut out = vec![0u8; width * height * 4];
  out.par_chunks_mut(4).enumerate().for_each(|(index, pixel)| {
    let (x, y) = (index % width, index / width);
    let left = y * width + x.saturating_sub(thickness);
    let above = y.saturating_sub(thickness) * width + x;
    let edge = difference(index, left).max(difference(index, above)) > limit;
    pixel[..3].copy_from_slice(if edge { &p_ink } else { &p_paper });
    pixel[3] = source[index * 4 + 3];
  });
  p_image.set_rgba(out);
}

/// Draws a line wherever the color of an image changes, as the outline of a comic book is drawn between the flat areas of color: ink
/// where a pixel's color differs from that of the pixel a thickness to the left of it, or the one a thickness above it, and paper
/// everywhere else. Create one with [`color_edges`].
///
/// A pixel is on a line when the mean of the differences of its red, green and blue levels, from `0` to `255`, is more than the
/// tolerance. The lines are as wide as the thickness and sit on the right of an upright change and below a flat one. It looks at the
/// color and not only the brightness, so a red next to a green of the same brightness is an edge, which a [`sobel`](crate::sobel)
/// of the brightness would not find. The ink is black and the paper white unless set, so the result can be multiplied over the picture
/// to outline it. The alpha of the image is kept.
#[derive(Clone)]
pub struct ColorEdges {
  thickness: u32,
  tolerance: u32,
  ink: Color,
  paper: Color,
  options: Options,
}

impl ColorEdges {
  /// How different two colors must be to make an edge: the mean difference of their red, green and blue levels, from `0` to `255`.
  /// Defaults to 16, which is clearly different. A flat-color picture, where an edge is any change at all, wants a small one such
  /// as `1`; a photograph, where grain makes small changes everywhere, wants a larger one.
  pub fn with_tolerance(mut self, p_tolerance: u8) -> Self {
    self.tolerance = p_tolerance as u32;
    self
  }

  /// The color of the lines. Defaults to black. Its alpha is not used.
  pub fn with_ink(mut self, p_ink: Color) -> Self {
    self.ink = p_ink;
    self
  }

  /// The color everywhere else. Defaults to white. Its alpha is not used.
  pub fn with_paper(mut self, p_paper: Color) -> Self {
    self.paper = p_paper;
    self
  }

  fn rgb(p_color: &Color) -> [u8; 3] {
    let (red, green, blue) = p_color.rgb();
    [red, green, blue]
  }
}

impl GpuProcessor for ColorEdges {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    let mut uniforms = Vec::with_capacity(48);
    uniforms.extend_from_slice(&self.thickness.max(1).to_le_bytes());
    uniforms.extend_from_slice(&self.tolerance.to_le_bytes());
    uniforms.extend_from_slice(&0u32.to_le_bytes());
    uniforms.extend_from_slice(&0u32.to_le_bytes());
    for color in [&self.ink, &self.paper] {
      let [red, green, blue] = Self::rgb(color);
      for value in [red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0, 1.0] {
        uniforms.extend_from_slice(&value.to_le_bytes());
      }
    }
    vec![GpuPass::new(include_str!("color_edges.wgsl"), uniforms).with_reach(self.thickness.max(1) as f32)]
  }
}

impl Effect for ColorEdges {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  /// A pixel is compared with one a thickness away.
  fn padding(&self) -> i32 {
    self.thickness.max(1) as i32
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_color_edges(p_image, self.thickness, self.tolerance, Self::rgb(&self.ink), Self::rgb(&self.paper));
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

/// Lines where the color of the image changes, as ink on paper.
/// - `p_thickness`: How wide the lines are in pixels, and how far away the pixel is that each one is compared with. `0` is taken as 1.
pub fn color_edges(p_thickness: u32) -> ColorEdges {
  ColorEdges {
    thickness: p_thickness,
    tolerance: 16,
    ink: Color::black(),
    paper: Color::white(),
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;

  /// Left half red and right half blue, 20 pixels wide, with an optional lighter bottom half.
  fn two_colors() -> Image {
    let mut pixels = Vec::new();
    for _y in 0..10 {
      for x in 0..20 {
        pixels.extend_from_slice(if x < 10 { &[200, 20, 20, 255] } else { &[20, 20, 200, 255] });
      }
    }
    Image::new_from_pixels(20, 10, pixels, Channels::RGBA)
  }

  fn is_ink(p_image: &Image, p_x: u32, p_y: u32) -> bool {
    p_image.rgba()[((p_y * 20 + p_x) * 4) as usize] == 0
  }

  #[test]
  fn a_line_as_wide_as_the_thickness_is_drawn_where_the_color_changes() {
    let mut image = two_colors();
    color_edges(3).apply_on_cpu(&mut image);
    let inked: Vec<u32> = (0..20).filter(|&x| is_ink(&image, x, 5)).collect();
    // The change is between columns 9 and 10, and the line sits on the right of it.
    assert_eq!(inked, vec![10, 11, 12]);
    // Every row has the same line, and nothing else is ink.
    assert!((0..10).all(|y| (0..20).filter(|&x| is_ink(&image, x, y)).count() == 3));
  }

  #[test]
  fn a_flat_picture_has_no_edges_and_a_horizontal_change_is_found_too() {
    let gray: Vec<u8> = (0..64).flat_map(|_| [90, 90, 90, 255]).collect();
    let mut flat = Image::new_from_pixels(8, 8, gray, Channels::RGBA);
    color_edges(2).apply_on_cpu(&mut flat);
    assert!(flat.rgba().chunks_exact(4).all(|pixel| pixel[..3] == [255, 255, 255]));
    // Dark on top and light below: the line is on the first rows of the light part.
    let rows: Vec<u8> = (0..8).flat_map(|y| (0..8).flat_map(move |_| if y < 4 { [10, 10, 10, 255] } else { [240, 240, 240, 255] })).collect();
    let mut stripes = Image::new_from_pixels(8, 8, rows, Channels::RGBA);
    color_edges(2).apply_on_cpu(&mut stripes);
    let inked: Vec<u32> = (0..8).filter(|&y| stripes.rgba()[((y * 8) * 4) as usize] == 0).collect();
    assert_eq!(inked, vec![4, 5]);
  }

  #[test]
  fn a_change_smaller_than_the_tolerance_is_not_an_edge_and_the_colors_and_alpha_are_used() {
    let pixels: Vec<u8> = (0..16).flat_map(|x| if x < 8 { [100, 100, 100, 77] } else { [110, 110, 110, 77] }).collect();
    let mut image = Image::new_from_pixels(16, 1, pixels.clone(), Channels::RGBA);
    // A difference of 10 levels is under the tolerance of 16, so there is no line.
    color_edges(1).apply_on_cpu(&mut image);
    assert!(image.rgba().chunks_exact(4).all(|pixel| pixel[..3] == [255, 255, 255]));
    // With a tolerance of 5 it is a line, in the ink on the paper, and the alpha is the picture's.
    let mut image = Image::new_from_pixels(16, 1, pixels, Channels::RGBA);
    color_edges(1).with_tolerance(5).with_ink(Color::from_rgb(255, 0, 0)).with_paper(Color::from_rgb(0, 0, 255)).apply_on_cpu(&mut image);
    let rgba = image.rgba();
    assert_eq!(rgba[8 * 4..8 * 4 + 4], [255, 0, 0, 77]);
    assert_eq!(rgba[7 * 4..7 * 4 + 4], [0, 0, 255, 77]);
  }

  #[test]
  fn a_change_of_color_with_the_same_brightness_is_an_edge() {
    // A red and a green of about the same brightness.
    let pixels: Vec<u8> = (0..16).flat_map(|x| if x < 8 { [200, 0, 0, 255] } else { [0, 100, 0, 255] }).collect();
    let mut image = Image::new_from_pixels(16, 1, pixels, Channels::RGBA);
    color_edges(1).apply_on_cpu(&mut image);
    assert_eq!(image.rgba()[8 * 4], 0);
  }

  #[test]
  fn the_gpu_draws_the_same_lines_as_the_cpu() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};

    let (width, height) = (47u32, 31u32);
    let pixels: Vec<u8> = (0..width * height)
      .flat_map(|index| {
        let (x, y) = (index % width, index / width);
        let band = ((x / 7 + y / 5) % 3) as u8;
        [band * 90 + (x % 3) as u8 * 6, 40 + band * 30, 200 - band * 80 + (y % 4) as u8 * 5, 255 - (x % 5) as u8]
      })
      .collect();
    let effect = color_edges(2).with_tolerance(12).with_ink(Color::from_rgb(10, 20, 30)).with_paper(Color::from_rgb(250, 240, 230));
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let gpu = renderer.process(&[effect.passes(width, height)], width, height, &pixels)?;
    let mut cpu = Image::new_from_pixels(width, height, pixels, Channels::RGBA);
    effect.apply_on_cpu(&mut cpu);
    assert_eq!(gpu, cpu.to_rgba_vec());
    Ok(())
  }
}
