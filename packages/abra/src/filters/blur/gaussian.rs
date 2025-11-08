#![allow(unused_imports, unused_variables, unused_mut, unused_assignments, dead_code)]
use crate::image::Image;
use rayon::prelude::*;

fn gaussian_kernel_1d(radius: u32) -> Vec<f32> {
  let mut kernel = vec![0.0; (2 * radius + 1) as usize];
  let sigma = radius as f32 / 2.0;
  let pi = std::f32::consts::PI;
  let sum = (0..=radius)
    .map(|x| {
      let value = (-(x as f32 * x as f32) / (2.0 * sigma * sigma)).exp() / (2.0 * pi * sigma * sigma);
      kernel[radius as usize + x as usize] = value;
      kernel[radius as usize - x as usize] = value;
      value
    })
    .sum::<f32>();
  kernel.iter_mut().for_each(|value| *value /= sum);
  kernel
}

// fn apply_gaussian_blur_1D(x: i32, y: i32, radius: f32, src: texture_2d<f32>, horizontal: bool) -> Vec<f32> {}

/// Applies a Gaussian blur to an image.
/// * `image` - A mutable reference to the image to be blurred.
/// * `radius` - The radius of the Gaussian kernel.
pub fn gaussian(image: &mut Image, radius: u32) {
  let kernel = gaussian_kernel_1d(radius);
  let (width, height) = image.dimensions::<i32>();
  let mut pixels = image.empty_rgb_pixel_vec();
  let kernel_radius = radius as i32;
  let kernel_size = 2 * kernel_radius + 1;

  pixels.par_chunks_mut(3).enumerate().for_each(|(i, pixel)| {
    let x = i as i32 % width;
    let y = i as i32 / width;
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    let mut weight_sum = 0.0;

    // Apply horizontal blur
    // for kx in -kernel_radius..=kernel_radius {
    //   let px = (x + kx).clamp(0, width as i32 - 1);
    //   let pixel_index = (y * width as i32 + px) as usize;
    //   let weight = kernel[(kx + kernel_radius) as usize];
    //   r += image.r[pixel_index] as f32 * weight;
    //   g += image.g[pixel_index] as f32 * weight;
    //   b += image.b[pixel_index] as f32 * weight;
    //   weight_sum += weight;
    // }

    // Apply vertical blur
    // for ky in -kernel_radius..=kernel_radius {
    //   let py = (y + ky).clamp(0, height as i32 - 1);
    //   let pixel_index = (py * width as i32 + x) as usize;
    //   let weight = kernel[(ky + kernel_radius) as usize];
    //   r += image.r[pixel_index] as f32 * weight;
    //   g += image.g[pixel_index] as f32 * weight;
    //   b += image.b[pixel_index] as f32 * weight;
    //   weight_sum += weight;
    // }

    r /= weight_sum;
    g /= weight_sum;
    b /= weight_sum;

    // pixel[0] = r.clamp(0.0, 255.0) as u8;
    // pixel[1] = g.clamp(0.0, 255.0) as u8;
    // pixel[2] = b.clamp(0.0, 255.0) as u8;
  });

  image.set_rgb(pixels);

  // let apply = image.apply_shader_from_string(
  //   include_str!("gaussian.wgsl"),
  //   Some(vec![ShaderBinding {
  //     name: "radius".into(),
  //     value: radius as f32,
  //   }]),
  // );

  // // Double the radius to ensure the kernel size is appropriate
  // radius *= 2;

  // // Calculate the size of the kernel
  // let kernel_size = 2 * radius + 1;

  // // Initialize the kernel with zeros
  // let mut kernel = vec![0.0; (kernel_size * kernel_size) as usize];

  // // Calculate the standard deviation (sigma) for the Gaussian function
  // let sigma = radius as f32 / 2.0;

  // // Variable to accumulate the sum of all kernel values for normalization
  // let mut sum = 0.0;

  // // Fill the kernel with Gaussian values
  // for y in 0..kernel_size {
  //   for x in 0..kernel_size {
  //     // Calculate the distance from the center of the kernel
  //     let x_dist = x - radius;
  //     let y_dist = y - radius;

  //     // Calculate the Gaussian value for the current position
  //     let value = (-(x_dist * x_dist + y_dist * y_dist) as f32 / (2.0 * sigma * sigma)).exp() / (2.0 * PI * sigma * sigma);

  //     // Assign the calculated value to the kernel
  //     kernel[(y * kernel_size + x) as usize] = value;

  //     // Accumulate the sum of the kernel values
  //     sum += value;
  //   }
  // }

  // // Normalize the kernel values so that their sum is 1
  // for value in kernel.iter_mut() {
  //   *value /= sum;
  // }

  // // Define the radius of the kernel for the blur function
  // let kernel_radius = radius;

  // // Apply the Gaussian blur to the image using the generated kernel
  // gaussian_blur(image, &kernel, kernel_radius, kernel_size as usize);
}

/// Applies a Gaussian blur to an image using the provided kernel.
/// * `image` - A mutable reference to the image to be blurred.
/// * `kernel` - A slice containing the Gaussian kernel values.
/// * `kernel_radius` - The radius of the Gaussian kernel.
/// * `kernel_size` - The size of the Gaussian kernel.
#[allow(dead_code)]
fn gaussian_blur(image: &mut Image, kernel: &[f32], kernel_radius: i32, kernel_size: usize) {
  // Get the width and height of the image
  let (width, height) = image.dimensions::<u32>();
  let width = width as i32;
  let height = height as i32;

  // Create a vector to store the blurred pixel values
  let mut pixels = image.empty_rgb_pixel_vec();

  // Process each pixel in parallel
  pixels.par_chunks_mut(3).enumerate().for_each(|(i, pixel)| {
    // Calculate the x and y coordinates of the current pixel
    let x = i as i32 % width;
    let y = i as i32 / width;

    // Initialize the RGB values to accumulate the weighted sum
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;

    // Iterate over the kernel
    for ky in -kernel_radius..=kernel_radius {
      for kx in -kernel_radius..=kernel_radius {
        // Calculate the coordinates of the neighboring pixel
        let px = (x + kx).clamp(0, width - 1);
        let py = (y + ky).clamp(0, height - 1);

        // Calculate the index of the neighboring pixel
        let pixel_index = (py * width + px) as usize;

        // Get the weight from the kernel
        let weight = kernel[(ky + kernel_radius) as usize * kernel_size + (kx + kernel_radius) as usize];

        // Accumulate the weighted RGB values
        // r += image.r[pixel_index] as f32 * weight;
        // g += image.g[pixel_index] as f32 * weight;
        // b += image.b[pixel_index] as f32 * weight;
      }
    }

    // Clamp the accumulated RGB values to the range [0, 255] and assign them to the pixel
    // pixel[0] = r.clamp(0.0, 255.0) as u8;
    // pixel[1] = g.clamp(0.0, 255.0) as u8;
    // pixel[2] = b.clamp(0.0, 255.0) as u8;
  });

  // Update the image with the blurred pixel values
  image.set_rgb(pixels);
}
