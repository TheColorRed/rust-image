use core::Image;
use rayon::prelude::*;

/// Applies a kernel to an image.
/// A kernel is a matrix used for convolution operations in image processing.
/// This function applies the given kernel to each pixel of the image,
/// modifying the pixel values based on the kernel weights and neighboring pixels.
pub fn apply_kernel(image: &mut Image, kernel: &[f32]) {
  let (width, height) = image.dimensions::<u32>();
  let mut new_pixels = vec![0; (width * height * 4) as usize];
  let old_pixels = image.rgba();

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % width;
    let y = i as u32 / width;
    let mut new_pixel = [0.0; 4];
    let mut kernel_index = 0;
    for dy in -1..=1 {
      for dx in -1..=1 {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
          let old_index = (ny as u32 * width + nx as u32) as usize;
          for c in 0..4 {
            new_pixel[c] += old_pixels[old_index * 4 + c] as f32 * kernel[kernel_index];
          }
        }
        kernel_index += 1;
      }
    }
    for c in 0..4 {
      chunk[c] = new_pixel[c].round().clamp(0.0, 255.0) as u8;
    }
  });

  image.set_rgba_owned(new_pixels);
}
