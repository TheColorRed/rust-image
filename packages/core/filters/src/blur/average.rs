use core::Image;
use rayon::prelude::*;

/// Blurs an image using the average blur algorithm.
pub fn average_blur(image: &mut Image, radius: u32) {
  let kernel_size = radius * 2 + 1;
  let kernel_area = kernel_size * kernel_size;
  let kernel = vec![1.0 / kernel_area as f32; kernel_area as usize];
  let (width, height) = image.dimensions();
  let src = image.rgba();
  let mut out = image.empty_pixel_vec();

  out.par_chunks_mut((width as usize) * 4).enumerate().for_each(|(y, row_out)| {
    for x in 0..width as usize {
      let mut r = 0.0;
      let mut g = 0.0;
      let mut b = 0.0;

      for dy in 0..kernel_size {
        for dx in 0..kernel_size {
          let nx = (x as usize).saturating_sub(radius as usize) + dx as usize;
          let ny = y.saturating_sub(radius as usize) + dy as usize;

          if nx >= width || ny >= height as usize {
            continue;
          }

                let index = (ny * width as usize + nx as usize) as usize * 4;
                let weight = kernel[(dy * kernel_size + dx) as usize];

                r += src[index] as f32 * weight;
                g += src[index + 1] as f32 * weight;
                b += src[index + 2] as f32 * weight;
        }
      }

      let out_index = x * 4;
      row_out[out_index] = r as u8;
      row_out[out_index + 1] = g as u8;
      row_out[out_index + 2] = b as u8;
      // preserve alpha from source
      let alpha_index = (y * width as usize + x) * 4;
      row_out[out_index + 3] = src[alpha_index + 3];
    }
  });
  image.set_rgba_owned(out);
}
