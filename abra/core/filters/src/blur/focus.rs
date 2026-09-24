use crate::common::*;

use crate::blur::{GaussianBlur, LensBlur, gaussian_blur};
use abra_core::Channels;
use mask::{Mask, rgba_to_gray};
use options::Apply;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusShape {
  /// Circular focus area.
  Circle,
  /// Square focus area.
  Square,
  /// Diamond-shaped focus area.
  Diamond,
  /// Horizontal band of focus through the center, like a tilt-shift lens.
  Horizontal,
  /// Vertical band of focus through the center.
  Vertical,
}

/// The blur applied outside the focus area. Pass a blur builder to [`FocusBlur::with_blur`], which converts it.
#[derive(Clone)]
pub enum BlurType {
  Gaussian(GaussianBlur),
  Lens(LensBlur),
}

impl From<GaussianBlur> for BlurType {
  fn from(p_blur: GaussianBlur) -> Self {
    BlurType::Gaussian(p_blur)
  }
}

impl From<LensBlur> for BlurType {
  fn from(p_blur: LensBlur) -> Self {
    BlurType::Lens(p_blur)
  }
}

macro_rules! clamp_with_fn {
  ($(#[$doc:meta])* $name:ident, $field:ident, $min:expr, $max:expr) => {
    $(#[$doc])*
    pub fn $name(mut self, value: f32) -> Self {
      self.$field = value.clamp($min, $max);
      self
    }
  };
}

macro_rules! positive_with_fn {
  ($(#[$doc:meta])* $name:ident, $field:ident) => {
    $(#[$doc])*
    pub fn $name(mut self, value: f32) -> Self {
      self.$field = value.max(0.0);
      self
    }
  };
}

/// Where the focus area sits and how its edge fades into the blur.
///
/// Distances are normalized so that `1.0` is half of the image's shorter side; a circle with a radius of `1.0`
/// centered in the image touches the nearest edges.
#[derive(Clone, Copy, Debug)]
pub struct FocusGeometry {
  center_x: f32,
  center_y: f32,
  radius: f32,
  sharpness: f32,
  midpoint: f32,
  aspect_ratio: f32,
  rotation: f32,
}

impl Default for FocusGeometry {
  fn default() -> Self {
    Self {
      center_x: 0.5,
      center_y: 0.5,
      radius: 0.75,
      sharpness: 0.25,
      midpoint: 0.5,
      aspect_ratio: 0.0,
      rotation: 0.0,
    }
  }
}

impl FocusGeometry {
  pub fn new() -> Self {
    Self::default()
  }

  clamp_with_fn!(
    /// Horizontal center of the focus area as a fraction of the image width. Defaults to `0.5`.
    with_center_x, center_x, 0.0, 1.0
  );
  clamp_with_fn!(
    /// Vertical center of the focus area as a fraction of the image height. Defaults to `0.5`.
    with_center_y, center_y, 0.0, 1.0
  );
  positive_with_fn!(
    /// Distance from the center to where the blur reaches full strength. Defaults to `0.75`.
    with_radius, radius
  );
  clamp_with_fn!(
    /// Fraction of the radius that stays fully sharp. `1.0` is a hard edge. Defaults to `0.25`.
    with_sharpness, sharpness, 0.0, 1.0
  );
  clamp_with_fn!(
    /// Where in the fade the blur reaches half strength. Lower values blur sooner. Defaults to `0.5` (linear).
    with_midpoint, midpoint, 0.0, 1.0
  );
  clamp_with_fn!(
    /// Flattens the shape vertically: `0.0` is round, values toward `1.0` are flatter. Ignored by
    /// [`FocusShape::Horizontal`] and [`FocusShape::Vertical`]. Defaults to `0.0`.
    with_aspect_ratio, aspect_ratio, 0.0, 1.0
  );
  clamp_with_fn!(
    /// Clockwise rotation of the shape in degrees. Defaults to `0.0`.
    with_rotation, rotation, -180.0, 180.0
  );

  /// Blur strength at a pixel: `0.0` is fully sharp and `1.0` is fully blurred.
  /// - `p_x`, `p_y`: The pixel position.
  /// - `p_width`, `p_height`: The image dimensions.
  /// - `p_shape`: The shape of the focus area.
  fn weight(&self, p_x: f32, p_y: f32, p_width: f32, p_height: f32, p_shape: FocusShape) -> f32 {
    let unit = (p_width.min(p_height) / 2.0).max(1.0);
    let dx = (p_x - self.center_x * p_width) / unit;
    let dy = (p_y - self.center_y * p_height) / unit;
    // Rotate the point into the shape's own frame so the shape itself appears rotated clockwise.
    let (sin, cos) = self.rotation.to_radians().sin_cos();
    let local_x = dx * cos + dy * sin;
    let local_y = -dx * sin + dy * cos;
    // Keep a sliver of height so a fully flattened shape still has an inside.
    let flatten = 1.0 - self.aspect_ratio * 0.95;
    let (ax, ay) = (local_x.abs(), (local_y / flatten).abs());

    let distance = match p_shape {
      FocusShape::Circle => (ax * ax + ay * ay).sqrt(),
      FocusShape::Square => ax.max(ay),
      FocusShape::Diamond => ax + ay,
      FocusShape::Horizontal => local_y.abs(),
      FocusShape::Vertical => local_x.abs(),
    };

    let inner = self.radius * self.sharpness;
    if distance <= inner {
      return 0.0;
    }
    if distance >= self.radius {
      return 1.0;
    }
    let t = (distance - inner) / (self.radius - inner);
    // Bend the fade so it reaches 0.5 at the midpoint; a midpoint of 0.5 gives a straight line.
    let midpoint = self.midpoint.clamp(0.01, 0.99);
    t.powf(0.5f32.ln() / midpoint.ln())
  }
}

/// Blur outside a focus area while keeping the area itself sharp. Create one with [`focus_blur`], adjust it with the
/// `with_*` methods, then run it with [`Apply::apply`].
///
/// A mask or area set with [`Apply::with_options`] is combined with the focus area, so the blur only lands where
/// both allow it.
pub struct FocusBlur {
  /// Focus geometry configuration.
  geometry: FocusGeometry,
  /// Shape of the focus area.
  shape: FocusShape,
  /// Blur to apply outside the focus area.
  blur: BlurType,
  options: Options,
}

impl FocusBlur {
  /// Sets where the focus area sits and how its edge fades. Defaults to [`FocusGeometry::default`].
  pub fn with_geometry(mut self, p_geometry: FocusGeometry) -> Self {
    self.geometry = p_geometry;
    self
  }

  /// Sets the shape of the focus area. Defaults to [`FocusShape::Circle`].
  pub fn with_shape(mut self, p_shape: FocusShape) -> Self {
    self.shape = p_shape;
    self
  }

  /// Sets the blur applied outside the focus area, such as `gaussian_blur(12)` or `lens_blur(8)`. Any options set on
  /// that blur are replaced by the focus mask. Defaults to `gaussian_blur(25)`.
  pub fn with_blur(mut self, p_blur: impl Into<BlurType>) -> Self {
    self.blur = p_blur.into();
    self
  }

  /// Builds a mask covering the whole image: black where it stays sharp, white where it is fully blurred.
  /// When `p_mask` is given it is multiplied in, so the blur only lands where both masks allow it.
  fn focus_mask(&self, p_width: u32, p_height: u32, p_mask: Option<&Mask>) -> Mask {
    let (width, height) = (p_width as f32, p_height as f32);
    let user_mask = p_mask.map(|m| m.image().rgba()).filter(|m| m.len() == (p_width * p_height * 4) as usize);
    let mut pixels = vec![255u8; (p_width * p_height * 4) as usize];
    pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
      let x = (i as u32 % p_width) as f32 + 0.5;
      let y = (i as u32 / p_width) as f32 + 0.5;
      let mut weight = self.geometry.weight(x, y, width, height, self.shape);
      if let Some(user_mask) = user_mask {
        weight *= rgba_to_gray(&user_mask[i * 4..i * 4 + 4]) as f32 / 255.0;
      }
      let value = (weight * 255.0).round() as u8;
      chunk[..3].fill(value);
    });
    Mask::from_image(Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA))
  }
}

impl Apply for FocusBlur {
  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    let (width, height) = image.dimensions::<u32>();

    let user_options = self.options.as_ref();
    let focus_mask = self.focus_mask(width, height, user_options.and_then(|o| o.mask()));
    let mut apply_options = ApplyOptions::new().with_mask(focus_mask);
    if let Some(areas) = user_options.and_then(|o| o.area()) {
      apply_options = apply_options.with_areas(areas.to_vec());
    }

    match &self.blur {
      BlurType::Gaussian(blur) => blur.clone().with_options(apply_options).apply(image),
      BlurType::Lens(blur) => blur.clone().with_options(apply_options).apply(image),
    };
  }
}

/// Blurs the image outside a focus area. By default the focus is a circle in the center and the blur is
/// `gaussian_blur(25)`.
pub fn focus_blur() -> FocusBlur {
  FocusBlur {
    geometry: FocusGeometry::default(),
    shape: FocusShape::Circle,
    blur: gaussian_blur(25).into(),
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Color;

  /// A striped image, so any blur changes the pixels.
  fn striped_image(p_width: u32, p_height: u32) -> Image {
    let mut image = Image::new(p_width, p_height);
    for y in 0..p_height {
      for x in 0..p_width {
        let value = if x % 2 == 0 { 0 } else { 255 };
        image.set_pixel(x, y, (value, value, value, 255));
      }
    }
    image
  }

  fn pixel(p_image: &Image, p_x: u32, p_y: u32) -> (u8, u8, u8, u8) {
    p_image.get_pixel(p_x, p_y).unwrap()
  }

  #[test]
  fn circle_weight_is_sharp_inside_and_blurred_outside() {
    let geometry = FocusGeometry::new().with_radius(0.5).with_sharpness(1.0);
    assert_eq!(geometry.weight(50.0, 50.0, 100.0, 100.0, FocusShape::Circle), 0.0);
    assert_eq!(geometry.weight(0.0, 0.0, 100.0, 100.0, FocusShape::Circle), 1.0);
  }

  #[test]
  fn midpoint_sets_half_strength() {
    // Radius 1.0 with no sharp core: at 100x100 the fade runs from the center to 50px out.
    for midpoint in [0.2, 0.5, 0.8] {
      let geometry = FocusGeometry::new().with_radius(1.0).with_sharpness(0.0).with_midpoint(midpoint);
      let weight = geometry.weight(50.0 + 50.0 * midpoint, 50.0, 100.0, 100.0, FocusShape::Circle);
      assert!((weight - 0.5).abs() < 1e-3, "midpoint {midpoint} -> {weight}");
    }
  }

  #[test]
  fn horizontal_band_ignores_x() {
    let geometry = FocusGeometry::new().with_radius(0.2).with_sharpness(1.0);
    assert_eq!(geometry.weight(0.0, 50.0, 100.0, 100.0, FocusShape::Horizontal), 0.0);
    assert_eq!(geometry.weight(99.0, 50.0, 100.0, 100.0, FocusShape::Horizontal), 0.0);
    assert_eq!(geometry.weight(50.0, 0.0, 100.0, 100.0, FocusShape::Horizontal), 1.0);
  }

  #[test]
  fn rotation_turns_the_band() {
    let geometry = FocusGeometry::new().with_radius(0.2).with_sharpness(1.0).with_rotation(90.0);
    assert_eq!(geometry.weight(50.0, 0.0, 100.0, 100.0, FocusShape::Horizontal), 0.0);
    assert_eq!(geometry.weight(0.0, 50.0, 100.0, 100.0, FocusShape::Horizontal), 1.0);
  }

  #[test]
  fn keeps_center_sharp_and_blurs_corners() {
    let original = striped_image(64, 64);
    let mut image = original.clone();
    focus_blur()
      .with_geometry(FocusGeometry::new().with_radius(0.5).with_sharpness(1.0))
      .with_blur(gaussian_blur(3))
      .apply(&mut image);

    assert_eq!(pixel(&image, 32, 32), pixel(&original, 32, 32));
    assert_ne!(pixel(&image, 1, 1), pixel(&original, 1, 1));
  }

  #[test]
  fn black_user_mask_leaves_image_unchanged() {
    let original = striped_image(32, 32);
    let mut image = original.clone();
    let mask = Mask::from_image(Image::new_from_color(32, 32, Color::from_rgba(0, 0, 0, 255)));
    focus_blur()
      .with_blur(gaussian_blur(3))
      .with_options(ApplyOptions::new().with_mask(mask))
      .apply(&mut image);

    assert_eq!(image.rgba(), original.rgba());
  }
}
