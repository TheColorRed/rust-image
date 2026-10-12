//! `Image` from `primitives`, with `ImageExt` (reading and writing files), `Channels`, `Channel` and `Resolution`.
//!
//! Other languages hold an image as a handle and the library changes images through `&mut self`, so the handle keeps the
//! image behind a lock and changes it in place: `image.clearColor(color)` edits the image the handle holds, as in Rust.
//!
//! What is not exported, and why:
//! - `mut_channels` and `mut_pixels` take a function that runs on every pixel; a call across the language boundary for each
//!   pixel would be far slower than the library's own loop.
//! - `colors` hands out a borrowed `ndarray`, `from_deferred` and `deferred` deal in the library's internal recipe trait, and
//!   `into_rgba_vec` consumes the image: `to_rgba_vec` is the same pixels.
//! - `Channel::RGB`/`RGBA` are lists a caller writes.
//!
//! A pixel is `[r, g, b, a]`, an area is `[x, y, width, height]` and a point is `[x, y]`: the library's tuples as arrays. An
//! array of the wrong length panics, as the library does for input it cannot use.

use std::ops::{Add, Div, Mul, Sub};
use std::sync::{Arc, RwLock};

use abra::abra_core::{self, Channel, Channels, ImageExt, Resolution, Size, Transform, WriterOptions};

use crate::{Area, Color};

/// An image: RGBA pixels, or the recipe for them (see `abra_core::image::recipe`).
#[derive(Debug, uniffi::Object)]
pub struct Image(RwLock<abra_core::Image>);

// The channel layout of a pixel buffer.
#[uniffi::remote(Enum)]
pub enum Channels {
  RGBA,
  RGB,
}

// A single color channel within an RGBA pixel.
#[uniffi::remote(Enum)]
pub enum Channel {
  R,
  G,
  B,
  A,
}

// Physical pixel density metadata.
#[uniffi::remote(Record)]
pub struct Resolution {
  pub x_dpi: f32,
  pub y_dpi: f32,
}

// A width and height.
#[uniffi::remote(Record)]
pub struct Size {
  pub width: f32,
  pub height: f32,
}

// Options for saving an image.
#[uniffi::remote(Record)]
pub struct WriterOptions {
  pub quality: u8,
}

impl Image {
  /// A handle for an image of the library.
  pub fn from_core(image: abra_core::Image) -> Self {
    Self(RwLock::new(image))
  }

  /// A copy of the image of the library that this holds. Cheap: the pixels are shared until one of them changes.
  pub fn core(&self) -> abra_core::Image {
    self.0.read().unwrap().clone()
  }

  /// Changes the image in place.
  pub fn edit<R>(&self, change: impl FnOnce(&mut abra_core::Image) -> R) -> R {
    change(&mut self.0.write().unwrap())
  }
}

#[uniffi::export]
impl Image {
  /// An empty image of this size.
  #[uniffi::constructor]
  pub fn new(width: u32, height: u32) -> Self {
    Self::from_core(abra_core::Image::new(width, height))
  }

  /// An image from a pixel buffer laid out as `channels`. An RGB buffer is made fully opaque.
  #[uniffi::constructor]
  pub fn new_from_pixels(width: u32, height: u32, pixels: Vec<u8>, channels: Channels) -> Self {
    Self::from_core(abra_core::Image::new_from_pixels(width, height, pixels, channels))
  }

  /// An image of one color.
  #[uniffi::constructor]
  pub fn new_from_color(width: u32, height: u32, color: Arc<Color>) -> Self {
    Self::from_core(abra_core::Image::new_from_color(width, height, color.0))
  }

  /// Reads an image from a file.
  #[uniffi::constructor]
  pub fn read(file: String) -> Self {
    Self::from_core(abra_core::Image::read(file).unwrap_or_else(|message| panic!("{message}")))
  }

  /// Writes the image to a file; the extension chooses the format.
  pub fn write(&self, file: String, options: Option<WriterOptions>) {
    self.0.read().unwrap().write(file, options).unwrap_or_else(|message| panic!("{message}"));
  }

  /// The width in pixels.
  pub fn width(&self) -> u32 {
    self.0.read().unwrap().dimensions::<u32>().0
  }

  /// The height in pixels.
  pub fn height(&self) -> u32 {
    self.0.read().unwrap().dimensions::<u32>().1
  }

  /// The width and height.
  pub fn size(&self) -> Size {
    self.0.read().unwrap().size()
  }

  /// The physical pixel density.
  pub fn resolution(&self) -> Resolution {
    self.0.read().unwrap().resolution()
  }

  /// Sets the physical pixel density. The pixels are not resized.
  pub fn set_resolution(&self, resolution: Resolution) {
    self.edit(|image| image.set_resolution(resolution));
  }

  /// Fills the whole image with one color.
  pub fn clear_color(&self, color: Arc<Color>) {
    self.edit(|image| image.clear_color(color.0));
  }

  /// Replaces the image with these pixels, laid out as `channels`, at this size.
  pub fn set_pixels(&self, width: u32, height: u32, pixels: Vec<u8>, channels: Channels) {
    self.edit(|image| image.set_pixels(width, height, pixels, channels));
  }

  /// Replaces the pixels with RGBA pixels of the same size.
  pub fn set_rgba(&self, pixels: Vec<u8>) {
    self.edit(|image| image.set_rgba(pixels));
  }

  /// The pixel at this position as `[r, g, b, a]`, or nothing if it is outside the image.
  pub fn get_pixel(&self, x: u32, y: u32) -> Option<Vec<u8>> {
    self.0.read().unwrap().get_pixel(x, y).map(|(r, g, b, a)| vec![r, g, b, a])
  }

  /// The pixels of an area `[x, y, width, height]`, row by row, each as `[r, g, b, a]`. Positions outside the image are
  /// transparent black.
  pub fn get_pixels(&self, area: Vec<u32>) -> Vec<Vec<u8>> {
    let [x, y, width, height]: [u32; 4] = area.try_into().expect("an area takes 4 values");
    let pixels = self.0.read().unwrap().get_pixels((x, y, width, height));
    pixels.into_iter().map(|(r, g, b, a)| vec![r, g, b, a]).collect()
  }

  /// Sets the pixel at this position to `[r, g, b, a]`.
  pub fn set_pixel(&self, x: u32, y: u32, pixel: Vec<u8>) {
    let [r, g, b, a]: [u8; 4] = pixel.try_into().expect("a pixel takes 4 values");
    self.edit(|image| image.set_pixel(x, y, (r, g, b, a)));
  }

  /// Copies another image onto this one with its top left corner at `point`, `[x, y]`. Pixels are replaced, not blended, and
  /// the part outside this image is clipped.
  pub fn draw_image_at(&self, source: Arc<Image>, point: Vec<i32>) {
    let [x, y]: [i32; 2] = point.try_into().expect("a point takes 2 values");
    // The source is copied first, so drawing an image onto itself does not wait on its own lock.
    let source = source.core();
    self.edit(|image| image.draw_image_at(&source, (x, y)));
  }

  /// A copy of the image, the library's `Image::clone`. Cheap: the pixels are shared until one of the two changes.
  ///
  /// The Rust function is not named `clone`: the bridge calls an object's functions as `handle.name()`, and on the `Arc` that
  /// holds the image `clone` is `Arc::clone`, which hands back the same image and not a copy.
  #[uniffi::method(name = "clone")]
  pub fn duplicate(&self) -> Arc<Image> {
    Arc::new(Self::from_core(self.core()))
  }

  /// A copy cropped to `[x, y, width, height]`. The rectangle is clipped to the image.
  pub fn cropped(&self, x: u32, y: u32, width: u32, height: u32) -> Arc<Image> {
    let mut image = self.core();
    image.crop(x, y, width, height);
    Arc::new(Self::from_core(image))
  }

  /// Shares the pixels of another image, which this image then keeps until either one changes.
  pub fn copy_channel_data(&self, source: Arc<Image>) {
    let source = source.core();
    self.edit(|image| image.copy_channel_data(&source));
  }

  /// Whether the two images share their pixels, because one was copied from the other.
  pub fn shares_pixels_with(&self, other: Arc<Image>) -> bool {
    let other = other.core();
    self.0.read().unwrap().shares_pixels_with(&other)
  }

  /// The RGBA bytes of the pixels whose centers are inside the area, row by row. The area can be any shape.
  pub fn rgba_in_area(&self, area: Arc<Area>) -> Vec<u8> {
    self.0.read().unwrap().rgba_in_area(&area.core())
  }

  /// The pixels as RGBA bytes.
  pub fn rgba(&self) -> Vec<u8> {
    self.0.read().unwrap().rgba().to_vec()
  }

  /// The pixels as RGBA bytes.
  pub fn to_rgba_vec(&self) -> Vec<u8> {
    self.0.read().unwrap().to_rgba_vec()
  }

  /// The pixels as RGB bytes, without alpha.
  pub fn rgb(&self) -> Vec<u8> {
    self.0.read().unwrap().rgb()
  }

  /// A buffer of zeros the size of the pixels, to work in.
  pub fn empty_pixel_vec(&self) -> Vec<u8> {
    self.0.read().unwrap().empty_pixel_vec()
  }

  /// Adds `value` to the red, green and blue of every pixel, clamped to 0 to 255.
  pub fn add(&self, value: f32) {
    self.edit(|image| image.add(value));
  }

  /// Subtracts `value` from the red, green and blue of every pixel, clamped to 0 to 255.
  pub fn sub(&self, value: f32) {
    self.edit(|image| image.sub(value));
  }

  /// Multiplies the red, green and blue of every pixel by `value`, clamped to 0 to 255.
  pub fn mul(&self, value: f32) {
    self.edit(|image| image.mul(value));
  }

  /// Divides the red, green and blue of every pixel by `value`, clamped to 0 to 255.
  pub fn div(&self, value: f32) {
    self.edit(|image| image.div(value));
  }
}

/// How many bytes one pixel takes in this layout.
#[uniffi::export]
pub fn channels_bytes_per_pixel(channels: Channels) -> u32 {
  channels.bytes_per_pixel() as u32
}

/// Where a channel sits within an RGBA pixel: 0 to 3.
#[uniffi::export]
pub fn channel_index(channel: Channel) -> u32 {
  channel.index() as u32
}

/// A resolution of `x_dpi` by `y_dpi`.
#[uniffi::export]
pub fn resolution_new(x_dpi: f32, y_dpi: f32) -> Resolution {
  Resolution::new(x_dpi, y_dpi)
}

/// A resolution of `dpi` in both directions.
#[uniffi::export]
pub fn resolution_uniform(dpi: f32) -> Resolution {
  Resolution::uniform(dpi)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn color(r: u8, g: u8, b: u8, a: u8) -> Arc<Color> {
    Arc::new(Color::from_rgba(r, g, b, a))
  }

  #[test]
  fn a_clone_is_changed_without_changing_the_original() {
    let image = Image::new_from_color(2, 1, color(9, 8, 7, 255));
    let copy = image.duplicate();
    assert!(copy.shares_pixels_with(Arc::new(Image::from_core(image.core()))));
    copy.clear_color(color(1, 2, 3, 255));
    assert_eq!(image.get_pixel(0, 0), Some(vec![9, 8, 7, 255]));
    assert_eq!(copy.get_pixel(0, 0), Some(vec![1, 2, 3, 255]));
  }

  #[test]
  fn an_image_of_one_color_has_its_size_and_color() {
    let image = Image::new_from_color(3, 2, color(9, 8, 7, 255));
    assert_eq!((image.width(), image.height()), (3, 2));
    assert_eq!(
      image.size(),
      Size {
        width: 3.0,
        height: 2.0
      }
    );
    assert_eq!(image.get_pixel(2, 1), Some(vec![9, 8, 7, 255]));
    assert_eq!(image.get_pixel(3, 0), None);
  }

  #[test]
  fn changing_an_image_changes_it_in_place_and_leaves_its_copies() {
    let image = Image::new_from_color(2, 2, color(0, 0, 0, 255));
    let copy = Image::from_core(image.core());
    image.set_pixel(1, 1, vec![5, 6, 7, 8]);
    assert_eq!(image.get_pixel(1, 1), Some(vec![5, 6, 7, 8]));
    assert_eq!(copy.get_pixel(1, 1), Some(vec![0, 0, 0, 255]));
    image.clear_color(color(1, 2, 3, 4));
    assert_eq!(image.get_pixel(0, 0), Some(vec![1, 2, 3, 4]));
  }

  #[test]
  fn pixels_go_in_and_out_as_bytes() {
    let image = Image::new_from_pixels(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8], Channels::RGBA);
    assert_eq!(image.rgba(), vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(image.rgb(), vec![1, 2, 3, 5, 6, 7]);
    assert_eq!(image.empty_pixel_vec(), vec![0; 8]);
    image.set_rgba(vec![9; 8]);
    assert_eq!(image.to_rgba_vec(), vec![9; 8]);
    image.set_pixels(1, 1, vec![1, 2, 3], Channels::RGB);
    assert_eq!((image.width(), image.rgba()), (1, vec![1, 2, 3, 255]));
    assert_eq!(channels_bytes_per_pixel(Channels::RGB), 3);
    assert_eq!(channel_index(Channel::A), 3);
  }

  #[test]
  fn an_area_is_read_row_by_row() {
    let image =
      Image::new_from_pixels(2, 2, vec![1, 0, 0, 255, 2, 0, 0, 255, 3, 0, 0, 255, 4, 0, 0, 255], Channels::RGBA);
    let reds: Vec<u8> = image.get_pixels(vec![0, 1, 2, 1]).iter().map(|pixel| pixel[0]).collect();
    assert_eq!(reds, vec![3, 4]);
  }

  #[test]
  fn drawing_and_sharing_pixels_between_images() {
    let canvas = Image::new_from_color(3, 3, color(0, 0, 0, 255));
    let patch = Arc::new(Image::new_from_color(1, 1, color(200, 0, 0, 255)));
    canvas.draw_image_at(patch.clone(), vec![1, 2]);
    assert_eq!(canvas.get_pixel(1, 2).unwrap()[0], 200);
    assert_eq!(canvas.get_pixel(0, 0).unwrap()[0], 0);
    let twin = Image::new(1, 1);
    twin.copy_channel_data(patch.clone());
    assert!(twin.shares_pixels_with(patch));
    // Drawing an image onto itself does not lock up.
    let itself = Arc::new(Image::new_from_color(2, 2, color(5, 5, 5, 255)));
    itself.draw_image_at(itself.clone(), vec![0, 0]);
  }

  #[test]
  #[should_panic(expected = "a pixel takes 4 values")]
  fn a_pixel_of_the_wrong_length_panics() {
    Image::new(1, 1).set_pixel(0, 0, vec![1, 2, 3]);
  }

  #[test]
  fn scalar_operators_change_the_color_channels_only() {
    let image = Image::new_from_color(1, 1, color(100, 100, 100, 77));
    image.add(10.0);
    assert_eq!(image.get_pixel(0, 0), Some(vec![110, 110, 110, 77]));
    image.sub(10.0);
    image.mul(2.0);
    assert_eq!(image.get_pixel(0, 0).unwrap()[0], 200);
    image.div(4.0);
    assert_eq!(image.get_pixel(0, 0).unwrap()[0], 50);
  }

  #[test]
  fn resolution_is_metadata() {
    let image = Image::new(2, 2);
    image.set_resolution(resolution_uniform(300.0));
    assert_eq!(image.resolution(), Resolution::new(300.0, 300.0));
    assert_eq!(resolution_new(96.0, 72.0), Resolution::new(96.0, 72.0));
  }

  #[test]
  fn an_image_is_written_and_read_back() {
    let path = std::env::temp_dir().join("abra_uniffi_round_trip.png");
    let image = Image::new_from_color(2, 2, color(10, 20, 30, 255));
    image.write(path.to_string_lossy().into_owned(), None);
    let back = Image::read(path.to_string_lossy().into_owned());
    assert_eq!(back.get_pixel(1, 1), Some(vec![10, 20, 30, 255]));
    std::fs::remove_file(path).ok();
  }

  #[test]
  #[should_panic]
  fn reading_a_file_that_is_not_there_panics_with_the_librarys_message() {
    Image::read("no/such/file.png".to_string());
  }
}

#[cfg(test)]
mod bridge {
  use super::*;
  use uniffi::HandleAlloc;

  #[test]
  fn the_clone_of_the_bridge_is_a_new_image() {
    // The bridge is what other languages call, and it is not the same as calling the function in Rust.
    let image = Arc::new(Image::new_from_color(2, 1, Arc::new(Color::from_rgba(9, 8, 7, 255))));
    let handle = <Image as HandleAlloc<crate::UniFfiTag>>::new_handle(image.clone());
    let mut status = uniffi::RustCallStatus::default();
    let result = uniffi_abra_uniffi_fn_method_image_clone(handle, &mut status);
    let copy = unsafe { <Image as HandleAlloc<crate::UniFfiTag>>::consume_handle(result) };
    assert!(!Arc::ptr_eq(&copy, &image), "the clone of the bridge handed back the same image");
  }
}
