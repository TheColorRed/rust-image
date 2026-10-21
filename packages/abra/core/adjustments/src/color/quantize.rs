use abra_core::{
  Color, Image,
  image::gpu::{GpuPass, GpuProcessor},
};
use options::{Effect, Options};
use primitives::color::{Palette, lab_to_rgb, rgb_to_lab};
use rayon::prelude::*;
use std::sync::Arc;

/// A color in the Lab color space, where the distance between two colors is how different they look.
type Lab = [f32; 3];

/// The most colors a palette can have and still be painted on the GPU (the size of the table in `quantize.wgsl`). A longer palette is
/// painted on the CPU.
const MAX_GPU_COLORS: usize = 64;

fn lab_of(p_pixel: &[u8]) -> Lab {
  let (l, a, b) = rgb_to_lab(p_pixel[0], p_pixel[1], p_pixel[2]);
  [l, a, b]
}

/// Picks the starting centers: the sample nearest to the average color first, then again and again the sample that is farthest from
/// every center so far. It always gives the same centers for the same sample.
fn starting_centers(p_sample: &[Lab], p_colors: usize) -> Vec<Lab> {
  let mut sum = [0.0f64; 3];
  for color in p_sample {
    for channel in 0..3 {
      sum[channel] += color[channel] as f64;
    }
  }
  let count = p_sample.len() as f64;
  let mean = [(sum[0] / count) as f32, (sum[1] / count) as f32, (sum[2] / count) as f32];
  let mut centers = vec![p_sample[Palette::nearest_in(p_sample, &mean).unwrap_or(0)]];
  let mut gap: Vec<f32> = p_sample.iter().map(|color| Palette::distance(color, &centers[0])).collect();
  while centers.len() < p_colors {
    let (farthest, _) = gap.iter().enumerate().fold((0, -1.0f32), |best, (index, d)| if *d > best.1 { (index, *d) } else { best });
    let center = p_sample[farthest];
    for (index, color) in p_sample.iter().enumerate() {
      gap[index] = gap[index].min(Palette::distance(color, &center));
    }
    centers.push(center);
  }
  centers
}

/// Merges the two centers nearest to each other into one, again and again, while they are closer than `p_tolerance`. The merged center
/// is the average of the two, each counted as many times as it has colors, so the color that most pixels had stays nearest to itself.
/// How close two centers are is the distance between their Lab values with the lightness counted `p_lightness` times as much.
fn merge_close(p_centers: &mut Vec<Lab>, p_counts: &mut Vec<usize>, p_tolerance: f32, p_lightness: f32) {
  if p_tolerance <= 0.0 {
    return;
  }
  let limit = p_tolerance * p_tolerance;
  let weighed = |lab: &Lab| [lab[0] * p_lightness, lab[1], lab[2]];
  loop {
    let mut closest: Option<(usize, usize, f32)> = None;
    for first in 0..p_centers.len() {
      for second in first + 1..p_centers.len() {
        let d = Palette::distance(&weighed(&p_centers[first]), &weighed(&p_centers[second]));
        if d < limit && closest.is_none_or(|(_, _, best)| d < best) {
          closest = Some((first, second, d));
        }
      }
    }
    let Some((first, second, _)) = closest else { break };
    let (count_first, count_second) = (p_counts[first].max(1) as f32, p_counts[second].max(1) as f32);
    for channel in 0..3 {
      p_centers[first][channel] =
        (p_centers[first][channel] * count_first + p_centers[second][channel] * count_second) / (count_first + count_second);
    }
    p_counts[first] += p_counts[second];
    p_centers.remove(second);
    p_counts.remove(second);
  }
}

/// Works out the average colors of the pixels, from every `p_stride`th pixel, as RGB: `p_colors` of them, or fewer where two of them
/// are nearer to each other than `p_tolerance` and so are merged into one.
fn find_palette(
  p_pixels: &[u8], p_colors: usize, p_iterations: u32, p_stride: usize, p_tolerance: f32, p_lightness: f32,
) -> Vec<[u8; 3]> {
  let sample: Vec<Lab> = p_pixels.chunks_exact(4).step_by(p_stride).map(lab_of).collect();
  if sample.is_empty() {
    return Vec::new();
  }
  let mut centers = starting_centers(&sample, p_colors);
  for _ in 0..p_iterations {
    let mut sums = vec![[0.0f64; 3]; centers.len()];
    let mut counts = vec![0usize; centers.len()];
    for color in &sample {
      let index = Palette::nearest_in(&centers, color).unwrap_or(0);
      counts[index] += 1;
      for channel in 0..3 {
        sums[index][channel] += color[channel] as f64;
      }
    }
    let mut moved = false;
    for (index, center) in centers.iter_mut().enumerate() {
      if counts[index] == 0 {
        continue;
      }
      let count = counts[index] as f64;
      let next = [(sums[index][0] / count) as f32, (sums[index][1] / count) as f32, (sums[index][2] / count) as f32];
      if Palette::distance(center, &next) > 1e-6 {
        moved = true;
      }
      *center = next;
    }
    if !moved {
      break;
    }
  }
  let mut counts = vec![0usize; centers.len()];
  for color in &sample {
    counts[Palette::nearest_in(&centers, color).unwrap_or(0)] += 1;
  }
  merge_close(&mut centers, &mut counts, p_tolerance, p_lightness);
  centers
    .iter()
    .map(|center| {
      let (red, green, blue) = lab_to_rgb(center[0], center[1], center[2]);
      [red, green, blue]
    })
    .collect()
}

/// A palette of RGB colors.
fn palette_of_rgb(p_colors: &[[u8; 3]]) -> Palette {
  Palette::new(p_colors.iter().map(|color| Color::from_rgb(color[0], color[1], color[2])))
}

/// Paints every pixel with the color of the palette that goes with the one of `p_matching` nearest to it, keeping the alpha. The two
/// have as many colors, and are the same list when the pixels take the palette color nearest to themselves. The lightness of each color
/// counts `p_lightness` times as much when finding the nearest.
fn paint(p_pixels: &mut [u8], p_palette: &[[u8; 3]], p_matching: &[[u8; 3]], p_lightness: f32) {
  if p_palette.is_empty() {
    return;
  }
  let labs: Vec<Lab> = palette_of_rgb(p_matching).labs().iter().map(|lab| [lab[0] * p_lightness, lab[1], lab[2]]).collect();
  p_pixels.par_chunks_mut(4).for_each(|pixel| {
    let lab = lab_of(pixel);
    let color = p_palette[Palette::nearest_in(&labs, &[lab[0] * p_lightness, lab[1], lab[2]]).unwrap_or(0)];
    pixel[0] = color[0];
    pixel[1] = color[1];
    pixel[2] = color[2];
  });
}

/// Merges the colors of an image into a few. The image's colors are grouped by how alike they look (in the Lab color space, so that
/// "alike" matches what the eye sees), each group's average color is worked out, and every pixel is painted with the average color
/// of its group. Colors that were close become exactly one color, and the groups are found from the picture itself, so a photo of
/// a red apple on a green table keeps a red and a green. The alpha is left alone. Create one with [`quantize`].
///
/// The groups are found by k-means on a sample of the pixels: it starts from the average color and from colors far from it, then moves
/// each group's center to the average of the colors nearest to it, until the centers stop moving or the iterations run out. The same
/// image always gives the same colors. Finding the groups looks at the whole image, so it is done on the CPU and cannot be worked
/// out one part at a time.
///
/// With [`Quantize::with_palette`] the colors are given and nothing is found: every pixel is painted with the nearest of them. That
/// needs only the pixel itself, so it can run on the GPU (`quantize.wgsl`, for a palette of up to 32 colors) and on one part of an
/// image at a time. To find the colors once and paint with them on the GPU, get them with [`Quantize::palette`].
///
/// [`Quantize::with_sources`] changes which palette color a pixel takes: the palette color that goes with the source color nearest
/// to the pixel. Give it the colors found in the image and a palette of what each should become, and every color of the picture is
/// replaced by its own new color, while the pixels between two colors go to whichever is nearer.
///
/// The GPU version works out the Lab values in 32-bit floats as the CPU does, so it can differ from the CPU for a color that is
/// almost exactly as near to two palette colors.
#[derive(Clone)]
pub struct Quantize {
  colors: u32,
  palette: Option<Vec<[u8; 3]>>,
  sources: Option<Vec<[u8; 3]>>,
  iterations: u32,
  sample: u32,
  tolerance: f32,
  lightness: f32,
  options: Options,
}

impl Quantize {
  /// How different two of the found colors may be, as the distance between them in the Lab color space (about 2 is the least a
  /// person can see, 10 is clearly different and 100 is black against white), and still be merged into one color. Colors that
  /// are closer than this are merged, nearest first, into the average of the two (each counted as many times as it has pixels), so
  /// the number of colors given to [`quantize`] is the most there will be. Defaults to 0, which merges nothing. It does not apply to
  /// a palette given with [`Quantize::with_palette`].
  pub fn with_tolerance(mut self, p_distance: f32) -> Self {
    self.tolerance = p_distance;
    self
  }

  /// How much a difference in brightness counts, against a difference in color, when deciding how alike two colors are: both when
  /// found colors are merged by [`Quantize::with_tolerance`] and when each pixel takes the nearest color. Defaults to 1, the Lab
  /// distance as it is. A lower weight makes the light and the shade of one hue look more alike, such as the lit and the shaded part of
  /// one surface, and a gray look nearer to another gray than to a tan of the same brightness. Black and white stay apart while the
  /// weight is not far below 1. It is not used to find the groups.
  pub fn with_lightness(mut self, p_weight: f32) -> Self {
    self.lightness = p_weight;
    self
  }

  /// The most times the groups' centers are moved to the average of their colors. Defaults to 8.
  pub fn with_iterations(mut self, p_iterations: u32) -> Self {
    self.iterations = p_iterations;
    self
  }

  /// About how many pixels are looked at to find the groups, spread evenly over the image: more is slower and finds the groups more
  /// exactly. Every pixel is still painted. Defaults to 20000.
  pub fn with_sample(mut self, p_pixels: u32) -> Self {
    self.sample = p_pixels;
    self
  }

  /// Paints with these colors instead of finding them from the image: the number of colors given to [`quantize`] is not used.
  pub fn with_palette(mut self, p_palette: impl IntoIterator<Item = Color>) -> Self {
    self.palette = Some(
      p_palette
        .into_iter()
        .map(|color| {
          let (red, green, blue) = color.rgb();
          [red, green, blue]
        })
        .collect(),
    );
    self
  }

  /// The colors that decide which palette color a pixel takes, one for each color of the palette given with
  /// [`Quantize::with_palette`]: a pixel takes the palette color that goes with the source color nearest to it. Without them a pixel
  /// takes the palette color nearest to itself. With a different number of colors than the palette they are not used.
  pub fn with_sources(mut self, p_sources: impl IntoIterator<Item = Color>) -> Self {
    self.sources = Some(
      p_sources
        .into_iter()
        .map(|color| {
          let (red, green, blue) = color.rgb();
          [red, green, blue]
        })
        .collect(),
    );
    self
  }

  /// The colors the pixels of `p_image` are painted with: the palette given with [`Quantize::with_palette`], or else the average
  /// colors found from the image.
  pub fn palette(&self, p_image: &Image) -> Vec<Color> {
    self.palette_of(&p_image.rgba()).into_iter().map(|color| Color::from_rgb(color[0], color[1], color[2])).collect()
  }

  /// The colors a pixel is compared with, which go with the palette colors one by one: the sources when there are as many as the
  /// palette has colors, and otherwise the palette itself.
  fn matching_of<'a>(&'a self, p_palette: &'a [[u8; 3]]) -> &'a [[u8; 3]] {
    match &self.sources {
      Some(sources) if sources.len() == p_palette.len() => sources,
      _ => p_palette,
    }
  }

  fn palette_of(&self, p_pixels: &[u8]) -> Vec<[u8; 3]> {
    if let Some(palette) = &self.palette {
      return palette.clone();
    }
    let total = p_pixels.len() / 4;
    let stride = (total / self.sample.max(1) as usize).max(1);
    find_palette(p_pixels, self.colors.max(1) as usize, self.iterations, stride, self.tolerance, self.lightness)
  }
}

impl Effect for Quantize {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    0
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let pixels = p_image.colors().as_slice_mut().expect("Image colors must be contiguous");
    let palette = self.palette_of(pixels);
    paint(pixels, &palette, self.matching_of(&palette), self.lightness);
  }

  /// Only a palette that is given can be painted by the GPU: finding one needs the pixels.
  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    match &self.palette {
      Some(palette) if !palette.is_empty() && palette.len() <= MAX_GPU_COLORS => Some(self),
      _ => None,
    }
  }

  /// With a palette each pixel is painted on its own, so for a picture of any size it is the same.
  fn for_level(&self, _p_level: u32) -> Option<Arc<dyn abra_core::image::gpu::CpuProcessor>> {
    self.palette.as_ref().map(|_| Arc::new(self.clone()) as Arc<dyn abra_core::image::gpu::CpuProcessor>)
  }
}

impl GpuProcessor for Quantize {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    let palette = self.palette.as_deref().unwrap_or(&[]);
    let palette = &palette[..palette.len().min(MAX_GPU_COLORS)];
    let matching = palette_of_rgb(&self.matching_of(palette)[..palette.len()]);
    let labs = matching.labs();
    // The table of the shader: the count and the weight of the lightness padded to 16 bytes, then the Lab values of each color a pixel
    // is compared with (the lightness already weighted) and then the RGB of the palette color that goes with it, each as four floats.
    let mut bytes = Vec::with_capacity(16 + MAX_GPU_COLORS * 32);
    bytes.extend_from_slice(&(palette.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&self.lightness.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 8]);
    for slot in 0..MAX_GPU_COLORS {
      let lab = labs.get(slot).copied().unwrap_or([0.0; 3]);
      for value in [lab[0] * self.lightness, lab[1], lab[2], 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
      }
    }
    for slot in 0..MAX_GPU_COLORS {
      let rgb = palette.get(slot).copied().unwrap_or([0; 3]);
      for value in [rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0, 1.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
      }
    }
    vec![GpuPass::new(include_str!("quantize.wgsl"), bytes)]
  }
}

/// A merge of the image's colors into `p_colors` average colors. A `p_colors` of 0 is taken as 1, which paints the whole image its
/// average color.
pub fn quantize(p_colors: u32) -> Quantize {
  Quantize {
    colors: p_colors,
    palette: None,
    sources: None,
    iterations: 8,
    sample: 20000,
    tolerance: 0.0,
    lightness: 1.0,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;
  use std::collections::{HashMap, HashSet};

  /// A picture of three kinds of color, each with a little variation, in the order red, green, blue, row by row.
  fn three_kinds() -> Vec<u8> {
    let kinds = [[200u8, 30, 30], [30, 190, 40], [40, 30, 210]];
    let mut pixels = Vec::new();
    for at in 0..90usize {
      let kind = kinds[at % 3];
      let wobble = (at % 7) as u8;
      pixels.extend_from_slice(&[kind[0] + wobble, kind[1] + wobble, kind[2].saturating_sub(wobble), 255]);
    }
    pixels
  }

  fn distinct(p_rgba: &[u8]) -> HashSet<[u8; 3]> {
    p_rgba.chunks_exact(4).map(|pixel| [pixel[0], pixel[1], pixel[2]]).collect()
  }

  #[test]
  fn colors_that_are_alike_become_one_color() {
    let mut image = Image::new_from_pixels(90, 1, three_kinds(), Channels::RGBA);
    quantize(3).apply_on_cpu(&mut image);
    let out = image.to_rgba_vec();
    assert_eq!(distinct(&out).len(), 3);
    // Every pixel of one kind got the same color.
    let mut by_kind: HashMap<usize, HashSet<[u8; 3]>> = HashMap::new();
    for (at, pixel) in out.chunks_exact(4).enumerate() {
      by_kind.entry(at % 3).or_default().insert([pixel[0], pixel[1], pixel[2]]);
    }
    assert!(by_kind.values().all(|colors| colors.len() == 1));
  }

  #[test]
  fn the_color_of_a_group_is_near_its_average() {
    let mut image = Image::new_from_pixels(90, 1, three_kinds(), Channels::RGBA);
    quantize(3).apply_on_cpu(&mut image);
    let out = image.to_rgba_vec();
    // The reds were 200..206 and 30..36, and the group is painted near the middle of those.
    assert!((out[0] as i32 - 203).abs() <= 4, "red {:?}", &out[0..4]);
    assert!((out[1] as i32 - 33).abs() <= 4, "red {:?}", &out[0..4]);
  }

  #[test]
  fn the_alpha_is_left_alone_and_one_color_paints_the_average() {
    let mut pixels = three_kinds();
    for (at, pixel) in pixels.chunks_exact_mut(4).enumerate() {
      pixel[3] = at as u8;
    }
    let mut image = Image::new_from_pixels(90, 1, pixels.clone(), Channels::RGBA);
    quantize(0).apply_on_cpu(&mut image);
    let out = image.to_rgba_vec();
    assert_eq!(distinct(&out).len(), 1);
    assert!(out.chunks_exact(4).zip(pixels.chunks_exact(4)).all(|(after, before)| after[3] == before[3]));
  }

  #[test]
  fn a_given_palette_paints_the_nearest_color_and_the_image_gives_its_own_palette() {
    let palette = [Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255)];
    let mut image = Image::new_from_pixels(90, 1, three_kinds(), Channels::RGBA);
    quantize(5).with_palette(palette).apply_on_cpu(&mut image);
    let out = image.to_rgba_vec();
    assert_eq!(distinct(&out), HashSet::from([[255, 0, 0], [0, 0, 255]]));
    // The blue kind is the third of each three pixels.
    assert_eq!(&out[8..11], &[0, 0, 255]);

    let found = quantize(3).palette(&Image::new_from_pixels(90, 1, three_kinds(), Channels::RGBA));
    assert_eq!(found.len(), 3);
  }

  #[test]
  fn colors_that_are_closer_than_the_tolerance_are_merged_into_one() {
    // A light red, a middle red, a dark red, and a blue, row by row.
    let row = || {
      let mut pixels = Vec::new();
      for at in 0..40usize {
        let color = [[225u8, 70, 70], [170, 50, 50], [120, 38, 38], [40, 40, 210]][at % 4];
        pixels.extend_from_slice(&[color[0], color[1], color[2], 255]);
      }
      Image::new_from_pixels(40, 1, pixels, Channels::RGBA)
    };
    // Asked for four colors and given no tolerance, all four stay.
    let mut image = row();
    quantize(4).apply_on_cpu(&mut image);
    assert_eq!(distinct(&image.to_rgba_vec()).len(), 4);
    // With a tolerance the three reds are near enough to each other to be one color, and the blue is far from them.
    let mut image = row();
    quantize(4).with_tolerance(40.0).apply_on_cpu(&mut image);
    let out = image.to_rgba_vec();
    assert_eq!(distinct(&out).len(), 2);
    assert_eq!(out[0..4], out[4..8]);
    assert_eq!(out[4..8], out[8..12], "the three reds became one color");
    assert_ne!(out[0..3], out[12..15], "the blue is still apart");
  }

  #[test]
  fn a_lower_lightness_weight_merges_light_and_shade_of_one_hue_but_not_a_gray() {
    // A light red, a dark red of the same hue, and a gray as light as the light red, row by row.
    let row = || {
      let mut pixels = Vec::new();
      for at in 0..30usize {
        let color = [[225u8, 70, 70], [120, 38, 38], [170, 170, 170]][at % 3];
        pixels.extend_from_slice(&[color[0], color[1], color[2], 255]);
      }
      Image::new_from_pixels(30, 1, pixels, Channels::RGBA)
    };
    // As the Lab distance is, the light red and the dark red are further apart than the tolerance, so all three colors stay.
    let mut image = row();
    quantize(3).with_tolerance(34.0).apply_on_cpu(&mut image);
    assert_eq!(distinct(&image.to_rgba_vec()).len(), 3);
    // With brightness counting for less the two reds are near enough to be one color, and the gray is still far from them.
    let mut image = row();
    quantize(3).with_tolerance(34.0).with_lightness(0.4).apply_on_cpu(&mut image);
    let out = image.to_rgba_vec();
    assert_eq!(distinct(&out).len(), 2);
    assert_eq!(out[0..4], out[4..8], "the light red and the dark red became one color");
    assert_ne!(out[0..3], out[8..11], "the gray is still apart");
  }

  #[test]
  fn sources_decide_which_palette_color_a_pixel_takes() {
    // Pixels near red take the first palette color and pixels near blue take the second, whatever the colors of the palette are.
    let red = Color::from_rgb(200, 30, 30);
    let blue = Color::from_rgb(40, 30, 210);
    let mut image = Image::new_from_pixels(90, 1, three_kinds(), Channels::RGBA);
    quantize(0)
      .with_palette([Color::from_rgb(0, 255, 0), Color::from_rgb(255, 255, 0)])
      .with_sources([red, blue])
      .apply_on_cpu(&mut image);
    let out = image.to_rgba_vec();
    // The first pixel of each three is red, the second green and the third blue. Green is nearer to the red source than the blue.
    assert_eq!(out[0..3], [0, 255, 0], "red went to the first color");
    assert_eq!(out[8..11], [255, 255, 0], "blue went to the second color");
    // Sources of another number than the palette are not used: the pixels take the palette color nearest to themselves.
    let mut image = Image::new_from_pixels(90, 1, three_kinds(), Channels::RGBA);
    quantize(0).with_palette([Color::from_rgb(0, 255, 0), Color::from_rgb(255, 255, 0)]).with_sources([red]).apply_on_cpu(&mut image);
    assert_eq!(image.to_rgba_vec()[0..3], [255, 255, 0], "the red pixel is nearer to yellow than to green");
  }

  #[test]
  fn the_gpu_paints_a_sixty_four_color_palette_as_the_cpu_does() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};

    let (width, height) = (31, 19);
    let pixels: Vec<u8> =
      (0..width * height).flat_map(|pixel| [(pixel * 31) as u8, (pixel * 17) as u8, (pixel * 7) as u8, pixel as u8]).collect();
    let effect = quantize(0).with_palette(Palette::comic());
    assert_eq!(effect.palette(&Image::new_from_pixels(width, height, pixels.clone(), Channels::RGBA)).len(), 64);
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let gpu = renderer.process(&[effect.passes(width, height)], width, height, &pixels)?;
    let mut cpu = Image::new_from_pixels(width, height, pixels, Channels::RGBA);
    effect.apply_on_cpu(&mut cpu);
    let cpu = cpu.to_rgba_vec();
    let different = gpu.chunks_exact(4).zip(cpu.chunks_exact(4)).filter(|(a, b)| a != b).count();
    // With so many colors a pixel is more often almost exactly as near to two of them, and may go either way.
    assert!(different <= 6, "{different} of {} pixels differ", width * height);
    Ok(())
  }

  #[test]
  fn the_gpu_paints_a_palette_as_the_cpu_does() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};

    let (width, height) = (31, 19);
    let pixels: Vec<u8> =
      (0..width * height).flat_map(|pixel| [(pixel * 31) as u8, (pixel * 17) as u8, (pixel * 7) as u8, pixel as u8]).collect();
    let palette = [(250, 20, 40), (240, 220, 20), (20, 30, 120), (255, 240, 224), (10, 10, 10), (120, 120, 120)];
    let colors = palette.map(|(r, g, b)| Color::from_rgb(r, g, b));
    // The colors of the palette go with the sources in a different order, so the pixels do not take the nearest palette color.
    let mut sources = colors;
    sources.reverse();
    let effect = quantize(0).with_lightness(0.6).with_palette(colors).with_sources(sources);
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let gpu = renderer.process(&[effect.passes(width, height)], width, height, &pixels)?;
    let mut cpu = Image::new_from_pixels(width, height, pixels, Channels::RGBA);
    effect.apply_on_cpu(&mut cpu);
    let cpu = cpu.to_rgba_vec();
    let different = gpu.chunks_exact(4).zip(cpu.chunks_exact(4)).filter(|(a, b)| a != b).count();
    // A pixel that is almost exactly as near to two palette colors may go either way.
    assert!(different <= 2, "{different} of {} pixels differ", width * height);
    Ok(())
  }
}
