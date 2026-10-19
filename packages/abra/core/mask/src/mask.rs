pub use abra_core::rgba_to_gray;
use abra_core::{
  Area, Color, GrayPlane, Image, PointF, Rect, ResizeTarget, Size, Transform, TransformAlgorithm, blend,
};
use std::borrow::Cow;

use drawing::fill;
use rayon::prelude::*;

/// Helper trait to convert various types into an optional PointF
pub trait IntoOptionalPointF {
  /// Converts the value into an optional PointF.
  fn into_optional_point_f(self) -> Option<PointF>;
}

impl IntoOptionalPointF for Option<PointF> {
  fn into_optional_point_f(self) -> Option<PointF> {
    self
  }
}

impl IntoOptionalPointF for PointF {
  fn into_optional_point_f(self) -> Option<PointF> {
    Some(self)
  }
}

impl IntoOptionalPointF for (f32, f32) {
  fn into_optional_point_f(self) -> Option<PointF> {
    Some(PointF::from(self))
  }
}

impl IntoOptionalPointF for (i32, i32) {
  fn into_optional_point_f(self) -> Option<PointF> {
    Some(PointF::from(self))
  }
}

impl IntoOptionalPointF for (u32, u32) {
  fn into_optional_point_f(self) -> Option<PointF> {
    Some(PointF::from(self))
  }
}

impl IntoOptionalPointF for (f64, f64) {
  fn into_optional_point_f(self) -> Option<PointF> {
    Some(PointF::from(self))
  }
}

/// A mask says how much of an operation reaches each pixel of an image.
///
/// It holds one byte per pixel: `255` means fully applied, `0` means untouched, and values in between apply
/// partially. A mask is a quarter of the size of the same picture as an RGBA image. Clones share their bytes until one
/// of them changes.
#[derive(Clone, Debug)]
pub struct Mask {
  plane: GrayPlane,
}

/// Converts an owned or borrowed mask into a copy-on-write mask reference.
pub trait IntoMaskCow<'a> {
  /// Converts this value without cloning borrowed masks.
  fn into_mask_cow(self) -> Cow<'a, Mask>;
}

impl<'a> IntoMaskCow<'a> for &'a Mask {
  fn into_mask_cow(self) -> Cow<'a, Mask> {
    Cow::Borrowed(self)
  }
}

impl<'a> IntoMaskCow<'a> for Mask {
  fn into_mask_cow(self) -> Cow<'a, Mask> {
    Cow::Owned(self)
  }
}

impl Mask {
  /// Creates a mask the size of an existing image that lets everything through (all `255`).
  /// - `p_src_image`: The image whose size the mask takes.
  pub fn new_from_image(p_src_image: &Image) -> Mask {
    let (width, height) = p_src_image.dimensions::<u32>();
    Mask::from_values(width, height, vec![255; width as usize * height as usize])
  }

  /// Creates a mask that lets nothing through (all `0`), to draw into or to add to.
  /// - `p_width`, `p_height`: The size of the mask.
  pub fn empty(p_width: u32, p_height: u32) -> Mask {
    Mask::from_values(p_width, p_height, vec![0; p_width as usize * p_height as usize])
  }

  /// Creates a mask from one value per pixel.
  /// - `p_width`, `p_height`: The size of the mask.
  /// - `p_values`: The values, row by row from the top left. Its length must be `p_width * p_height`.
  pub fn from_values(p_width: u32, p_height: u32, p_values: Vec<u8>) -> Mask {
    Mask {
      plane: GrayPlane::new(p_width, p_height, p_values),
    }
  }

  /// Creates a mask from the brightness of each pixel of an image: white is `255` and black is `0`. The image's
  /// alpha is ignored.
  pub fn from_image(p_img: Image) -> Mask {
    Mask {
      plane: GrayPlane::from_image(&p_img),
    }
  }

  /// The mask's values and size, which is what effects read.
  pub fn plane(&self) -> &GrayPlane {
    &self.plane
  }

  /// The width of the mask in pixels.
  pub fn width(&self) -> u32 {
    self.plane.width()
  }

  /// The height of the mask in pixels.
  pub fn height(&self) -> u32 {
    self.plane.height()
  }

  /// The size of the mask as a tuple of `T` (generic integer type).
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u64>,
    <T as TryFrom<u64>>::Error: std::fmt::Debug,
  {
    self.plane.dimensions()
  }

  /// The mask values, one per pixel, row by row from the top left.
  pub fn values(&self) -> &[u8] {
    self.plane.values()
  }

  /// The mask value at a pixel, or `None` outside the mask.
  pub fn get(&self, p_x: u32, p_y: u32) -> Option<u8> {
    self.plane.get(p_x, p_y)
  }

  /// Creates a mask of the pixels of an image that are close to a color: `255` where a pixel has the color's hue and is
  /// saturated and bright enough, `0` where it does not, and values between along the edge when `p_feather` is more than
  /// `0`. Use it to pick out a color anywhere in a picture, then limit an effect to it, or combine it with other masks
  /// (such as an eye mask) to pick out that color in only one place.
  ///
  /// Gray pixels have no hue, so give `p_min_saturation` more than `0` to leave them out.
  /// - `p_image`: The image to look at.
  /// - `p_color`: The color whose hue is looked for. Only its hue is used.
  /// - `p_hue_range`: How far a pixel's hue may be from the color's, in degrees either way, and still count fully.
  /// - `p_min_saturation`: The least saturation, from `0` to `1`, a pixel may have.
  /// - `p_min_value`: The least brightness (HSV value), from `0` to `1`, a pixel may have.
  /// - `p_feather`: How soft the edges are, from `0` (hard) to `1`. Each limit eases in over that fraction of itself: the
  ///   hue range grows by it, and the saturation and brightness limits start lower by it.
  pub fn from_color_range(
    p_image: &Image, p_color: Color, p_hue_range: f32, p_min_saturation: f32, p_min_value: f32, p_feather: f32,
  ) -> Mask {
    let (width, height) = p_image.dimensions::<u32>();
    let target_hue = p_color.hsv().0;
    let feather = p_feather.clamp(0.0, 1.0);
    // 1 inside the limit, easing to 0 over `feather` of the limit past it, and 0 beyond that. Each limit is held to 0..=1 on its
    // own: two limits that are both far out would otherwise multiply two negative numbers into a positive one.
    let at_most = |value: f32, limit: f32| {
      if value <= limit {
        1.0
      } else if limit * feather > 0.0 {
        (1.0 - (value - limit) / (limit * feather)).max(0.0)
      } else {
        0.0
      }
    };
    // 1 at or above the limit, easing to 0 over `feather` of the limit below it, and 0 beyond that.
    let at_least = |value: f32, limit: f32| {
      if value >= limit {
        1.0
      } else if limit * feather > 0.0 {
        (1.0 - (limit - value) / (limit * feather)).max(0.0)
      } else {
        0.0
      }
    };
    let values: Vec<u8> = p_image
      .rgba()
      .par_chunks_exact(4)
      .map(|pixel| {
        let (hue, saturation, value) = Color::from_rgb(pixel[0], pixel[1], pixel[2]).hsv();
        let distance = (hue - target_hue).abs();
        let distance = distance.min(360.0 - distance);
        let weight =
          at_most(distance, p_hue_range) * at_least(saturation, p_min_saturation) * at_least(value, p_min_value);
        (weight.clamp(0.0, 1.0) * 255.0).round() as u8
      })
      .collect();
    Mask::from_values(width, height, values)
  }

  /// Creates a mask of how much of each pixel of an image is a foreground color rather than a background color, taking each pixel's
  /// color as a mix of the two: a pixel exactly the foreground color is `255`, one exactly the background color is `0`, and one halfway
  /// between them is half. It is the share of the foreground in the pixel, from where its color falls on the line from the background
  /// color to the foreground color. Use it to pull out thin parts (hair, fur, smoke) against a background of another color, where the
  /// pixels at the edge are a mix of both, or to take a color out of a picture.
  ///
  /// The two colors can be different for every pixel, so each is a picture: the color the foreground has near that pixel and the color
  /// the background has there (made, for example, by blurring the sure foreground and the sure background outwards, which fills in
  /// both under the edge). Where the two colors are too alike for the pixel's color to say which it is, the fallback mask decides.
  ///
  /// Panics when the pictures are not all the same size.
  /// - `p_image`: The image to look at. Only the color is used.
  /// - `p_foreground`: The foreground color at each pixel.
  /// - `p_background`: The background color at each pixel.
  /// - `p_fallback`: The mask used where the colors cannot tell. It is scaled to the image if it is another size.
  /// - `p_least_difference`: How far apart the two colors have to be, as a distance with each channel from `0` to `1`, for the
  ///   pixel's color to decide fully. Closer than that the fallback counts for more, down to nothing decided at no distance. `0` always
  ///   decides.
  pub fn from_color_mix(
    p_image: &Image, p_foreground: &Image, p_background: &Image, p_fallback: &Mask, p_least_difference: f32,
  ) -> Mask {
    let (width, height) = p_image.dimensions::<u32>();
    assert!(
      p_foreground.dimensions::<u32>() == (width, height) && p_background.dimensions::<u32>() == (width, height),
      "the image, the foreground colors and the background colors must be the same size"
    );
    let resized;
    let fallback = if p_fallback.dimensions::<u32>() == (width, height) {
      p_fallback.values()
    } else {
      resized = p_fallback.resized(width, height, TransformAlgorithm::Bilinear);
      resized.values()
    };
    let values = p_image
      .rgba()
      .par_chunks_exact(4)
      .zip(p_foreground.rgba().par_chunks_exact(4))
      .zip(p_background.rgba().par_chunks_exact(4))
      .zip(fallback.par_iter())
      .map(|(((pixel, foreground), background), fallback)| {
        let channel = |p_color: &[u8], p_at: usize| p_color[p_at] as f32 / 255.0;
        let (mut along, mut apart) = (0.0f32, 0.0f32);
        for at in 0..3 {
          let difference = channel(foreground, at) - channel(background, at);
          along += (channel(pixel, at) - channel(background, at)) * difference;
          apart += difference * difference;
        }
        let share = if apart > 0.0 { (along / apart).clamp(0.0, 1.0) } else { 0.0 };
        let decided = if p_least_difference > 0.0 { (apart.sqrt() / p_least_difference).clamp(0.0, 1.0) } else { 1.0 };
        let decided = if apart > 0.0 { decided } else { 0.0 };
        let mixed = decided * share + (1.0 - decided) * (*fallback as f32 / 255.0);
        (mixed * 255.0).round().clamp(0.0, 255.0) as u8
      })
      .collect();
    Mask::from_values(width, height, values)
  }

  /// The mask as a gray RGBA image, for drawing or saving.
  pub fn to_image(&self) -> Image {
    self.plane.to_image()
  }

  /// A copy of the mask scaled to a new size.
  /// - `p_width`, `p_height`: The new size.
  /// - `p_algorithm`: How values between pixels are worked out.
  pub fn resized(&self, p_width: u32, p_height: u32, p_algorithm: TransformAlgorithm) -> Mask {
    let mut image = self.to_image();
    image.resize(ResizeTarget::Exact(Size::new(p_width, p_height)), p_algorithm);
    Mask::from_image(image)
  }

  /// The smallest rectangle holding every pixel whose value is above `p_threshold`, or `None` when there is none. Use
  /// it to find where a mask has anything in it.
  /// - `p_threshold`: Pixels above this value count. `0` counts every pixel the mask reaches at all.
  pub fn bounds(&self, p_threshold: u8) -> Option<Rect> {
    let (width, height) = (self.width() as usize, self.height() as usize);
    let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
    for (row, values) in self.values().chunks_exact(width.max(1)).enumerate() {
      let Some(first) = values.iter().position(|value| *value > p_threshold) else {
        continue;
      };
      let last = values.iter().rposition(|value| *value > p_threshold).unwrap_or(first);
      left = left.min(first);
      right = right.max(last + 1);
      top = top.min(row);
      bottom = row + 1;
    }
    (right > left && bottom > top).then(|| Rect::from_edges(left, top, right, bottom))
  }

  /// How much of the mask is selected: the fraction, from `0` to `1`, of its pixels with a value above `p_threshold`. A mask with
  /// no pixels is `0`. Use it with [`Mask::cropped`] to see how full a region is, which tells a round, solid spot from a thin or
  /// patchy one of the same size.
  /// - `p_threshold`: Pixels above this value count.
  pub fn coverage(&self, p_threshold: u8) -> f32 {
    let values = self.values();
    if values.is_empty() {
      return 0.0;
    }
    values.iter().filter(|value| **value > p_threshold).count() as f32 / values.len() as f32
  }

  /// A copy of the part of the mask inside a rectangle. The rectangle is clipped to the mask; when none of it is on the
  /// mask the result has no pixels.
  /// - `p_rect`: The part to keep.
  pub fn cropped(&self, p_rect: Rect) -> Mask {
    let keep = p_rect.intersect(Rect::new((0, 0), (self.width(), self.height())));
    if keep.is_empty() {
      return Mask::from_values(0, 0, Vec::new());
    }
    let (left, top, right, bottom) = keep.edges::<u32>();
    let (width, height) = (right - left, bottom - top);
    let stride = self.width() as usize;
    let mut values = Vec::with_capacity(width as usize * height as usize);
    for row in top..bottom {
      let start = row as usize * stride + left as usize;
      values.extend_from_slice(&self.values()[start..start + width as usize]);
    }
    Mask::from_values(width, height, values)
  }

  /// This mask placed onto a larger, empty (all `0`) mask. The part that falls outside the new mask is cut off. It is the
  /// opposite of [`Mask::cropped`]: a mask found in a crop of a picture can be put back at the crop's place in the whole
  /// picture.
  /// - `p_width`, `p_height`: The size of the new mask.
  /// - `p_at`: Where the top left corner of this mask goes in the new mask. May be negative.
  pub fn placed(&self, p_width: u32, p_height: u32, p_at: impl Into<PointF>) -> Mask {
    let at = p_at.into();
    let (offset_x, offset_y) = (at.x.round() as i64, at.y.round() as i64);
    let mut values = vec![0u8; p_width as usize * p_height as usize];
    let columns = (0..self.width() as i64).filter(|x| (0..p_width as i64).contains(&(x + offset_x)));
    let (first, last) = (columns.clone().next(), columns.last());
    if let (Some(first), Some(last)) = (first, last) {
      for row in (0..self.height() as i64).filter(|y| (0..p_height as i64).contains(&(y + offset_y))) {
        let from = row as usize * self.width() as usize;
        let to = (row + offset_y) as usize * p_width as usize + (first + offset_x) as usize;
        let count = (last - first + 1) as usize;
        values[to..to + count].copy_from_slice(&self.values()[from + first as usize..from + first as usize + count]);
      }
    }
    Mask::from_values(p_width, p_height, values)
  }

  /// A mask with every value flipped: `255` becomes `0` and `0` becomes `255`.
  pub fn inverted(&self) -> Mask {
    Mask::from_values(self.width(), self.height(), self.values().iter().map(|value| 255 - value).collect())
  }

  /// The two masks added: where both reach a pixel the result is stronger, up to `255`. The result is the size of this
  /// mask; a mask of another size is first scaled to it (see [`Mask::combined`]).
  /// - `p_other`: The mask to add.
  pub fn added(&self, p_other: &Mask) -> Mask {
    self.combined(p_other, |a, b| a.saturating_add(b))
  }

  /// This mask with the other taken away: where the other reaches a pixel, this one is weaker, down to `0`. Use it to
  /// cut one region out of another.
  /// - `p_other`: The mask to take away.
  pub fn subtracted(&self, p_other: &Mask) -> Mask {
    self.combined(p_other, |a, b| a.saturating_sub(b))
  }

  /// The two masks multiplied: a pixel is only as strong as the weaker one, softly (`128` × `128` is `64`). Use it to
  /// keep what two masks agree on, such as the skin of one person.
  /// - `p_other`: The mask to multiply by.
  pub fn multiplied(&self, p_other: &Mask) -> Mask {
    self.combined(p_other, |a, b| ((a as u16 * b as u16 + 127) / 255) as u8)
  }

  /// The strongest of the two masks at each pixel: everything either one reaches.
  /// - `p_other`: The mask to combine with.
  pub fn maximum(&self, p_other: &Mask) -> Mask {
    self.combined(p_other, u8::max)
  }

  /// The weakest of the two masks at each pixel: only what both reach.
  /// - `p_other`: The mask to combine with.
  pub fn minimum(&self, p_other: &Mask) -> Mask {
    self.combined(p_other, u8::min)
  }

  /// This mask grown: each pixel takes the strongest value within `p_radius` pixels of it, so the edge of a region moves out
  /// by that many pixels. Use it to cover a little beyond what a model found, or to keep an effect from stopping short of an
  /// edge. The window is a square, and the edges of the mask are not eaten: pixels past them are ignored.
  /// - `p_radius`: How many pixels the edge moves out. `0` gives the same mask.
  pub fn dilated(&self, p_radius: u32) -> Mask {
    self.windowed(p_radius, true)
  }

  /// This mask shrunk: each pixel takes the weakest value within `p_radius` pixels of it, so the edge of a region moves in
  /// by that many pixels. Use it to pull a mask in off a background it leaks onto. The window is a square, and the edges of
  /// the mask are not eaten: pixels past them are ignored.
  /// - `p_radius`: How many pixels the edge moves in. `0` gives the same mask.
  pub fn eroded(&self, p_radius: u32) -> Mask {
    self.windowed(p_radius, false)
  }

  /// The strongest (`p_strongest`) or weakest value in the square window of `p_radius` pixels around each pixel, done as one
  /// pass along the rows and one along the columns.
  fn windowed(&self, p_radius: u32, p_strongest: bool) -> Mask {
    let (width, height) = (self.width() as usize, self.height() as usize);
    if p_radius == 0 || width == 0 || height == 0 {
      return self.clone();
    }
    let radius = p_radius as usize;
    let along_rows = |p_values: &[u8], p_width: usize| {
      let mut out = vec![0u8; p_values.len()];
      out.par_chunks_mut(p_width).zip(p_values.par_chunks(p_width)).for_each(|(row_out, row)| {
        window_extreme(row, radius, p_strongest, row_out);
      });
      out
    };
    let across = along_rows(self.values(), width);
    let flipped = transposed(&across, width, height);
    let down = along_rows(&flipped, height);
    Mask::from_values(self.width(), self.height(), transposed(&down, height, width))
  }

  /// Joins the values of two masks pixel by pixel. The result is the size of this mask. A mask of another size is
  /// scaled to it with a bilinear resize first, and a mask with no pixels counts as `0` everywhere.
  fn combined(&self, p_other: &Mask, p_join: impl Fn(u8, u8) -> u8) -> Mask {
    let resized;
    let other = if p_other.dimensions::<u32>() == self.dimensions::<u32>() {
      p_other.values()
    } else if p_other.width() == 0 || p_other.height() == 0 {
      resized = Mask::from_values(self.width(), self.height(), vec![0; self.values().len()]);
      resized.values()
    } else {
      resized = p_other.resized(self.width(), self.height(), TransformAlgorithm::Bilinear);
      resized.values()
    };
    Mask::from_values(
      self.width(),
      self.height(),
      self.values().iter().zip(other).map(|(a, b)| p_join(*a, *b)).collect(),
    )
  }
}

impl From<Mask> for GrayPlane {
  fn from(p_mask: Mask) -> GrayPlane {
    p_mask.plane
  }
}

impl From<&Mask> for GrayPlane {
  fn from(p_mask: &Mask) -> GrayPlane {
    p_mask.plane.clone()
  }
}

impl From<Mask> for Image {
  fn from(p_mask: Mask) -> Image {
    p_mask.to_image()
  }
}

/// How a [`Blob`] fades from its middle out to its edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Falloff {
  /// A bell: `exp(-fade * d²)`, where `d` is the distance from the middle in half sizes (`1` is the edge of the oval), out to
  /// `reach` (`d²` up to this, as `1.7`) and nothing beyond. A bigger `fade` is a tighter bell with a harder edge.
  Gaussian { fade: f32, reach: f32 },
  /// Full at the middle and nothing at the edge (`d` = `1`), along a smooth curve with no line anywhere. `fade` is how much of the
  /// way in from the edge the curve takes: `1` fades from the middle, and a smaller `fade` is full for longer and harder at the edge.
  Smooth { fade: f32 },
}

/// A soft oval to put in a mask: a place, a size, a turn and how it fades. See [`Mask::from_blobs`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blob {
  /// The middle of the oval, in pixels of the mask.
  pub center: PointF,
  /// Half the length (along its turn) and half the thickness (across it) of the oval, in pixels.
  pub half_size: Size,
  /// How far the oval is turned from lying flat, in radians.
  pub angle: f32,
  /// How it fades from its middle to its edge.
  pub falloff: Falloff,
}

impl Blob {
  /// How many half sizes out from the middle (`d`) the blob reaches anything.
  fn reach(&self) -> f32 {
    match self.falloff {
      Falloff::Gaussian { reach, .. } => reach.max(0.0).sqrt(),
      Falloff::Smooth { .. } => 1.0,
    }
  }

  /// How strong the blob is at a pixel, `0` to `255`.
  fn strength(&self, p_x: f32, p_y: f32, p_cos: f32, p_sin: f32) -> u8 {
    let (dx, dy) = (p_x - self.center.x, p_y - self.center.y);
    let along = (dx * p_cos + dy * p_sin) / self.half_size.width;
    let across = (-dx * p_sin + dy * p_cos) / self.half_size.height;
    let squared = along * along + across * across;
    match self.falloff {
      Falloff::Gaussian { fade, reach } if squared < reach => (255.0 * (-fade * squared).exp()).round() as u8,
      Falloff::Gaussian { .. } => 0,
      Falloff::Smooth { fade } => {
        let edge = ((1.0 - squared.sqrt()) / fade).clamp(0.0, 1.0);
        (255.0 * edge * edge * (3.0 - 2.0 * edge)).round() as u8
      }
    }
  }
}

impl Mask {
  /// A mask of soft ovals, for shading a face or any region with a place and a shape. Where ovals overlap the strongest one
  /// wins. Only the pixels an oval can reach are worked on, a row at a time in parallel, so a few small ovals on a large mask
  /// cost little.
  /// - `p_width`, `p_height`: The size of the mask.
  /// - `p_blobs`: The ovals. One with no size is left out.
  pub fn from_blobs(p_width: u32, p_height: u32, p_blobs: &[Blob]) -> Mask {
    let (width, height) = (p_width as usize, p_height as usize);
    let mut values = vec![0u8; width * height];
    for blob in p_blobs.iter().filter(|blob| blob.half_size.width > 0.0 && blob.half_size.height > 0.0) {
      let (sin, cos) = blob.angle.sin_cos();
      let radius = blob.half_size.width.max(blob.half_size.height) * blob.reach();
      let left = (blob.center.x - radius).floor().max(0.0) as usize;
      let right = ((blob.center.x + radius).ceil().max(0.0) as usize + 1).min(width);
      let top = (blob.center.y - radius).floor().max(0.0) as usize;
      let bottom = ((blob.center.y + radius).ceil().max(0.0) as usize + 1).min(height);
      if left >= right || top >= bottom {
        continue;
      }
      values.par_chunks_mut(width).enumerate().skip(top).take(bottom - top).for_each(|(y, row)| {
        for x in left..right {
          row[x] = row[x].max(blob.strength(x as f32, y as f32, cos, sin));
        }
      });
    }
    Mask::from_values(p_width, p_height, values)
  }
}

impl From<Image> for Mask {
  fn from(p_img: Image) -> Mask {
    Mask::from_image(p_img)
  }
}

impl Mask {
  /// Draws a filled area onto the mask with the specified color.
  /// - `p_area`: The Area to draw.
  /// - `p_color`: The Color to use for the area.
  /// - `p_at`: Optional position of the filled image's top-left corner. Defaults to the top-left of the area's bounds, so the
  ///   area lands where it is drawn.
  pub fn draw_area(&mut self, p_area: &Area, p_color: Color, p_at: impl IntoOptionalPointF) {
    let color = self.to_color(p_color);
    let filled_image = fill(p_area, &color).to_image();
    let position = p_at.into_optional_point_f().unwrap_or_else(|| {
      let (min_x, min_y, _, _) = p_area.bounds().edges::<f32>();
      PointF::new(min_x, min_y)
    });
    let mut image = self.to_image();
    blend::blend(&filled_image).with_offset((position.x as i32, position.y as i32)).apply(&mut image);
    // A new buffer, so clones made earlier keep the old values.
    self.plane = Mask::from_image(image).plane;
  }

  /// Apply the mask to an image by adjusting the image's alpha channel.
  ///
  /// The mask value becomes the alpha:
  /// - 255 -> alpha 255 (fully opaque / visible)
  /// - 0 -> alpha 0 (fully transparent / hidden)
  ///
  /// The mask size must match the image size. If you need positioning, use `Image::set_from` or a temporary canvas.
  pub fn apply_to_image(&self, p_image: &mut Image) {
    if let Some(pixels) = p_image.colors().as_slice_mut() {
      apply_mask_to_pixels_rgba(pixels, self.values());
    }
  }

  /// Multiplies the image's alpha by the mask, so what is already transparent stays so and white keeps the image as it is
  /// (`apply_to_image` replaces the alpha instead). The mask size must match the image size.
  pub fn multiply_alpha(&self, p_image: &mut Image) {
    assert_eq!(self.dimensions::<u32>(), p_image.dimensions::<u32>(), "mask must be the same size as the image");
    if let Some(pixels) = p_image.colors().as_slice_mut() {
      for (rgba, &m) in pixels.chunks_mut(4).zip(self.values()) {
        rgba[3] = ((rgba[3] as u16 * m as u16 + 127) / 255) as u8;
      }
    }
  }

  fn to_color(&self, p_color: Color) -> Color {
    let c = ((p_color.r as u16 + p_color.g as u16 + p_color.b as u16) / 3) as u8;
    Color::from_rgba(c, c, c, p_color.a)
  }
}

/// Converts a grayscale mask value to an alpha value where:
/// - 255 (white) => 255 alpha (fully opaque/visible)
/// - 0 (black) => 0 alpha (fully transparent/hidden)
/// - 127 (gray) => ~127 alpha (~50% opacity)
pub fn mask_value_to_alpha(p_value: u8) -> u8 {
  p_value
}

/// Applies a mask to an image by setting the image's alpha channel from the provided mask data.
///
/// Semantics:
/// - White (255) in the mask becomes fully opaque (alpha 255)
/// - Black (0) in the mask becomes fully transparent (alpha 0)
/// - Gray values map linearly between 0 and 255
///
/// Mask input may be one of:
/// - Grayscale: length = width * height
/// - RGBA: length = width * height * 4 (converted to grayscale)
pub fn apply_mask_to_image(p_image: &mut Image, p_mask: &[u8]) {
  let (width, height) = p_image.dimensions::<usize>();
  if let Some(pixels) = p_image.colors().as_slice_mut() {
    apply_mask_to_rgba_pixels(pixels, p_mask, width * height);
  }
}

/// Applies a mask directly to an RGBA pixel slice by setting its alpha channel.
///
/// See `apply_mask_to_image` for mask semantics and accepted formats.
pub fn apply_mask_to_pixels_rgba(p_pixels: &mut [u8], p_mask: &[u8]) {
  assert!(p_pixels.len() % 4 == 0, "pixels must be RGBA (len divisible by 4)");
  apply_mask_to_rgba_pixels(p_pixels, p_mask, p_pixels.len() / 4);
}

fn apply_mask_to_rgba_pixels(p_pixels: &mut [u8], p_mask: &[u8], p_px_count: usize) {
  // A grayscale mask is used as it is; only an RGBA mask needs converting.
  let mask_gray: Cow<[u8]> = match p_mask.len() {
    len if len == p_px_count => Cow::Borrowed(p_mask),
    len if len == p_px_count * 4 => Cow::Owned(p_mask.chunks(4).map(rgba_to_gray).collect()),
    other => panic!("Invalid mask size: expected {} (gray) or {} (rgba) but got {}", p_px_count, p_px_count * 4, other),
  };

  for (rgba, &m) in p_pixels.chunks_mut(4).zip(mask_gray.iter()) {
    rgba[3] = mask_value_to_alpha(m);
  }
}

/// For each position of `p_line`, the strongest (`p_strongest`) or weakest value within `p_radius` positions either way
/// (fewer at the ends), written to `p_out`. One pass, with a queue of the positions that could still be the answer.
fn window_extreme(p_line: &[u8], p_radius: usize, p_strongest: bool, p_out: &mut [u8]) {
  let beats = |p_new: u8, p_old: u8| if p_strongest { p_new >= p_old } else { p_new <= p_old };
  let mut queue = std::collections::VecDeque::new();
  for i in 0..p_line.len() + p_radius {
    if i < p_line.len() {
      while queue.back().is_some_and(|&back| beats(p_line[i], p_line[back])) {
        queue.pop_back();
      }
      queue.push_back(i);
    }
    if i >= p_radius {
      let at = i - p_radius;
      while queue.front().is_some_and(|&front| front + p_radius < at) {
        queue.pop_front();
      }
      p_out[at] = p_line[*queue.front().expect("the window always holds the position itself")];
    }
  }
}

/// `p_values`, which is `p_width` wide and `p_height` tall, turned so rows become columns.
fn transposed(p_values: &[u8], p_width: usize, p_height: usize) -> Vec<u8> {
  let mut out = vec![0u8; p_values.len()];
  for y in 0..p_height {
    for x in 0..p_width {
      out[x * p_height + y] = p_values[y * p_width + x];
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Area, Color, Image};

  /// A row of `p_colors` as an image.
  fn row_of(p_colors: &[(u8, u8, u8)]) -> Image {
    let pixels: Vec<u8> = p_colors.iter().flat_map(|(r, g, b)| [*r, *g, *b, 255]).collect();
    Image::new_from_pixels(p_colors.len() as u32, 1, pixels, abra_core::Channels::RGBA)
  }

  #[test]
  fn a_color_mix_is_the_share_of_the_foreground_in_each_pixel() {
    let foreground = row_of(&[(200, 100, 0); 5]);
    let background = row_of(&[(0, 0, 0); 5]);
    // Pure background, a quarter, half, three quarters and pure foreground.
    let image = row_of(&[(0, 0, 0), (50, 25, 0), (100, 50, 0), (150, 75, 0), (200, 100, 0)]);
    let mask = Mask::from_color_mix(&image, &foreground, &background, &Mask::from_values(5, 1, vec![0; 5]), 0.0);
    let found = mask.values();
    assert_eq!((found[0], found[4]), (0, 255));
    assert!((found[2] as i32 - 128).abs() <= 1 && (found[1] as i32 - 64).abs() <= 1 && (found[3] as i32 - 191).abs() <= 1, "{found:?}");
  }

  #[test]
  fn a_color_beyond_either_end_is_held_to_zero_or_full() {
    let foreground = row_of(&[(100, 100, 100); 2]);
    let background = row_of(&[(50, 50, 50); 2]);
    let image = row_of(&[(10, 10, 10), (250, 250, 250)]);
    let mask = Mask::from_color_mix(&image, &foreground, &background, &Mask::empty(2, 1), 0.0);
    assert_eq!(mask.values(), &[0, 255]);
  }

  #[test]
  fn where_the_colors_are_alike_the_fallback_decides() {
    // The same color for the foreground and the background: the pixel cannot say which it is.
    let same = row_of(&[(90, 90, 90), (90, 90, 90)]);
    let mask = Mask::from_color_mix(&same, &same, &same, &Mask::from_values(2, 1, vec![40, 220]), 0.2);
    assert_eq!(mask.values(), &[40, 220]);
    // Colors only a little apart: the pixel decides partly. The difference is 0.1 over a least of 0.2, so half the decision is its own.
    let foreground = row_of(&[(128, 128, 128)]);
    let background = row_of(&[(102, 102, 102)]);
    let pixel = row_of(&[(128, 128, 128)]);
    let partly = Mask::from_color_mix(&pixel, &foreground, &background, &Mask::from_values(1, 1, vec![0]), 0.35);
    assert!(partly.values()[0] > 100 && partly.values()[0] < 255, "{:?}", partly.values());
  }

  #[test]
  #[should_panic(expected = "must be the same size")]
  fn a_color_mix_of_pictures_of_different_sizes_panics() {
    let _ = Mask::from_color_mix(&row_of(&[(1, 1, 1)]), &row_of(&[(1, 1, 1), (1, 1, 1)]), &row_of(&[(1, 1, 1)]), &Mask::empty(1, 1), 0.0);
  }

  #[test]
  fn dilating_grows_a_region_by_the_radius() {
    let mut values = vec![0u8; 25];
    values[12] = 255;
    let grown = Mask::from_values(5, 5, values).dilated(1);
    let lit: Vec<usize> = (0..25).filter(|&i| grown.values()[i] == 255).collect();
    assert_eq!(lit, vec![6, 7, 8, 11, 12, 13, 16, 17, 18]);
  }

  #[test]
  fn eroding_shrinks_a_region_by_the_radius() {
    let mut values = vec![0u8; 49];
    for y in 1..6 {
      for x in 1..6 {
        values[y * 7 + x] = 255;
      }
    }
    let shrunk = Mask::from_values(7, 7, values).eroded(1);
    assert_eq!((shrunk.get(1, 1), shrunk.get(2, 2), shrunk.get(3, 3), shrunk.get(5, 5)), (Some(0), Some(255), Some(255), Some(0)));
  }

  #[test]
  fn eroding_does_not_eat_the_edges_of_the_mask() {
    let full = Mask::from_values(4, 3, vec![255; 12]);
    assert_eq!(full.eroded(2).values(), full.values());
  }

  #[test]
  fn a_radius_of_zero_changes_nothing() {
    let mask = Mask::from_values(3, 1, vec![10, 200, 30]);
    assert_eq!(mask.dilated(0).values(), mask.values());
    assert_eq!(mask.eroded(0).values(), mask.values());
  }

  #[test]
  fn dilating_and_eroding_work_on_a_wide_radius_and_a_tall_mask() {
    let mut values = vec![0u8; 3 * 20];
    values[10 * 3 + 1] = 200;
    let grown = Mask::from_values(3, 20, values).dilated(5);
    let lit: Vec<usize> = (0..20).filter(|&y| grown.values()[y * 3 + 1] == 200).collect();
    assert_eq!(lit, (5..=15).collect::<Vec<_>>());
  }

  #[test]
  fn a_gaussian_blob_is_full_in_the_middle_and_fades_out() {
    let blob = Blob {
      center: PointF::new(20.0, 10.0),
      half_size: Size::new(8.0, 4.0),
      angle: 0.0,
      falloff: Falloff::Gaussian { fade: 3.0, reach: 1.7 },
    };
    let mask = Mask::from_blobs(40, 20, &[blob]);

    assert_eq!(mask.get(20, 10), Some(255));
    // One half width out is `d²` of 1: `255 * exp(-3)`.
    assert_eq!(mask.get(28, 10), Some((255.0 * (-3.0f32).exp()).round() as u8));
    assert!(mask.get(24, 10).unwrap() > mask.get(28, 10).unwrap());
    // Past the reach, and a long way off the mask's own edge, there is nothing.
    assert_eq!(mask.get(33, 10), Some(0));
    assert_eq!(mask.get(0, 0), Some(0));
  }

  #[test]
  fn a_smooth_blob_is_nothing_at_its_edge_and_turns_with_its_angle() {
    let flat = Blob {
      center: PointF::new(20.0, 20.0),
      half_size: Size::new(10.0, 4.0),
      angle: 0.0,
      falloff: Falloff::Smooth { fade: 1.0 },
    };
    let upright = Blob { angle: std::f32::consts::FRAC_PI_2, ..flat };
    let flat_mask = Mask::from_blobs(40, 40, &[flat]);
    let upright_mask = Mask::from_blobs(40, 40, &[upright]);

    assert_eq!(flat_mask.get(20, 20), Some(255));
    assert_eq!(flat_mask.get(30, 20), Some(0));
    assert!(flat_mask.get(25, 20).unwrap() > 0);
    // Flat it reaches 5 pixels sideways and not up; turned a quarter it is the other way round.
    assert!(flat_mask.get(25, 20).unwrap() > flat_mask.get(20, 25).unwrap());
    assert!(upright_mask.get(20, 25).unwrap() > upright_mask.get(25, 20).unwrap());
  }

  #[test]
  fn overlapping_blobs_keep_the_stronger_and_blobs_off_the_mask_are_left_out() {
    let blob = |x: f32, strength_fade: f32| Blob {
      center: PointF::new(x, 5.0),
      half_size: Size::new(4.0, 4.0),
      angle: 0.0,
      falloff: Falloff::Gaussian { fade: strength_fade, reach: 1.7 },
    };
    let both = Mask::from_blobs(20, 10, &[blob(8.0, 1.0), blob(10.0, 6.0), blob(-500.0, 1.0), blob(500.0, 1.0)]);
    let first = Mask::from_blobs(20, 10, &[blob(8.0, 1.0)]);
    let second = Mask::from_blobs(20, 10, &[blob(10.0, 6.0)]);

    for x in 0..20 {
      assert_eq!(both.get(x, 5), Some(first.get(x, 5).unwrap().max(second.get(x, 5).unwrap())));
    }
  }

  #[test]
  fn borrowed_mask_input_keeps_the_original_reference() {
    let mask = Mask::from_image(Image::new(1, 1));
    let mask_ref = &mask;

    assert!(matches!(mask_ref.into_mask_cow(), Cow::Borrowed(borrowed) if std::ptr::eq(borrowed, mask_ref)));
  }

  #[test]
  fn a_mask_holds_one_byte_per_pixel() {
    let mask = Mask::from_image(Image::new_from_color(20, 10, Color::from_rgba(255, 255, 255, 255)));

    assert_eq!(mask.dimensions::<u32>(), (20, 10));
    assert_eq!(mask.values().len(), 200);
    assert_eq!(mask.get(19, 9), Some(255));
    assert_eq!(mask.get(20, 0), None);
  }

  #[test]
  fn an_image_round_trips_through_a_mask() {
    let mut image = Image::new_from_color(2, 1, Color::from_rgba(255, 255, 255, 255));
    image.set_pixel(0, 0, (40, 40, 40, 255));
    let mask = Mask::from_image(image);

    assert_eq!(mask.values(), &[40, 255]);
    assert_eq!(mask.to_image().rgba(), &[40, 40, 40, 255, 255, 255, 255, 255]);
  }

  #[test]
  fn resizing_keeps_a_uniform_mask_uniform() {
    let mask = Mask::from_values(4, 4, vec![200; 16]).resized(8, 6, TransformAlgorithm::Bilinear);

    assert_eq!(mask.dimensions::<u32>(), (8, 6));
    assert!(mask.values().iter().all(|value| *value == 200));
  }

  fn pair() -> (Mask, Mask) {
    (Mask::from_values(4, 1, vec![0, 100, 200, 255]), Mask::from_values(4, 1, vec![255, 100, 100, 128]))
  }

  fn colors(p_pixels: &[(u8, u8, u8)]) -> Image {
    let rgba: Vec<u8> = p_pixels.iter().flat_map(|(r, g, b)| [*r, *g, *b, 255]).collect();
    Image::new_from_pixels(p_pixels.len() as u32, 1, rgba, abra_core::Channels::RGBA)
  }

  #[test]
  fn an_empty_mask_lets_nothing_through_until_something_is_drawn() {
    let mut mask = Mask::empty(6, 6);
    assert_eq!((mask.width(), mask.height()), (6, 6));
    assert!(mask.values().iter().all(|value| *value == 0));
    // The area's top left corner is placed at `at`: this circle's box starts at (1, 1).
    mask.draw_area(&Area::circle((3.0, 3.0), 2.0), Color::white(), (1.0, 1.0));
    assert_eq!(mask.get(3, 3), Some(255));
    assert_eq!(mask.get(0, 0), Some(0));
  }

  #[test]
  fn a_color_range_picks_out_the_pixels_with_that_hue() {
    // Red, orange-red, green, gray, dark red.
    let image = colors(&[
      (220, 30, 30),
      (230, 130, 30),
      (30, 200, 30),
      (120, 120, 120),
      (30, 5, 5),
    ]);
    let mask = Mask::from_color_range(&image, Color::red(), 20.0, 0.4, 0.3, 0.0);
    assert_eq!(
      mask.values(),
      &[255, 0, 0, 0, 0],
      "orange is too far, green and gray are not red, dark red is too dark"
    );
    let wider = Mask::from_color_range(&image, Color::red(), 35.0, 0.4, 0.3, 0.0);
    assert_eq!(wider.values()[1], 255, "a wider range takes in the orange-red");
  }

  #[test]
  fn a_color_range_wraps_around_the_hue_circle() {
    // Hue 350 and hue 10 are both 10 degrees from red at 0.
    let image = colors(&[(220, 30, 60), (220, 60, 30)]);
    assert_eq!(Mask::from_color_range(&image, Color::red(), 20.0, 0.4, 0.3, 0.0).values(), &[255, 255]);
  }

  #[test]
  fn a_pixel_far_outside_every_limit_is_never_selected_however_much_it_is_feathered() {
    // Blue-gray, like an iris: its hue is far from red and its saturation is below the limit, which are both far out.
    let image = colors(&[(110, 130, 150), (120, 128, 135), (10, 10, 10)]);
    for feather in [0.0, 0.3, 1.0] {
      let mask = Mask::from_color_range(&image, Color::red(), 22.0, 0.5, 0.15, feather);
      assert_eq!(mask.values(), &[0, 0, 0], "feather {feather}");
    }
  }

  #[test]
  fn feathering_eases_the_edge_instead_of_cutting_it() {
    let image = colors(&[(220, 30, 30), (220, 105, 30)]);
    let hard = Mask::from_color_range(&image, Color::red(), 20.0, 0.4, 0.3, 0.0);
    let soft = Mask::from_color_range(&image, Color::red(), 20.0, 0.4, 0.3, 1.0);
    assert_eq!(hard.values()[1], 0);
    assert!(soft.values()[1] > 0 && soft.values()[1] < 255, "partly selected: {}", soft.values()[1]);
    assert_eq!(soft.values()[0], 255, "a pixel well inside is still fully selected");
  }

  #[test]
  fn bounds_find_where_a_mask_has_anything_in_it() {
    let mut values = vec![0u8; 5 * 4];
    values[1 * 5 + 2] = 200;
    values[2 * 5 + 3] = 50;
    let mask = Mask::from_values(5, 4, values);
    assert_eq!(mask.bounds(0), Some(Rect::from_edges(2, 1, 4, 3)));
    assert_eq!(mask.bounds(100), Some(Rect::from_edges(2, 1, 3, 2)), "only values above the threshold count");
    assert_eq!(mask.bounds(200), None);
    assert_eq!(Mask::from_values(0, 0, vec![]).bounds(0), None);
  }

  #[test]
  fn coverage_is_the_fraction_of_the_mask_that_is_selected() {
    let mask = Mask::from_values(4, 1, vec![0, 100, 200, 255]);
    assert_eq!(mask.coverage(0), 0.75);
    assert_eq!(mask.coverage(100), 0.5);
    assert_eq!(mask.coverage(255), 0.0);
    assert_eq!(Mask::from_values(0, 0, vec![]).coverage(0), 0.0);
  }

  #[test]
  fn a_mask_is_cropped_and_placed_back() {
    let whole = Mask::from_values(4, 3, (0..12).collect());
    let part = whole.cropped(Rect::new((1, 1), (2, 2)));
    assert_eq!((part.width(), part.height()), (2, 2));
    assert_eq!(part.values(), &[5, 6, 9, 10]);
    assert_eq!(whole.cropped(Rect::new((3, 2), (9, 9))).values(), &[11], "clipped to the mask");
    assert_eq!(whole.cropped(Rect::new((10, 10), (2, 2))).dimensions::<u32>(), (0, 0), "nothing of it is on the mask");

    let back = part.placed(4, 3, (1, 1));
    assert_eq!(back.values(), &[0, 0, 0, 0, 0, 5, 6, 0, 0, 9, 10, 0]);
  }

  #[test]
  fn a_mask_placed_partly_outside_is_cut_off() {
    let part = Mask::from_values(2, 2, vec![1, 2, 3, 4]);
    assert_eq!(part.placed(3, 3, (2, 2)).values(), &[0, 0, 0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(part.placed(3, 3, (-1, -1)).values(), &[4, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert!(part.placed(3, 3, (5, 5)).values().iter().all(|value| *value == 0), "entirely outside");
  }

  #[test]
  fn masks_are_inverted_added_and_subtracted_without_wrapping() {
    let (a, b) = pair();
    assert_eq!(a.inverted().values(), &[255, 155, 55, 0]);
    assert_eq!(a.added(&b).values(), &[255, 200, 255, 255], "adding stops at 255");
    assert_eq!(a.subtracted(&b).values(), &[0, 0, 100, 127], "subtracting stops at 0");
  }

  #[test]
  fn masks_are_multiplied_softly_and_joined_by_strongest_and_weakest() {
    let (a, b) = pair();
    assert_eq!(a.multiplied(&b).values(), &[0, 39, 78, 128]);
    assert_eq!(Mask::from_values(1, 1, vec![128]).multiplied(&Mask::from_values(1, 1, vec![128])).values(), &[64]);
    assert_eq!(a.maximum(&b).values(), &[255, 100, 200, 255]);
    assert_eq!(a.minimum(&b).values(), &[0, 100, 100, 128]);
  }

  #[test]
  fn a_mask_of_another_size_is_scaled_to_this_one() {
    let a = Mask::from_values(4, 4, vec![200; 16]);
    let half = Mask::from_values(1, 1, vec![100]);
    let result = a.subtracted(&half);
    assert_eq!(result.dimensions::<u32>(), (4, 4), "the result keeps this mask's size");
    assert!(result.values().iter().all(|value| *value == 100));
  }

  #[test]
  fn an_empty_mask_counts_as_nothing() {
    let a = Mask::from_values(2, 1, vec![10, 20]);
    let empty = Mask::from_values(0, 0, vec![]);
    assert_eq!(a.added(&empty).values(), &[10, 20]);
    assert_eq!(a.multiplied(&empty).values(), &[0, 0]);
    assert_eq!(empty.added(&a).dimensions::<u32>(), (0, 0));
  }

  #[test]
  fn mask_clone_shares_buffer_and_cow_on_draw() {
    let img = Image::new_from_color(20, 20, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::from(img);
    let ptr1 = mask.values().as_ptr();
    let mask_clone = mask.clone();
    let ptr2 = mask_clone.values().as_ptr();
    assert_eq!(ptr1, ptr2, "Mask clones should share their values");

    // Drawing on the original must leave the clone alone.
    let area = Area::rect((1.0, 1.0), (2.0, 2.0));
    mask.draw_area(&area, Color::black(), None);
    assert_ne!(mask.values().as_ptr(), ptr2, "Drawing should have given the original mask its own values");
    assert_eq!(
      mask_clone.values().as_ptr(),
      ptr2,
      "Clone's buffer pointer should still be same after original mutated"
    );
    assert_eq!(mask_clone.get(1, 1), Some(255), "the clone is unchanged");
    assert_eq!(mask.get(1, 1), Some(0), "the original was drawn on");
  }

  #[test]
  fn test_apply_mask_to_pixels_rgba() {
    // Two pixels: RGBA (red, green)
    let mut pixels: Vec<u8> = vec![255, 0, 0, 255, 0, 255, 0, 255];
    // mask: first pixel black (transparent), second pixel white (opaque)
    let mask: Vec<u8> = vec![0, 255];
    apply_mask_to_pixels_rgba(&mut pixels, &mask);
    assert_eq!(pixels[3], 0);
    assert_eq!(pixels[7], 255);
  }

  #[test]
  fn test_apply_mask_to_image_using_mask_struct() {
    let mut img = Image::new_from_color(2, 1, Color::from_rgba(255, 0, 0, 255));
    // create a mask image with first pixel black, second white
    let mut mask_img = Image::new_from_color(2, 1, Color::from_rgba(255, 255, 255, 255));
    // set first pixel to black on mask
    mask_img.set_pixel(0, 0, (0, 0, 0, 255));
    let mask = Mask::from(mask_img);
    mask.apply_to_image(&mut img);
    let rgba = img.to_rgba_vec();
    assert_eq!(rgba[3], 0);
    assert_eq!(rgba[7], 255);
  }

  #[test]
  fn apply_to_image_does_not_copy_mask() {
    let mut img = Image::new_from_color(2, 1, Color::from_rgba(255, 0, 0, 255));
    let mut mask_img = Image::new_from_color(2, 1, Color::from_rgba(255, 255, 255, 255));
    // set first pixel to black on mask
    mask_img.set_pixel(0, 0, (0, 0, 0, 255));
    let mask = Mask::from(mask_img);
    let before_ptr = mask.values().as_ptr();
    mask.apply_to_image(&mut img);
    let after_ptr = mask.values().as_ptr();
    assert_eq!(before_ptr, after_ptr, "apply_to_image should not mutate or clone the mask's internal buffer");
  }

  #[test]
  fn draw_area_without_position_lands_where_the_area_is() {
    let img = Image::new_from_color(10, 10, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::new_from_image(&img);
    mask.draw_area(&Area::rect((6.0, 6.0), (2.0, 2.0)), Color::black(), None);
    assert_eq!(mask.get(7, 7), Some(0));
    assert_eq!(mask.get(1, 1), Some(255));
  }

  #[test]
  fn draw_area_respects_feathering() {
    let img = Image::new_from_color(10, 10, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::new_from_image(&img);
    let area = Area::rect((1.0, 1.0), (8.0, 8.0)).with_feather(2);
    mask.draw_area(&area, Color::black(), None);
    // center should be black (0) - fully drawn area
    let center_alpha = mask.get(5, 5).unwrap();
    assert_eq!(center_alpha, 0);
    // ensure we have at least one partially transparent pixel within the area (alpha not strictly 0 or 255)
    let mut found_partial = false;
    let min_x = 1usize;
    let min_y = 1usize;
    let max_x = 8usize;
    let max_y = 8usize;
    for y in min_y..=max_y {
      for x in min_x..=max_x {
        let alpha = mask.get(x as u32, y as u32).unwrap();
        if alpha > 0 && alpha < 255 {
          found_partial = true;
          break;
        }
      }
      if found_partial {
        break;
      }
    }
    assert!(found_partial, "Expected to find partial-coverage pixels for feathered area");
  }

  #[test]
  fn draw_star_area_offset_is_correct() {
    use abra_core::image::image_ext::*;
    use abra_core::{AspectRatio, Shape};
    let img = Image::new_from_color(200, 200, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::new_from_image(&img);
    // Create a star area sized to half the image and not positioned explicitly
    let area = Area::shape(Shape::Star).fit(img.size() / 2, AspectRatio::meet());
    mask.draw_area(&area, Color::black(), None);
    // compute topmost row with non-white pixel
    let mut topmost: Option<u32> = None;
    for y in 0..200u32 {
      for x in 0..200u32 {
        if mask.get(x, y).unwrap() != 255 {
          topmost = Some(y);
          break;
        }
      }
      if topmost.is_some() {
        break;
      }
    }
    assert!(topmost.is_some());
    // If shape is anchored to 0, expect topmost to be 0
    assert_eq!(topmost.unwrap(), 0);
  }

  #[test]
  fn draw_star_area_offset_with_position() {
    use abra_core::image::image_ext::*;
    use abra_core::{AspectRatio, Shape};
    let img = Image::new_from_color(200, 200, Color::from_rgba(255, 255, 255, 255));
    let mut mask = Mask::new_from_image(&img);
    let area = Area::shape(Shape::Star).fit(img.size() / 2, AspectRatio::meet());
    // Draw with an explicit offset
    mask.draw_area(&area, Color::black(), (10.0, 20.0));
    // compute topmost row with non-white pixel
    let mut topmost: Option<u32> = None;
    for y in 0..200u32 {
      for x in 0..200u32 {
        if mask.get(x, y).unwrap() != 255 {
          topmost = Some(y);
          break;
        }
      }
      if topmost.is_some() {
        break;
      }
    }
    assert!(topmost.is_some());
    // Expect topmost to be 20 due to the offset provided earlier
    assert_eq!(topmost.unwrap(), 20);
  }
}
