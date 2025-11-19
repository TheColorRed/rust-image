use crate::{blur::blur, kernel::apply_kernel, sobel::sobel_horizontal};
use adjustments::color::grayscale;
use core::Image;

// TODO: Implement the glowing_edges filter to look a little more like Photoshop's glowing edges filter.
/// Applies the glowing edges filter to the image.
pub fn glowing_edges(image: &mut Image, edge_width: u32, edge_brightness: u32, _smoothness: u32) {
  // Step 1: Convert to grayscale
  let mut clone = image.clone();
  grayscale(&mut clone);

  // Step 2: Apply Sobel filter to detect edges
  sobel_horizontal(&mut clone);

  // Step 3: Adjust edge width by dilating the edges
  for _ in 0..edge_width {
    apply_kernel(&mut clone, &[0.0, 0.5, 0.0, 0.5, 1.0, 0.5, 0.0, 0.5, 0.0]);
  }

  // Step 4: Adjust edge brightness
  let mut pixels = clone.empty_pixel_vec();
  // pixels.par_chunks_mut(4).enumerate().for_each(|(i, pixel)| {
  //   let edge_pixel_r = clone.r[i] as f32;
  //   let edge_pixel_g = clone.g[i] as f32;
  //   let edge_pixel_b = clone.b[i] as f32;

  //   let brightness_r = edge_pixel_r * edge_brightness as f32;
  //   let brightness_g = edge_pixel_g * edge_brightness as f32;
  //   let brightness_b = edge_pixel_b * edge_brightness as f32;

  //   pixel[0] = brightness_r.min(255.0) as u8;
  //   pixel[1] = brightness_g.min(255.0) as u8;
  //   pixel[2] = brightness_b.min(255.0) as u8;
  //   pixel[3] = 255;
  // });
  clone.set_rgba(pixels);

  // Step 5: Apply Gaussian blur to smooth the edges
  blur(&mut clone);

  // image.set_pixels(clone.rgba().to_vec());

  // Step 6: Combine the edges with the original image
  // for (original_pixel, edge_pixel) in clone.r.par_iter_mut().zip(image.r.iter_mut()) {
  //   let blended_pixel = (*original_pixel as f32 * 0.5 + *edge_pixel as f32 * 0.5).min(255.0) as u8;
  //   *original_pixel = blended_pixel;
  // }

  // let mut pixels = image.empty_pixel_vec();
  // pixels.par_chunks_mut(4).enumerate().for_each(|(i, pixel)| {
  //   let original_pixel = image.r[i] as f32;
  //   let blended_pixel = (original_pixel as f32 * 0.5 + pixel[0] as f32 * 0.5).min(255.0) as u8;
  //   pixel[0] = blended_pixel;
  //   pixel[1] = blended_pixel;
  //   pixel[2] = blended_pixel;
  // });

  // image.copy_channel_data(&clone);
}
