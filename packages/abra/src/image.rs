use std::time::Instant;

use crate::Channels;
use crate::color::Color;
use crate::geometry::{Area, PointF, Size};
use crate::transform::{Crop, Resize, Rotate, TransformAlgorithm, crop};
use crate::utils::debug::DebugInfo;
use crate::utils::fs::WriterOptions;
use crate::utils::fs::file_info::FileInfo;
use crate::utils::fs::readers::svg::read_svg;
use crate::utils::fs::readers::{gif::read_gif, jpeg::read_jpg, png::read_png, webp::read_webp};
use crate::utils::fs::writers::{gif::write_gif, jpeg::write_jpg, png::write_png, webp::write_webp};
use ndarray::{Array1, ArrayViewMut1, Axis};
use rayon::prelude::*;

#[derive(Debug)]
/// An image.
pub struct Image {
  /// The width of the image.
  width: u32,
  /// The height of the image.
  height: u32,
  /// The number of colors in the image (width * height).
  color_len: i32,
  /// The colors of the image.
  /// The colors are stored in a 3D array where the first dimension is the width, the second dimension is the height, and the third dimension is the RGBA channels.
  colors: Array1<u8>,
  /// The level of anti-aliasing. Default is 4.
  pub anti_aliasing_level: u32,
}

impl Image {
  /// Create a new image.
  /// - `width`: The width of the image.
  /// - `height`: The height of the image.
  pub fn new(width: impl TryInto<u32>, height: impl TryInto<u32>) -> Image {
    let width = width.try_into().ok().unwrap_or(0);
    let height = height.try_into().ok().unwrap_or(0);
    let colors = Array1::zeros(width as usize * height as usize * 4);
    Image {
      width,
      height,
      color_len: width as i32 * height as i32,
      colors,
      anti_aliasing_level: 4,
    }
  }

  /// Create a new image from a vector of pixels.
  pub fn new_from_pixels(width: u32, height: u32, pixels: Vec<u8>, channels: Channels) -> Image {
    let mut img = Image::new(width, height);
    match channels {
      Channels::RGBA => img.set_rgba(pixels),
      Channels::RGB => img.set_rgb(pixels),
    }
    // img.set_rgba(pixels);
    img
  }

  /// Create a new image from a file.
  /// - `path`: The file path.
  pub fn new_from_path(path: impl Into<String>) -> Image {
    let mut img = Image::new(0, 0);
    img.open(path.into().as_str());
    img
  }

  /// Create a new image filled with a specific color.
  /// - `width`: The width of the image.
  /// - `height`: The height of the image.
  /// - `color`: The color to fill the image with.
  pub fn new_from_color(width: u32, height: u32, color: Color) -> Image {
    let mut img = Image::new(width, height);
    img.clear_color(color);
    img
  }

  /// Get an empty RGBA pixel buffer that is the sames size as the image.
  pub fn empty_pixel_vec(&self) -> Vec<u8> {
    vec![0; (self.width * self.height) as usize * 4]
  }

  /// Get an empty RGB pixel buffer that is the sames size as the image.
  pub fn empty_rgb_pixel_vec(&self) -> Vec<u8> {
    vec![0; (self.width * self.height) as usize * 3]
  }

  /// Sets the image to be fully transparent by setting all channels to zero.
  pub fn clear(&mut self) {
    let size = (self.width * self.height) as usize;
    self.colors = Array1::zeros(size * 4);
  }

  /// Clears the current image and fills it with a specific color.
  pub fn clear_color(&mut self, color: Color) {
    let size = (self.width * self.height) as usize;
    let mut pixels = Vec::with_capacity(size * 4);
    for _ in 0..size {
      pixels.push(color.r);
      pixels.push(color.g);
      pixels.push(color.b);
      pixels.push(color.a);
    }
    self.colors = Array1::from_shape_vec(size * 4, pixels).unwrap();
  }

  /// Copies the channel data from another image and sets it to this image.
  /// - `src`: The source image to get the channel data from.
  pub fn copy_channel_data(&mut self, src: &Image) {
    self.colors = src.colors.clone();
  }

  /// Opens an image into the image buffer.
  /// - `file`: The file path.
  pub fn open(&mut self, file: impl Into<String>) {
    let start = Instant::now();
    let info: FileInfo;
    let file = file.into();
    if file.ends_with(".jpg") || file.ends_with(".jpeg") {
      info = read_jpg(&file).unwrap();
    } else if file.ends_with(".webp") {
      info = read_webp(&file).unwrap();
    } else if file.ends_with(".png") {
      info = read_png(&file).unwrap();
    } else if file.ends_with(".gif") {
      info = read_gif(&file).unwrap();
    } else if file.ends_with(".svg") {
      info = read_svg(&file).unwrap();
    } else {
      panic!("Attempting to open unsupported file format");
    }

    self.width = info.width;
    self.height = info.height;
    self.set_new_pixels(info.pixels, info.width, info.height);
    self.color_len = self.width as i32 * self.height as i32;
    DebugInfo::ImageOpened(file.to_string(), self.width, self.height, start.elapsed()).log();
  }

  /// Saves the image buffer to a file.
  /// - `file`: The file path.
  pub fn save(&self, file: impl Into<String>, options: impl Into<Option<WriterOptions>>) {
    if self.width == 0 || self.height == 0 {
      panic!("Attempting to save an image with zero width or height");
    }

    let start = Instant::now();
    let options = options.into();
    let file = file.into();
    if file.ends_with(".jpg") || file.ends_with(".jpeg") {
      write_jpg(&file, &self, &options).unwrap();
    } else if file.ends_with(".webp") {
      write_webp(&file, &self).unwrap();
    } else if file.ends_with(".png") {
      write_png(&file, &self, &options).unwrap();
    } else if file.ends_with(".gif") {
      write_gif(&file, &self, &options).unwrap();
    } else {
      panic!("Attempting to save unsupported file format");
    }
    DebugInfo::ImageSaved(file.to_string(), self.width, self.height, start.elapsed()).log();
  }

  /// Get the dimensions of the image and return it as a tuple of type `T`.
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u32>,
    <T as TryFrom<u32>>::Error: std::fmt::Debug,
  {
    let width = T::try_from(self.width).unwrap();
    let height = T::try_from(self.height).unwrap();
    (width, height)
  }

  /// Get the size of the image and return it as a `Size` struct.
  pub fn size(&self) -> Size {
    let width = self.width;
    let height = self.height;
    Size::new(width, height)
  }

  /// Set the pixels of the image from a vector into their respective channels.
  /// - `pixels`: The pixels of the image.
  pub fn set_rgba(&mut self, data: Vec<u8>) {
    self.colors = Array1::from_shape_vec(self.width as usize * self.height as usize * 4, data).unwrap();
  }

  /// Set the pixels of the image from a vector into their respective channels.
  /// - `pixels`: The pixels of the image.
  pub fn set_rgb(&mut self, data: Vec<u8>) {
    let (width, height) = self.dimensions::<usize>();
    if data.len() != width * height * 3 {
      panic!("Trying to set {} pixels into an image with {} pixels.", data.len(), self.width * self.height * 3);
    }

    // Replace the colors with the new RGB data ignoring the alpha channel
    let current = self.colors.to_vec();
    let new_data: Vec<u8> = data
      .par_chunks(3)
      .zip(current.par_chunks(4))
      .flat_map_iter(|(rgb, a)| [rgb[0], rgb[1], rgb[2], a[3]])
      .collect();

    self.colors = Array1::from_shape_vec(self.width as usize * self.height as usize * 4, new_data).unwrap();
  }

  /// Set the colors of the image.
  pub fn set_colors(&mut self, colors: Array1<u8>) {
    self.set_rgba(colors.to_vec());
  }

  /// Set the pixels of the image from a vector into their respective channels.
  /// - `channel`: The channel that the pixels belong to.
  /// - `pixels`: The pixels for the channel.
  pub fn set_channel(&mut self, channel: impl Into<String>, pixels: Vec<u8>) {
    let channel = channel.into();
    let mut current = self.colors.to_vec();
    current
      .par_chunks_mut(4)
      .enumerate()
      .for_each(|(i, chunk)| match channel.as_str() {
        "r" => chunk[0] = pixels[i],
        "g" => chunk[1] = pixels[i],
        "b" => chunk[2] = pixels[i],
        "a" => chunk[3] = pixels[i],
        _ => (),
      });
    self.colors = Array1::from_shape_vec(self.width as usize * self.height as usize * 4, current).unwrap();
  }

  /// Set the pixels of the image from another image into their respective channels at a specific position.
  /// - `src`: The source image to get the pixels from.
  /// - `dest_x`: The x position to start setting the pixels.
  /// - `dest_y`: The y position to start setting the pixels.
  pub fn set_from(&mut self, src: &Image, point: impl Into<PointF>) {
    let point = point.into();
    let dest_x = point.x as i32;
    let dest_y = point.y as i32;

    for y in 0..src.height as i32 {
      for x in 0..src.width as i32 {
        let target_x = dest_x + x;
        let target_y = dest_y + y;
        if target_x >= 0 && target_y >= 0 && target_x < self.width as i32 && target_y < self.height as i32 {
          if let Some(pixel) = src.get_pixel(x as u32, y as u32) {
            self.set_pixel(target_x as u32, target_y as u32, pixel);
          }
        }
      }
    }
  }

  /// Set the pixels of the image from a vector into their respective channels when the new pixel data size is different from the current pixel data size, or when the width and/or height of the image is different.
  /// - `pixels`: The pixels of the image. Either as an RGBA or RGB vector.
  /// - `width`: The width of the image.
  /// - `height`: The height of the image.
  pub fn set_new_pixels(&mut self, data: Vec<u8>, width: u32, height: u32) {
    let is_rgba = data.len() == width as usize * height as usize * 4;
    let is_rgb = data.len() == width as usize * height as usize * 3;
    #[rustfmt::skip]
    let channels = if is_rgba { 4 } else if is_rgb { 3 } else {
      panic!(
        "Invalid pixel data size, expected {} (rgba) or {} (rgb) but got {}",
        width * height * 4,
        width * height * 3,
        data.len()
      );
    };

    self.width = width;
    self.height = height;
    // fill with 255
    self.colors = Array1::zeros(width as usize * height as usize * 4);
    let mut pixels = data.clone();
    if channels == 3 {
      pixels = pixels.par_chunks(3).flat_map(|p| vec![p[0], p[1], p[2], 255]).collect();
    }
    self.set_rgba(pixels);
  }

  /// Checks if the image or image data is in RGBA format.
  /// - `data`: Optional image data to check. If None, checks the image itself.
  pub fn is_rgba(&self, data: impl Into<Option<Vec<u8>>>) -> bool {
    if let Some(pixels) = data.into() {
      pixels.len() == (self.width * self.height * 4) as usize
    } else {
      println!("len: {}", self.colors.len());
      self.colors.len() == (self.width * self.height * 4) as usize
    }
  }

  /// Checks if the image or image data is in RGB format.
  /// - `data`: Optional image data to check. If None, checks the image itself.
  pub fn is_rgb(&self, data: impl Into<Option<Vec<u8>>>) -> bool {
    if let Some(pixels) = data.into() {
      pixels.len() == (self.width * self.height * 3) as usize
    } else {
      self.colors.len() == (self.width * self.height * 3) as usize
    }
  }

  /// Get the pixel at a specific location.
  /// - `x`: The x coordinate.
  /// - `y`: The y coordinate.
  pub fn get_pixel(&self, x: u32, y: u32) -> Option<(u8, u8, u8, u8)> {
    let index = ((y * self.width + x) as usize) * 4;
    if index + 3 >= self.colors.len() {
      return None;
    }
    Some((self.colors[index], self.colors[index + 1], self.colors[index + 2], self.colors[index + 3]))
  }

  /// Set the pixel at a specific location.
  /// - `x`: The x coordinate.
  /// - `y`: The y coordinate.
  /// - `pixel`: The pixel to set.
  pub fn set_pixel(&mut self, x: u32, y: u32, pixel: (u8, u8, u8, u8)) {
    let index = (y * self.width + x) as usize * 4;
    self.colors[index] = pixel.0;
    self.colors[index + 1] = pixel.1;
    self.colors[index + 2] = pixel.2;
    self.colors[index + 3] = pixel.3;
  }

  /// Get a reference to the image.
  pub fn as_ref(&self) -> &Image {
    self
  }

  /// Get a mutable reference to the image.
  pub fn as_ref_mut(&mut self) -> &mut Image {
    self
  }

  /// Gets the rgba colors of the image.
  /// Shortcut for `join_channels("rgba")`
  pub fn rgba(&self) -> Vec<u8> {
    self.colors.to_vec()
  }

  pub fn colors(&mut self) -> &mut Array1<u8> {
    &mut self.colors
  }

  /// Gets the red channel of the image.
  pub fn red(&self) -> Vec<u8> {
    self
      .colors
      .axis_chunks_iter(Axis(0), 4)
      .into_par_iter()
      .map(|row| row.iter().take(1).copied().collect::<Vec<_>>())
      .flatten()
      .collect()
  }

  /// Gets the green channel of the image.
  pub fn green(&self) -> Vec<u8> {
    self
      .colors
      .axis_chunks_iter(Axis(0), 4)
      .into_par_iter()
      .map(|row| row.iter().skip(1).take(1).copied().collect::<Vec<_>>())
      .flatten()
      .collect()
  }

  /// Gets the blue channel of the image.
  pub fn blue(&self) -> Vec<u8> {
    self
      .colors
      .axis_chunks_iter(Axis(0), 4)
      .into_par_iter()
      .map(|row| row.iter().skip(2).take(1).copied().collect::<Vec<_>>())
      .flatten()
      .collect()
  }

  /// Gets the alpha channel of the image.
  pub fn alpha(&self) -> Vec<u8> {
    self
      .colors
      .axis_chunks_iter(Axis(0), 4)
      .into_par_iter()
      .map(|row| row.iter().skip(3).take(1).copied().collect::<Vec<_>>())
      .flatten()
      .collect()
  }

  /// Gets the rgb colors of the image without the alpha channel.
  /// Shortcut for `join_channels("rgb")`
  pub fn rgb(&self) -> Vec<u8> {
    self
      .colors
      .axis_chunks_iter(Axis(0), 4)
      .into_par_iter()
      .map(|row| row.iter().take(3).copied().collect::<Vec<_>>())
      .flatten()
      .collect()
  }

  /// Iterate over the pixels of the image to apply a function on each pixel.
  /// The callback takes a pixel as an ArrayViewMut1 and should modify the pixel in place.
  pub fn mut_pixels_simd<F>(&mut self, callback: F)
  where
    F: Fn(ArrayViewMut1<u8>) + Send + Sync,
  {
    self
      .colors
      .axis_chunks_iter_mut(Axis(0), 4)
      .into_par_iter()
      .for_each(|row| {
        callback(row);
        // row.axis_iter_mut(Axis(0)).into_par_iter().for_each(|pixel| callback(pixel));
      });
  }

  /// Iterate over the channels of the image to apply a function on each channel including the alpha channel.
  /// The callback takes a pixel channel value and should return a new value for that channel.
  /// ```ignore
  /// let image = Image::new_from_path("input.png");
  /// // Invert all channels of the image including alpha.
  /// image.mut_channels_rgba(|value| 255 - value);
  /// ```
  pub fn mut_channels_rgba<F>(&mut self, callback: F)
  where
    F: Fn(u8) -> u8 + Send + Sync,
  {
    self.colors.par_map_inplace(|x| *x = callback(*x));
  }

  /// Iterate over the channels of the image to apply a function on each channel except the alpha channel.
  /// The callback takes a pixel channel value and should return a new value for that channel.
  /// ```ignore
  /// let image = Image::new_from_path("input.png");
  /// // Invert the RGB channels of the image.
  /// image.mut_channels_rgb(|value| 255 - value);
  /// ```
  pub fn mut_channels_rgb<F>(&mut self, callback: F)
  where
    F: Fn(u8) -> u8 + Send + Sync,
  {
    self
      .colors
      .axis_chunks_iter_mut(Axis(0), 4)
      .into_par_iter()
      .for_each(|mut row| {
        row.iter_mut().take(3).for_each(|pixel| *pixel = callback(*pixel));
      });
  }

  /// Iterate over a specific channel of the image to apply a function on each pixel of that channel.
  /// - `p_channel`: The channel to modify ("r", "g", "b", or "a").
  /// - `p_callback`: The callback function that takes a pixel channel value and returns a new value for that channel.
  /// ```ignore
  /// let image = Image::new_from_path("input.png");
  /// // Increase the red channel by 10 for each pixel.
  /// image.mut_channel("r", |value| value + 10);
  /// ```
  pub fn mut_channel<F>(&mut self, p_channel: impl Into<String>, p_callback: F)
  where
    F: Fn(u8) -> u8 + Send + Sync,
  {
    let channel = p_channel.into();
    self
      .colors
      .axis_chunks_iter_mut(Axis(0), 4)
      .into_par_iter()
      .for_each(|mut row| match channel.as_str() {
        "r" => row.iter_mut().take(1).for_each(|pixel| *pixel = p_callback(*pixel)),
        "g" => row
          .iter_mut()
          .skip(1)
          .take(1)
          .for_each(|pixel| *pixel = p_callback(*pixel)),
        "b" => row
          .iter_mut()
          .skip(2)
          .take(1)
          .for_each(|pixel| *pixel = p_callback(*pixel)),
        "a" => row
          .iter_mut()
          .skip(3)
          .take(1)
          .for_each(|pixel| *pixel = p_callback(*pixel)),
        _ => (),
      });
  }

  /// Iterate over the pixels of the image to apply a function on each pixel.
  /// The callback takes an index and a pixel as an ArrayViewMut1 and should modify the pixel in place.
  pub fn mut_pixels_with_position<F>(&mut self, callback: F)
  where
    F: Fn(usize, usize, ArrayViewMut1<u8>) + Send + Sync,
  {
    // Flatten the array to a 1D array and then iterate in parallel chunks
    self
      .colors
      .as_slice_mut()
      .unwrap()
      .par_chunks_mut(self.width as usize * 4)
      .enumerate()
      .for_each(|(y, row)| {
        row
          .chunks_mut(4)
          .enumerate()
          .for_each(|(x, pixel)| callback(x, y, ArrayViewMut1::from_shape(4, pixel).unwrap()));
      });
  }

  /// Iterate over the pixels of the image to apply a function on each pixel.
  pub fn mut_pixels<F>(&mut self, callback: F)
  where
    F: Fn(ArrayViewMut1<u8>) + Send + Sync,
  {
    self
      .colors
      .axis_chunks_iter_mut(Axis(0), 4)
      .into_par_iter()
      .for_each(|pixel| callback(pixel));
  }

  /// Get the area of the image.
  pub fn area(&self) -> Area {
    Area::rect((0, 0), (self.width, self.height))
  }
}

impl Clone for Image {
  fn clone(&self) -> Image {
    Image {
      width: self.width,
      height: self.height,
      color_len: self.color_len,
      colors: self.colors.clone(),
      anti_aliasing_level: self.anti_aliasing_level,
    }
  }
}

impl Crop for Image {
  fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> &mut Self {
    crop(self, x, y, width, height);
    self
  }
}

impl Resize for Image {
  fn resize(&mut self, p_width: u32, p_height: u32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    crate::transform::resize(self, p_width, p_height, algorithm);
    self
  }

  fn resize_percentage(&mut self, percentage: f32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    crate::transform::resize_percentage(self, percentage, algorithm);
    self
  }

  fn resize_width(&mut self, p_width: u32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    crate::transform::width(self, p_width, algorithm);
    self
  }

  fn resize_height(&mut self, p_height: u32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    crate::transform::height(self, p_height, algorithm);
    self
  }

  fn resize_width_relative(&mut self, p_width: i32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    crate::transform::width_relative(self, p_width, algorithm);
    self
  }

  fn resize_height_relative(&mut self, p_height: i32, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    crate::transform::height_relative(self, p_height, algorithm);
    self
  }
}

impl Rotate for Image {
  fn rotate(&mut self, degrees: impl Into<f64>, algorithm: impl Into<Option<TransformAlgorithm>>) -> &mut Self {
    crate::transform::rotate(self, degrees, algorithm);
    self
  }

  fn flip_horizontal(&mut self) -> &mut Self {
    crate::transform::horizontal(self);
    self
  }

  fn flip_vertical(&mut self) -> &mut Self {
    crate::transform::vertical(self);
    self
  }
}
