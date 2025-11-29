use core::ops::{Add, Div, Mul, Sub};
use ndarray::{Array1, Axis};
use rayon::prelude::*;
use std::sync::Arc;

use crate::channels::Channels;
use crate::color::Color;

/// Minimal Image type with RGBA buffer representation (Arc-backed for cheap cloning).
///
/// This structure holds pixel data in a contiguous RGBA buffer and is designed
/// to be lightweight to clone via an `Arc` reference to the underlying pixel
/// array. When a mutation happens, the buffer will be cloned on write using
/// `Arc::make_mut` (copy-on-write semantics).
///
/// ```ignore
/// let mut img = Image::new(64, 64);
/// img.clear_color(Color::from_rgba(255, 255, 255, 255));
/// ```
#[derive(Debug, Clone)]
pub struct Image {
  width: u32,
  height: u32,
  #[allow(unused)]
  color_len: u32,
  colors: Arc<Array1<u8>>,
  pub anti_aliasing_level: u32,
}

impl Image {
  /// Create a new empty image with the given width and height.
  ///
  /// - `p_width`: The width of the image in pixels.
  /// - `p_height`: The height of the image in pixels.
  ///
  /// The new image will have a zero-initialized RGBA buffer.
  pub fn new(p_width: impl TryInto<u32>, p_height: impl TryInto<u32>) -> Image {
    let width = p_width.try_into().ok().unwrap_or(0);
    let height = p_height.try_into().ok().unwrap_or(0);
    let colors = Arc::new(Array1::zeros(width as usize * height as usize * 4));
    Image {
      width,
      height,
      color_len: width * height * 4,
      colors,
      anti_aliasing_level: 4,
    }
  }

  /// Create a new image from an owned pixel buffer.
  ///
  /// - `p_width`: The width of the image in pixels.
  /// - `p_height`: The height of the image in pixels.
  /// - `p_pixels`: The pixel buffer, either RGB or RGBA depending on `p_channels`.
  /// - `p_channels`: The channel format of the input pixel buffer.
  ///
  /// This function consumes the provided `Vec<u8>` and avoids extra copies when
  /// possible.
  pub fn new_from_pixels(p_width: u32, p_height: u32, p_pixels: Vec<u8>, p_channels: Channels) -> Image {
    let mut img = Image::new(p_width, p_height);
    match p_channels {
      Channels::RGBA => img.set_rgba_owned(p_pixels),
      Channels::RGB => img.set_rgb_owned(p_pixels),
    }
    img
  }

  /// Create a new image with a solid color fill.
  ///
  /// - `p_width`: The width of the image in pixels.
  /// - `p_height`: The height of the image in pixels.
  /// - `color`: The color to fill the image with.
  ///
  /// ```ignore
  /// let img = Image::new_from_color(100, 100, Color::from_rgba(255, 0, 0, 255));
  /// ```
  pub fn new_from_color(p_width: u32, p_height: u32, color: Color) -> Image {
    let mut img = Image::new(p_width, p_height);
    img.clear_color(color);
    img
  }

  /// Return a zeroed RGBA Vec<u8> the same size as this image.
  ///
  /// Useful when you need a scratch buffer for pixel operations.
  pub fn empty_pixel_vec(&self) -> Vec<u8> {
    vec![0; (self.width * self.height) as usize * 4]
  }

  /// Fill the entire image with a solid color.
  ///
  /// - `p_color`: The color to fill into every pixel.
  pub fn clear_color(&mut self, p_color: Color) {
    let size = (self.width * self.height) as usize;
    let mut pixels = Vec::with_capacity(size * 4);
    for _ in 0..size {
      pixels.push(p_color.r);
      pixels.push(p_color.g);
      pixels.push(p_color.b);
      pixels.push(p_color.a);
    }
    *Arc::make_mut(&mut self.colors) = Array1::from_shape_vec(size * 4, pixels).unwrap();
  }

  /// Replace the pixel buffer with the provided slice of RGBA data.
  ///
  /// The slice must contain exactly `width * height * 4` bytes or this will panic.
  /// This operation performs a copy from `p_data` into the internal buffer.
  pub fn set_rgba(&mut self, p_data: &[u8]) {
    *Arc::make_mut(&mut self.colors) =
      Array1::from_shape_vec(self.width as usize * self.height as usize * 4, p_data.to_vec()).unwrap();
  }

  /// Replace the pixel buffer by taking ownership of an RGBA `Vec<u8>`.
  ///
  /// This avoids an additional copy of the input vector when possible.
  pub fn set_rgba_owned(&mut self, p_data: Vec<u8>) {
    *Arc::make_mut(&mut self.colors) =
      Array1::from_shape_vec(self.width as usize * self.height as usize * 4, p_data).unwrap();
  }

  /// Set the pixel buffer from an RGB buffer, preserving existing alpha values.
  ///
  /// - `p_data`: Owned RGB data (3 channels per pixel). The length must equal
  ///   `width * height * 3` or this function will panic.
  ///
  /// This method copies the image data and reconstructs an RGBA buffer by
  /// combining RGB channels with the alpha channel from the current image.
  pub fn set_rgb_owned(&mut self, p_data: Vec<u8>) {
    let (width, height) = self.dimensions::<usize>();
    if p_data.len() != width * height * 3 {
      panic!("Trying to set {} pixels into an image with {} pixels.", p_data.len(), self.width * self.height * 3);
    }
    let current = self.colors.to_vec();
    let new_data: Vec<u8> = p_data
      .par_chunks(3)
      .zip(current.par_chunks(4))
      .flat_map_iter(|(rgb, a)| [rgb[0], rgb[1], rgb[2], a[3]])
      .collect();
    *Arc::make_mut(&mut self.colors) =
      Array1::from_shape_vec(self.width as usize * self.height as usize * 4, new_data).unwrap();
  }

  /// Replace the image contents and resize to the specified dimensions using
  /// the provided pixel data which may be either RGB or RGBA.
  ///
  /// - `p_data`: A pixel buffer as RGB or RGBA.
  /// - `p_width`: New width.
  /// - `p_height`: New height.
  ///
  /// This will panic if `p_data` length does not match either `width*height*3` or
  /// `width*height*4`.
  pub fn set_new_pixels(&mut self, p_data: &[u8], p_width: u32, p_height: u32) {
    let is_rgba = p_data.len() == p_width as usize * p_height as usize * 4;
    let is_rgb = p_data.len() == p_width as usize * p_height as usize * 3;
    let channels = if is_rgba {
      4
    } else if is_rgb {
      3
    } else {
      panic!(
        "Invalid pixel data size, expected {} (rgba) or {} (rgb) but got {}",
        p_width * p_height * 4,
        p_width * p_height * 3,
        p_data.len()
      );
    };

    self.width = p_width;
    self.height = p_height;
    *Arc::make_mut(&mut self.colors) = Array1::zeros(p_width as usize * p_height as usize * 4);
    let mut pixels = p_data.to_vec();
    if channels == 3 {
      pixels = pixels.par_chunks(3).flat_map(|p| vec![p[0], p[1], p[2], 255]).collect();
    }
    self.set_rgba_owned(pixels);
  }

  /// Read the pixel at the specified coordinates.
  ///
  /// Returns `Some((r,g,b,a))` when the coordinates are inside the image bounds
  /// and `None` otherwise.
  pub fn get_pixel(&self, p_x: u32, p_y: u32) -> Option<(u8, u8, u8, u8)> {
    let index = ((p_y * self.width + p_x) as usize) * 4;
    if index + 3 >= self.colors.len() {
      return None;
    }
    Some((self.colors[index], self.colors[index + 1], self.colors[index + 2], self.colors[index + 3]))
  }

  /// Set the pixel at the specified coordinates to the given RGBA value.
  ///
  /// # Panics
  /// Panics if the coordinates are out of bounds (attempts to write past the
  /// underlying buffer will cause a panic through indexing).
  pub fn set_pixel(&mut self, p_x: u32, p_y: u32, pixel: (u8, u8, u8, u8)) {
    let index = (p_y * self.width + p_x) as usize * 4;
    let arr = Arc::make_mut(&mut self.colors);
    arr[index] = pixel.0;
    arr[index + 1] = pixel.1;
    arr[index + 2] = pixel.2;
    arr[index + 3] = pixel.3;
  }

  /// Set the pixels of the image from another image into their respective channels at a specific position.
  /// - `src`: The source image to get the pixels from.
  /// - `point`: The (x, y) destination coordinates to start setting the pixels.
  /// Copy pixels from a source image into this image at the specified point.
  ///
  /// - `p_src`: The source `Image` to copy from.
  /// - `p_point`: The destination `(x,y)` coordinates in this image where the
  ///   top-left of the source should be placed. Negative values are allowed and
  ///   will clip the source accordingly.
  pub fn set_from(&mut self, p_src: &Image, p_point: (i32, i32)) {
    let dest_x = p_point.0 as i32;
    let dest_y = p_point.1 as i32;

    for y in 0..p_src.height as i32 {
      for x in 0..p_src.width as i32 {
        let target_x = dest_x + x;
        let target_y = dest_y + y;
        if target_x >= 0 && target_y >= 0 && target_x < self.width as i32 && target_y < self.height as i32 {
          if let Some(pixel) = p_src.get_pixel(x as u32, y as u32) {
            self.set_pixel(target_x as u32, target_y as u32, pixel);
          }
        }
      }
    }
  }

  /// Borrow the internal RGBA buffer slice for read-only access.
  ///
  /// This avoids cloning the buffer for read-only operations.
  pub fn rgba(&self) -> &[u8] {
    self.colors.as_slice().expect("Image colors must be contiguous")
  }

  /// Copy the pixel buffer (`Arc`) from another `Image` into this one.
  ///
  /// This performs a cheap `Arc` clone: the buffer will be shared until one of
  /// the images mutates it (copy-on-write).
  pub fn copy_channel_data(&mut self, p_src: &Image) {
    self.colors = p_src.colors.clone();
  }

  /// Get a mutable reference to the internal pixel buffer as an `ndarray`.
  ///
  /// This triggers copy-on-write if the underlying buffer is shared.
  pub fn colors(&mut self) -> &mut Array1<u8> {
    Arc::make_mut(&mut self.colors)
  }

  /// Return a cloned, owned Vec<u8> containing the RGBA pixels for this image.
  pub fn to_rgba_vec(&self) -> Vec<u8> {
    self.colors.to_vec()
  }

  /// Consume the Image and return the underlying RGBA Vec<u8>.
  ///
  /// If the underlying `Arc` is shared, the buffer will be cloned and returned.
  pub fn into_rgba_vec(self) -> Vec<u8> {
    match Arc::try_unwrap(self.colors) {
      Ok(arr) => arr.to_vec(),
      Err(arc) => arc.to_vec(),
    }
  }

  /// Return an owned Vec<u8> containing only the RGB channels (no alpha).
  pub fn rgb(&self) -> Vec<u8> {
    self
      .colors
      .axis_chunks_iter(Axis(0), 4)
      .into_par_iter()
      .map(|row| row.iter().take(3).copied().collect::<Vec<_>>())
      .flatten()
      .collect()
  }

  /// Return the image dimensions as a tuple of `T` (generic integer type).
  ///
  /// - `T`: The integer type to convert the dimensions to (for example `usize`).
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u64>,
    <T as TryFrom<u64>>::Error: std::fmt::Debug,
  {
    let width = T::try_from(self.width as u64).unwrap();
    let height = T::try_from(self.height as u64).unwrap();
    (width, height)
  }

  // Channel mutators similar to core
  /// Mutate the R/G/B channels for each pixel using the provided callback.
  ///
  /// The callback receives a single channel value and should return the new
  /// channel value.
  pub fn mut_channels_rgb<F>(&mut self, p_callback: F)
  where
    F: Fn(u8) -> u8 + Send + Sync,
  {
    Arc::make_mut(&mut self.colors)
      .axis_chunks_iter_mut(Axis(0), 4)
      .into_par_iter()
      .for_each(|mut row| {
        row.iter_mut().take(3).for_each(|pixel| *pixel = p_callback(*pixel));
      });
  }

  /// Iterate over a specific channel of the image to apply a function on each pixel of that channel.
  /// - `p_channel`: The channel to modify ("r", "g", "b", or "a").
  /// Mutate a single channel for each pixel using the provided callback.
  ///
  /// - `p_channel`: One of "r", "g", "b", or "a" indicating which channel to mutate.
  /// - `p_callback`: Callback that receives the old channel value and returns a new one.
  pub fn mut_channel<F>(&mut self, p_channel: impl Into<String>, p_callback: F)
  where
    F: Fn(u8) -> u8 + Send + Sync,
  {
    let channel = p_channel.into();
    Arc::make_mut(&mut self.colors)
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

  /// Iterate over each pixel and apply a callback with an ndarray `ArrayViewMut1<u8>`.
  ///
  /// Recommended for per-pixel processing and operations that need access to
  /// all channels simultaneously.
  pub fn mut_pixels<F>(&mut self, p_callback: F)
  where
    F: Fn(ndarray::ArrayViewMut1<u8>) + Send + Sync,
  {
    Arc::make_mut(&mut self.colors)
      .axis_chunks_iter_mut(Axis(0), 4)
      .into_par_iter()
      .for_each(|pixel| p_callback(pixel));
  }

  /// Iterate over each pixel buffer chunk for SIMD-friendly operations.
  ///
  /// It provides the pixel as an `ArrayViewMut1<u8>` and is intended for
  /// callbacks that can operate on the full 4-channel pixel at once.
  pub fn mut_pixels_simd<F>(&mut self, p_callback: F)
  where
    F: Fn(ndarray::ArrayViewMut1<u8>) + Send + Sync,
  {
    Arc::make_mut(&mut self.colors)
      .axis_chunks_iter_mut(Axis(0), 4)
      .into_par_iter()
      .for_each(|row| {
        p_callback(row);
      });
  }

  #[cfg(test)]
  /// For tests: return a raw pointer to the underlying buffer for pointer comparison
  /// between clones to verify copy-on-write behavior.
  pub fn buffer_ptr(&self) -> *const u8 {
    self
      .colors
      .as_slice()
      .expect("Image colors must be contiguous")
      .as_ptr()
  }
}

impl<T: Into<f32>> Mul<T> for &mut Image {
  type Output = Image;

  /// Multiply each pixel channel by a scalar factor.
  ///
  /// This performs per-channel multiplication and clamps the results to [0,255].
  fn mul(self, rhs: T) -> Self::Output {
    let rhs = rhs.into();
    self.mut_channels_rgb(|pixel| {
      let value = (pixel as f32 * rhs).round();
      value.clamp(0.0, 255.0) as u8
    });
    self.clone()
  }
}

impl<T: Into<f32>> Sub<T> for &mut Image {
  type Output = Image;

  /// Subtract a scalar value from each pixel channel.
  ///
  /// This performs per-channel subtraction and clamps the results to [0,255].
  fn sub(self, rhs: T) -> Self::Output {
    let rhs = rhs.into();
    self.mut_channels_rgb(|pixel| {
      let value = (pixel as f32 - rhs).round();
      value.clamp(0.0, 255.0) as u8
    });
    self.clone()
  }
}

impl<T: Into<f32>> Div<T> for &mut Image {
  type Output = Image;

  /// Divide each pixel channel by a scalar value.
  ///
  /// This performs per-channel division and clamps the results to [0,255].
  fn div(self, rhs: T) -> Self::Output {
    let rhs = rhs.into();
    self.mut_channels_rgb(|pixel| {
      let value = (pixel as f32 / rhs).round();
      value.clamp(0.0, 255.0) as u8
    });
    self.clone()
  }
}

impl<T: Into<f32>> Add<T> for &mut Image {
  type Output = Image;

  /// Add a scalar value to each pixel channel.
  ///
  /// This performs per-channel addition and clamps the results to [0,255].
  fn add(self, rhs: T) -> Self::Output {
    let rhs = rhs.into();
    self.mut_channels_rgb(|pixel| {
      let value = (pixel as f32 + rhs).round();
      value.clamp(0.0, 255.0) as u8
    });
    self.clone()
  }
}
