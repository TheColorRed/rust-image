use abra_core::{
  Image,
  image::gpu::{GpuAux, GpuPass},
};
use primitives::{
  color::{linear_to_srgb, srgb_u8_to_linear_f32},
  gaussian::gaussian_weights,
};
use rayon::prelude::*;

pub(super) fn luminance(p_rgb: [f32; 3]) -> f32 {
  0.2126 * p_rgb[0] + 0.7152 * p_rgb[1] + 0.0722 * p_rgb[2]
}

/// Red and green hold the separate shadow and highlight neighborhood brightness.
pub(super) fn contexts(p_image: &Image, p_radii: [u32; 2]) -> Vec<u8> {
  let (width, height) = p_image.dimensions::<usize>();
  let decode: [f32; 256] = std::array::from_fn(|i| srgb_u8_to_linear_f32(i as u8));
  let mut pixels = vec![0; width * height * 4];
  pixels.par_chunks_mut(4).zip(p_image.rgba().par_chunks(4)).for_each(|(dst, src)| {
    let gray = (linear_to_srgb(luminance([
      decode[src[0] as usize],
      decode[src[1] as usize],
      decode[src[2] as usize],
    ]))
      * 255.0)
      .round() as u8;
    dst.copy_from_slice(&[gray, gray, 0, 255]);
  });
  if width == 0 || height == 0 || p_radii == [0, 0] {
    return pixels;
  }
  let weights = p_radii.map(gaussian_weights);
  for vertical in [false, true] {
    let mut out = vec![0; pixels.len()];
    out.par_chunks_mut(width * 4).enumerate().for_each(|(y, row)| {
      for x in 0..width {
        for channel in 0..2 {
          let radius = p_radii[channel] as i32;
          let mut sum = 0u32;
          for (offset, &weight) in (-radius..=radius).zip(&weights[channel]) {
            let nx = if vertical { x } else { (x as i32 + offset).clamp(0, width as i32 - 1) as usize };
            let ny = if vertical { (y as i32 + offset).clamp(0, height as i32 - 1) as usize } else { y };
            sum += pixels[(ny * width + nx) * 4 + channel] as u32 * weight;
          }
          row[x * 4 + channel] = ((sum + 32768) >> 16) as u8;
        }
        row[x * 4 + 3] = 255;
      }
    });
    pixels = out;
  }
  pixels
}

pub(super) fn passes(p_radii: [u32; 2]) -> Vec<GpuPass> {
  let mut passes = vec![GpuPass::new(
    include_str!("./highlights_shadows_context.wgsl"),
    Vec::new(),
  )];
  if p_radii == [0, 0] {
    return passes;
  }
  let maximum = p_radii[0].max(p_radii[1]);
  let width = 2 * maximum + 1;
  let mut table = vec![0; width as usize * 2 * 4];
  for channel in 0..2 {
    for (index, weight) in gaussian_weights(p_radii[channel]).into_iter().enumerate() {
      let at = (channel * width as usize + (maximum - p_radii[channel]) as usize + index) * 4;
      table[at..at + 4].copy_from_slice(&[weight as u8, (weight >> 8) as u8, (weight >> 16) as u8, 255]);
    }
  }
  let table: std::sync::Arc<[u8]> = table.into();
  for direction in 0..2u32 {
    let uniforms =
      [p_radii[0], p_radii[1], direction, maximum].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>();
    passes.push(GpuPass::new(include_str!("./highlights_shadows_blur.wgsl"), uniforms).with_pixel_exact().with_aux(GpuAux {
      width,
      height: 2,
      rgba: table.clone(),
    }));
  }
  passes
}
