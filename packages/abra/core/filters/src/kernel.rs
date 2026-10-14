use abra_core::Image;
use rayon::prelude::*;

/// Applies a kernel to an image.
/// A kernel is a matrix used for convolution operations in image processing.
/// This function applies the given kernel to each pixel of the image,
/// modifying the pixel values based on the kernel weights and neighboring pixels. The edge pixels are repeated past the edge.
pub fn apply_kernel(p_image: &mut Image, p_kernel: &[f32]) {
  let (width, height) = p_image.dimensions::<u32>();
  let mut new_pixels = vec![0; (width * height * 4) as usize];
  let old_pixels = p_image.rgba();

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = i as u32 % width;
    let y = i as u32 / width;
    let mut new_pixel = [0.0; 4];
    let mut kernel_index = 0;
    for dy in -1..=1 {
      for dx in -1..=1 {
        // Past the edge is the pixel at the edge, so a part cut from the picture gives what the whole picture has there.
        let nx = (x as i32 + dx).clamp(0, width as i32 - 1);
        let ny = (y as i32 + dy).clamp(0, height as i32 - 1);
        let old_index = (ny as u32 * width + nx as u32) as usize;
        for c in 0..4 {
          new_pixel[c] += old_pixels[old_index * 4 + c] as f32 * p_kernel[kernel_index];
        }
        kernel_index += 1;
      }
    }
    for c in 0..4 {
      chunk[c] = new_pixel[c].round().clamp(0.0, 255.0) as u8;
    }
  });

  p_image.set_rgba(new_pixels);
}
