use crate::{
  apply_filter,
  blur::blur,
  kernel::apply_kernel,
  sobel::{SobelDirection, sobel},
};
use abra_core::{Image, ImageRef};
use adjustments::color::grayscale;
use options::Apply;

// TODO: Implement the glowing_edges filter to look a little more like Photoshop's glowing edges filter.
/// Applies the glowing edges filter to the image.
fn apply_glowing_edges(p_image: &mut Image, p_edge_width: u32, _edge_brightness: u32, _smoothness: u32) {
  // Step 1: Convert to grayscale
  let mut clone = p_image.clone();
  grayscale().apply(&mut clone);

  // Step 2: Apply Sobel filter to detect edges
  sobel(SobelDirection::Horizontal).apply(&mut clone);

  // Step 3: Adjust edge width by dilating the edges
  for _ in 0..p_edge_width {
    apply_kernel(&mut clone, &[0.0, 0.5, 0.0, 0.5, 1.0, 0.5, 0.0, 0.5, 0.0]);
  }

  // Step 4: Adjust edge brightness
  let pixels = clone.empty_pixel_vec();
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
  blur().apply(&mut clone);

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

#[derive(Clone)]
pub struct GlowingEdges {
  edge_width: u32,
  edge_brightness: u32,
  smoothness: u32,
  options: options::Options,
}
options::cpu_processor!(GlowingEdges);

impl Apply for GlowingEdges {
  fn options(&self) -> &options::Options {
    &self.options
  }
  fn options_mut(&mut self) -> &mut options::Options {
    &mut self.options
  }
  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    let options = self.options.clone();
    apply_filter!(apply_glowing_edges, image, options, 1, self.edge_width, self.edge_brightness, self.smoothness);
  }
}
impl GlowingEdges {
  /// Sets the width of the detected edges. Defaults to `2`.
  pub fn with_edge_width(mut self, p_edge_width: u32) -> Self {
    self.edge_width = p_edge_width;
    self
  }

  /// Sets how bright the edges glow. Defaults to `6`.
  pub fn with_edge_brightness(mut self, p_edge_brightness: u32) -> Self {
    self.edge_brightness = p_edge_brightness;
    self
  }

  /// Sets how much the edges are smoothed. Defaults to `5`.
  pub fn with_smoothness(mut self, p_smoothness: u32) -> Self {
    self.smoothness = p_smoothness;
    self
  }
}

/// Finds the edges in the image and makes them glow against a dark background.
pub fn glowing_edges() -> GlowingEdges {
  GlowingEdges {
    edge_width: 2,
    edge_brightness: 6,
    smoothness: 5,
    options: None,
  }
}
