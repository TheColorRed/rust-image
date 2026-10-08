use core::ops::{Add, Div, Mul, Sub};
use ndarray::{Array1, Axis};
use rayon::prelude::*;
use std::any::Any;
use std::borrow::Cow;
use std::sync::{Arc, OnceLock};

use crate::channels::{Channel, Channels};
use crate::color::Color;
use crate::resolution::Resolution;

/// Pixels that are described but not made yet, such as the result of a chain of GPU passes.
///
/// An [`Image`] can stand for work that has not run: whoever made it keeps the recipe, and the first time its pixels are
/// read the image asks for them once and remembers them. Code that never reads the pixels, such as a preview that draws the
/// recipe on the GPU, never pays for them. This crate knows nothing about GPUs; whatever implements this does.
pub trait DeferredPixels: Send + Sync + std::fmt::Debug {
  /// The `width * height * 4` RGBA pixels. This must always succeed: if the fast way to make them fails, such as a GPU that
  /// went away, it makes them the slow way, so the picture is never lost.
  fn render(&self) -> Vec<u8>;

  /// This as `Any`, so whoever made the recipe can recognise it again and use it without making the pixels.
  fn as_any(&self) -> &dyn Any;
}

/// A [`DeferredPixels`] and the pixels it made, once they have been asked for. Clones of an image share one of these, so
/// the work runs once however many of them are read.
#[derive(Debug)]
struct Deferred {
  source: Arc<dyn DeferredPixels>,
  made: OnceLock<Arc<Array1<u8>>>,
}

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
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct Image {
  width: u32,
  height: u32,
  /// The pixels. Empty while `deferred` is set; read them through [`Image::ready`], never directly.
  colors: Arc<Array1<u8>>,
  /// Pixels not made yet. Set by [`Image::from_deferred`] and cleared by anything that changes the image.
  deferred: Option<Arc<Deferred>>,
  pub anti_aliasing_level: u32,
  resolution: Resolution,
}

// What other languages can call on an image. A color is passed as a handle there, so `new_from_color` has its own
// version here under the same name.
#[cfg(feature = "uniffi")]
#[uniffi::export]
impl Image {
  /// Create a new image with a solid color fill.
  #[uniffi::constructor(name = "new_from_color")]
  pub fn uniffi_new_from_color(p_width: u32, p_height: u32, p_color: Arc<Color>) -> Image {
    Image::new_from_color(p_width, p_height, *p_color)
  }

  /// The width of the image in pixels.
  pub fn width(&self) -> u32 {
    self.width
  }

  /// The height of the image in pixels.
  pub fn height(&self) -> u32 {
    self.height
  }
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
      colors,
      deferred: None,
      anti_aliasing_level: 4,
      resolution: Resolution::default(),
    }
  }

  /// Create a new image from a pixel buffer.
  ///
  /// - `p_width`: The width of the image in pixels.
  /// - `p_height`: The height of the image in pixels.
  /// - `p_pixels`: The pixel buffer, either RGB or RGBA depending on `p_channels`. Pass an owned
  ///   `Vec<u8>` to move it in without copying, or a `&[u8]` / `&Vec<u8>` to copy it.
  /// - `p_channels`: The channel layout of `p_pixels`. RGB input is made fully opaque.
  ///
  /// Panics if the buffer length does not match `p_width * p_height * channels`.
  pub fn new_from_pixels<'a>(
    p_width: u32, p_height: u32, p_pixels: impl Into<Cow<'a, [u8]>>, p_channels: Channels,
  ) -> Image {
    let mut img = Image::new(0, 0);
    img.set_pixels(p_width, p_height, p_pixels, p_channels);
    img
  }

  /// Create a new image with a solid color fill.
  ///
  /// - `p_width`: The width of the image in pixels.
  /// - `p_height`: The height of the image in pixels.
  /// - `p_color`: The color to fill the image with.
  ///
  /// ```ignore
  /// let img = Image::new_from_color(100, 100, Color::from_rgba(255, 0, 0, 255));
  /// ```
  pub fn new_from_color(p_width: u32, p_height: u32, p_color: Color) -> Image {
    let mut img = Image::new(p_width, p_height);
    img.clear_color(p_color);
    img
  }

  /// An image of `p_width` x `p_height` whose pixels `p_source` makes when they are first read.
  ///
  /// Nothing runs here. Reading the pixels (`rgba`, `get_pixel`, saving) makes them once, and changing the image makes them
  /// first and then edits them as usual, so code that does not know about deferred images works with one unchanged.
  pub fn from_deferred(p_width: u32, p_height: u32, p_source: Arc<dyn DeferredPixels>) -> Image {
    let mut image = Image::new(0, 0);
    image.width = p_width;
    image.height = p_height;
    image.deferred = Some(Arc::new(Deferred {
      source: p_source,
      made: OnceLock::new(),
    }));
    image
  }

  /// The recipe for the pixels if they have not been asked for yet, `None` for an image that has its pixels. An image that was
  /// read already has them, so its recipe is not offered again.
  pub fn deferred(&self) -> Option<&Arc<dyn DeferredPixels>> {
    self.deferred.as_ref().filter(|deferred| deferred.made.get().is_none()).map(|deferred| &deferred.source)
  }

  /// Whether this image and `p_other` are known to hold the very same pixels, because one was cloned from the other. An image
  /// whose pixels have not been made yet shares nothing, and neither does one whose pixels merely look the same.
  pub fn shares_pixels_with(&self, p_other: &Image) -> bool {
    let made = |image: &Image| match &image.deferred {
      None => Some(image.colors.clone()),
      Some(deferred) => deferred.made.get().cloned(),
    };
    matches!((made(self), made(p_other)), (Some(a), Some(b)) if Arc::ptr_eq(&a, &b))
  }

  /// The pixels, making them first if this image is deferred.
  fn ready(&self) -> &Arc<Array1<u8>> {
    let Some(deferred) = &self.deferred else { return &self.colors };
    deferred.made.get_or_init(|| {
      let mut pixels = deferred.source.render();
      debug_assert_eq!(pixels.len(), self.pixel_count() * 4, "a deferred image made pixels of the wrong size");
      pixels.resize(self.pixel_count() * 4, 0);
      Arc::new(Array1::from_vec(pixels))
    })
  }

  /// Turns a deferred image into one that has its pixels, ready to be changed. Does nothing for any other image.
  fn make_ready(&mut self) {
    if self.deferred.is_some() {
      self.colors = Arc::clone(self.ready());
      self.deferred = None;
    }
  }

  /// Return a zeroed RGBA Vec<u8> the same size as this image.
  ///
  /// Useful when you need a scratch buffer for pixel operations.
  pub fn empty_pixel_vec(&self) -> Vec<u8> {
    vec![0; self.pixel_count() * 4]
  }

  /// Returns the physical pixel density metadata.
  pub fn resolution(&self) -> Resolution {
    self.resolution
  }

  /// Updates physical pixel density metadata without resizing the image.
  pub fn set_resolution(&mut self, p_resolution: Resolution) {
    self.resolution = p_resolution;
  }

  /// Fill the entire image with a solid color.
  ///
  /// - `p_color`: The color to fill into every pixel.
  pub fn clear_color(&mut self, p_color: Color) {
    let rgba = [p_color.r, p_color.g, p_color.b, p_color.a];
    let pixel_count = self.pixel_count();
    // Everything is overwritten, so a deferred image does not need its pixels made first.
    if self.deferred.take().is_some() {
      self.colors = Arc::new(Array1::from_vec(rgba.repeat(pixel_count)));
      return;
    }
    match Arc::get_mut(&mut self.colors).and_then(|colors| colors.as_slice_mut()) {
      // The buffer is not shared, so fill it in place without reallocating.
      Some(pixels) if pixels.len() == pixel_count * 4 => {
        pixels.par_chunks_exact_mut(4).for_each(|pixel| pixel.copy_from_slice(&rgba));
      }
      _ => self.colors = Arc::new(Array1::from_vec(rgba.repeat(pixel_count))),
    }
  }

  /// Replace the image contents, resizing to the given dimensions.
  ///
  /// - `p_width`: The new width in pixels.
  /// - `p_height`: The new height in pixels.
  /// - `p_pixels`: The pixel buffer, either RGB or RGBA depending on `p_channels`. Pass an owned
  ///   `Vec<u8>` to move it in without copying, or a `&[u8]` / `&Vec<u8>` to copy it.
  /// - `p_channels`: The channel layout of `p_pixels`. RGB input is made fully opaque.
  ///
  /// Panics if the buffer length does not match `p_width * p_height * channels`.
  pub fn set_pixels<'a>(
    &mut self, p_width: u32, p_height: u32, p_pixels: impl Into<Cow<'a, [u8]>>, p_channels: Channels,
  ) {
    let pixels = p_pixels.into();
    let pixel_count = p_width as usize * p_height as usize;
    assert_eq!(
      pixels.len(),
      pixel_count * p_channels.bytes_per_pixel(),
      "Pixel data length mismatch for a {}x{} {:?} image",
      p_width,
      p_height,
      p_channels
    );

    let rgba = match p_channels {
      Channels::RGBA => pixels.into_owned(),
      Channels::RGB => {
        let mut rgba = vec![255u8; pixel_count * 4];
        rgba
          .par_chunks_exact_mut(4)
          .zip(pixels.par_chunks_exact(3))
          .for_each(|(dst, src)| dst[..3].copy_from_slice(src));
        rgba
      }
    };

    self.width = p_width;
    self.height = p_height;
    self.deferred = None;
    // Replace the Arc instead of mutating through it so a shared buffer is never cloned just to be overwritten.
    self.colors = Arc::new(Array1::from_vec(rgba));
  }

  /// Replace the pixel buffer with RGBA data of the same dimensions.
  ///
  /// Shorthand for [`Image::set_pixels`] when the size is unchanged.
  ///
  /// - `p_pixels`: RGBA pixels. Pass an owned `Vec<u8>` to move it in without copying.
  pub fn set_rgba<'a>(&mut self, p_pixels: impl Into<Cow<'a, [u8]>>) {
    self.set_pixels(self.width, self.height, p_pixels, Channels::RGBA);
  }

  /// Read the pixel at the specified coordinates.
  ///
  /// Returns `Some((r,g,b,a))` when the coordinates are inside the image bounds
  /// and `None` otherwise.
  pub fn get_pixel(&self, p_x: u32, p_y: u32) -> Option<(u8, u8, u8, u8)> {
    if p_x >= self.width || p_y >= self.height {
      return None;
    }
    let index = (p_y as usize * self.width as usize + p_x as usize) * 4;
    let colors = self.ready();
    Some((colors[index], colors[index + 1], colors[index + 2], colors[index + 3]))
  }

  /// Get the RGBA pixel data for a specified rectangular area.
  pub fn get_pixels(&self, p_area: (u32, u32, u32, u32)) -> Vec<(u8, u8, u8, u8)> {
    let (x, y, width, height) = p_area;
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for j in 0..height {
      for i in 0..width {
        pixels.push(self.get_pixel(x + i, y + j).unwrap_or((0, 0, 0, 0)));
      }
    }
    pixels
  }

  /// Set the pixel at the specified coordinates to the given RGBA value.
  ///
  /// # Panics
  /// Panics if the coordinates are out of bounds (attempts to write past the
  /// underlying buffer will cause a panic through indexing).
  pub fn set_pixel(&mut self, p_x: u32, p_y: u32, p_pixel: (u8, u8, u8, u8)) {
    debug_assert!(p_x < self.width && p_y < self.height, "Pixel ({}, {}) is out of bounds", p_x, p_y);
    let index = (p_y as usize * self.width as usize + p_x as usize) * 4;
    self.make_ready();
    let arr = Arc::make_mut(&mut self.colors);
    arr[index] = p_pixel.0;
    arr[index + 1] = p_pixel.1;
    arr[index + 2] = p_pixel.2;
    arr[index + 3] = p_pixel.3;
  }

  /// Copy pixels from a source image into this image at the specified point.
  ///
  /// Pixels are replaced, not blended.
  ///
  /// - `p_src`: The source `Image` to copy from.
  /// - `p_point`: The destination `(x,y)` coordinates in this image where the
  ///   top-left of the source should be placed. Negative values are allowed and
  ///   will clip the source accordingly.
  pub fn draw_image_at(&mut self, p_src: &Image, p_point: (i32, i32)) {
    let (dst_w, dst_h) = (self.width as i64, self.height as i64);
    let (src_w, src_h) = (p_src.width as i64, p_src.height as i64);
    let (dx, dy) = (p_point.0 as i64, p_point.1 as i64);

    // Clip the source rectangle against the destination bounds.
    let (x0, x1) = (dx.max(0), (dx + src_w).min(dst_w));
    let (y0, y1) = (dy.max(0), (dy + src_h).min(dst_h));
    if x0 >= x1 || y0 >= y1 {
      return;
    }

    let row_bytes = ((x1 - x0) * 4) as usize;
    let src = p_src.rgba();
    self
      .pixels_mut()
      .par_chunks_exact_mut(dst_w as usize * 4)
      .enumerate()
      .skip(y0 as usize)
      .take((y1 - y0) as usize)
      .for_each(|(y, row)| {
        let src_start = (((y as i64 - dy) * src_w + (x0 - dx)) * 4) as usize;
        let dst_start = (x0 * 4) as usize;
        row[dst_start..dst_start + row_bytes].copy_from_slice(&src[src_start..src_start + row_bytes]);
      });
  }

  /// Borrow the internal RGBA buffer slice for read-only access.
  ///
  /// This avoids cloning the buffer for read-only operations.
  pub fn rgba(&self) -> &[u8] {
    self.ready().as_slice().expect("Image colors must be contiguous")
  }

  /// Copy the pixel buffer (`Arc`) from another `Image` into this one.
  ///
  /// This performs a cheap `Arc` clone: the buffer will be shared until one of
  /// the images mutates it (copy-on-write).
  pub fn copy_channel_data(&mut self, p_src: &Image) {
    self.colors = Arc::clone(p_src.ready());
    self.deferred = None;
  }

  /// Get a mutable reference to the internal pixel buffer as an `ndarray`.
  ///
  /// This triggers copy-on-write if the underlying buffer is shared.
  pub fn colors(&mut self) -> &mut Array1<u8> {
    self.make_ready();
    Arc::make_mut(&mut self.colors)
  }

  /// Return a cloned, owned Vec<u8> containing the RGBA pixels for this image.
  pub fn to_rgba_vec(&self) -> Vec<u8> {
    self.ready().to_vec()
  }

  /// Consume the Image and return the underlying RGBA Vec<u8>.
  ///
  /// The buffer is moved out without copying unless the underlying `Arc` is shared.
  pub fn into_rgba_vec(mut self) -> Vec<u8> {
    self.make_ready();
    match Arc::try_unwrap(self.colors) {
      Ok(arr) => {
        let (pixels, offset) = arr.into_raw_vec_and_offset();
        debug_assert_eq!(offset.unwrap_or(0), 0, "Image colors must start at offset 0");
        pixels
      }
      Err(arc) => arc.to_vec(),
    }
  }

  /// Return an owned Vec<u8> containing only the RGB channels (no alpha).
  pub fn rgb(&self) -> Vec<u8> {
    let rgba = self.rgba();
    let mut rgb = vec![0u8; rgba.len() / 4 * 3];
    rgb.par_chunks_exact_mut(3).zip(rgba.par_chunks_exact(4)).for_each(|(dst, src)| dst.copy_from_slice(&src[..3]));
    rgb
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

  /// Mutate the given channels of every pixel using the provided callback.
  ///
  /// - `p_channels`: The channels to mutate, e.g. `Channel::RGB`, `Channel::RGBA`, or `[Channel::A]`.
  /// - `p_callback`: Callback that receives the old channel value and returns a new one.
  ///
  /// ```ignore
  /// image.mut_channels(Channel::RGB, |value| 255 - value); // invert
  /// image.mut_channels([Channel::A], |value| value / 2); // halve opacity
  /// ```
  pub fn mut_channels<F>(&mut self, p_channels: impl AsRef<[Channel]>, p_callback: F)
  where
    F: Fn(u8) -> u8 + Send + Sync,
  {
    let mut mask = [false; 4];
    for channel in p_channels.as_ref() {
      mask[channel.index()] = true;
    }
    self.pixels_mut().par_chunks_exact_mut(4).for_each(|pixel| {
      for (value, enabled) in pixel.iter_mut().zip(mask) {
        if enabled {
          *value = p_callback(*value);
        }
      }
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
    self.make_ready();
    Arc::make_mut(&mut self.colors)
      .axis_chunks_iter_mut(Axis(0), 4)
      .into_par_iter()
      .for_each(|pixel| p_callback(pixel));
  }

  /// Applies `p_op` to the red, green, and blue channels of every pixel, clamping to 0-255.
  fn apply_scalar_rgb(&mut self, p_op: impl Fn(f32) -> f32 + Send + Sync) {
    self.pixels_mut().par_chunks_exact_mut(4).for_each(|pixel| {
      for value in &mut pixel[..3] {
        *value = p_op(*value as f32).clamp(0.0, 255.0) as u8;
      }
    });
  }

  fn pixel_count(&self) -> usize {
    self.width as usize * self.height as usize
  }

  /// Mutable RGBA slice, triggering copy-on-write if the buffer is shared.
  fn pixels_mut(&mut self) -> &mut [u8] {
    self.make_ready();
    Arc::make_mut(&mut self.colors).as_slice_mut().expect("Image colors must be contiguous")
  }
}

/// Implements a scalar arithmetic operator on `&mut Image` that applies to the RGB channels
/// (alpha is untouched) and clamps the results to 0-255.
macro_rules! impl_scalar_op {
  ($trait:ident, $method:ident, $op:tt, $doc:literal) => {
    impl<T: Into<f32>> $trait<T> for &mut Image {
      type Output = ();

      #[doc = $doc]
      fn $method(self, p_rhs: T) {
        let rhs = p_rhs.into();
        self.apply_scalar_rgb(|value| value $op rhs);
      }
    }
  };
}

impl_scalar_op!(Add, add, +, "Add a scalar value to each RGB channel.");
impl_scalar_op!(Sub, sub, -, "Subtract a scalar value from each RGB channel.");
impl_scalar_op!(Mul, mul, *, "Multiply each RGB channel by a scalar factor.");
impl_scalar_op!(Div, div, /, "Divide each RGB channel by a scalar value.");

#[cfg(test)]
mod tests {
  use super::*;

  /// A recipe that counts how many times it is made and can be told to make the wrong amount.
  #[derive(Debug)]
  struct Counting {
    made: std::sync::atomic::AtomicUsize,
    pixels: Vec<u8>,
  }

  impl DeferredPixels for Counting {
    fn render(&self) -> Vec<u8> {
      self.made.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
      self.pixels.clone()
    }

    fn as_any(&self) -> &dyn Any {
      self
    }
  }

  fn counting(p_pixels: Vec<u8>) -> Arc<Counting> {
    Arc::new(Counting {
      made: Default::default(),
      pixels: p_pixels,
    })
  }

  fn made(p_source: &Counting) -> usize {
    p_source.made.load(std::sync::atomic::Ordering::SeqCst)
  }

  #[test]
  fn a_deferred_image_makes_nothing_until_its_pixels_are_read_and_then_only_once() {
    let source = counting(vec![10, 20, 30, 255, 40, 50, 60, 255]);
    let image = Image::from_deferred(2, 1, source.clone());
    let clone = image.clone();
    assert_eq!(image.dimensions::<u32>(), (2, 1));
    assert!(image.deferred().is_some());
    assert_eq!(made(&source), 0, "nothing runs until the pixels are read");
    assert_eq!(image.get_pixel(1, 0), Some((40, 50, 60, 255)));
    assert_eq!(clone.rgba(), &[10, 20, 30, 255, 40, 50, 60, 255]);
    assert_eq!(image.to_rgba_vec().len(), 8);
    assert_eq!(made(&source), 1, "the clones share one run");
    assert!(image.deferred().is_none(), "once the pixels exist the recipe is not offered again");
  }

  #[test]
  fn changing_a_deferred_image_makes_its_pixels_first_and_leaves_its_clones_alone() {
    let source = counting(vec![10, 20, 30, 255, 40, 50, 60, 255]);
    let original = Image::from_deferred(2, 1, source.clone());
    let mut edited = original.clone();
    edited.set_pixel(0, 0, (1, 2, 3, 4));
    assert_eq!(edited.rgba(), &[1, 2, 3, 4, 40, 50, 60, 255]);
    assert_eq!(original.rgba(), &[10, 20, 30, 255, 40, 50, 60, 255]);
    assert_eq!(made(&source), 1);

    let mut inverted = Image::from_deferred(2, 1, counting(vec![10, 20, 30, 255, 40, 50, 60, 255]));
    inverted.mut_channels(Channel::RGB, |value| 255 - value);
    assert_eq!(inverted.into_rgba_vec(), vec![245, 235, 225, 255, 215, 205, 195, 255]);
  }

  #[test]
  fn replacing_a_deferred_image_does_not_make_its_pixels() {
    let source = counting(vec![0; 8]);
    let mut image = Image::from_deferred(2, 1, source.clone());
    image.clear_color(Color::from_rgba(9, 8, 7, 6));
    assert_eq!(image.rgba(), &[9, 8, 7, 6, 9, 8, 7, 6]);
    let mut other = Image::from_deferred(2, 1, source.clone());
    other.set_rgba(vec![1u8; 8]);
    assert_eq!(other.rgba(), &[1; 8]);
    assert_eq!(made(&source), 0);
  }

  #[test]
  fn clones_share_pixels_and_copies_and_unmade_images_do_not() {
    let image = Image::new_from_color(2, 2, Color::white());
    assert!(image.shares_pixels_with(&image.clone()));
    assert!(!image.shares_pixels_with(&Image::new_from_color(2, 2, Color::white())));
    let deferred = Image::from_deferred(1, 1, counting(vec![0; 4]));
    assert!(!deferred.shares_pixels_with(&deferred.clone()), "nothing is shared before the pixels exist");
    deferred.rgba();
    assert!(deferred.shares_pixels_with(&deferred.clone()));
  }

  #[test]
  fn a_recipe_can_be_recognised_again_by_whoever_made_it() {
    let source = counting(vec![0; 4]);
    let image = Image::from_deferred(1, 1, source.clone());
    let recipe = image.deferred().expect("not made yet");
    assert!(recipe.as_any().downcast_ref::<Counting>().is_some());
  }

  #[test]
  fn rgb_pixels_are_opaque() {
    let img = Image::new_from_pixels(2, 1, vec![10, 20, 30, 40, 50, 60], Channels::RGB);
    assert_eq!(img.rgba(), &[10, 20, 30, 255, 40, 50, 60, 255]);
  }

  #[test]
  fn set_pixels_accepts_slices_and_resizes() {
    let mut img = Image::new(1, 1);
    let data = [1u8, 2, 3, 4, 5, 6, 7, 8];
    img.set_pixels(1, 2, &data[..], Channels::RGBA);
    assert_eq!(img.dimensions::<u32>(), (1, 2));
    assert_eq!(img.rgba(), &data);
  }

  #[test]
  #[should_panic(expected = "Pixel data length mismatch")]
  fn set_pixels_rejects_wrong_length() {
    Image::new_from_pixels(2, 2, vec![0u8; 12], Channels::RGBA);
  }

  #[test]
  fn get_pixel_rejects_x_past_width() {
    let img = Image::new(2, 2);
    assert!(img.get_pixel(2, 0).is_none());
    assert!(img.get_pixel(1, 1).is_some());
  }

  #[test]
  fn clear_color_fills_shared_and_unique_buffers() {
    let mut img = Image::new(2, 2);
    let shared = img.clone();
    img.clear_color(Color::red());
    assert!(img.rgba().chunks_exact(4).all(|pixel| pixel == [255, 0, 0, 255]));
    assert!(shared.rgba().iter().all(|value| *value == 0));
    img.clear_color(Color::blue());
    assert!(img.rgba().chunks_exact(4).all(|pixel| pixel == [0, 0, 255, 255]));
  }

  #[test]
  fn draw_image_at_clips_negative_and_overflowing_offsets() {
    let mut dst = Image::new(3, 3);
    let src = Image::new_from_color(2, 2, Color::white());
    dst.draw_image_at(&src, (-1, 2));
    let lit: Vec<_> = (0..3)
      .flat_map(|y| (0..3).map(move |x| (x, y)))
      .filter(|(x, y)| dst.get_pixel(*x, *y).unwrap().3 == 255)
      .collect();
    assert_eq!(lit, vec![(0, 2)]);
  }

  #[test]
  fn rgb_and_into_rgba_vec_round_trip() {
    let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
    let img = Image::new_from_pixels(2, 1, data.clone(), Channels::RGBA);
    assert_eq!(img.rgb(), vec![1, 2, 3, 5, 6, 7]);
    assert_eq!(img.into_rgba_vec(), data);
  }

  #[test]
  fn mut_channels_only_touches_selected_channels() {
    let mut img = Image::new_from_pixels(1, 1, vec![10, 20, 30, 40], Channels::RGBA);
    img.mut_channels(Channel::RGB, |value| 255 - value);
    assert_eq!(img.rgba(), &[245, 235, 225, 40]);
    img.mut_channels([Channel::A], |value| value / 2);
    assert_eq!(img.rgba(), &[245, 235, 225, 20]);
  }

  #[test]
  fn scalar_ops_clamp_and_skip_alpha() {
    let mut img = Image::new_from_pixels(1, 1, vec![100, 200, 10, 50], Channels::RGBA);
    let _ = &mut img * 2.0f32;
    assert_eq!(img.rgba(), &[200, 255, 20, 50]);
    let _ = &mut img - 30.0f32;
    assert_eq!(img.rgba(), &[170, 225, 0, 50]);
  }
}
