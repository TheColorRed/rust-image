//! The stamping engine of `drawing` (`Sprites`, `Sprite`, `SpriteTrail`, `TrailPoint`, `Blend`) and the pieces of `abra-core` that make
//! things to stamp: the seeded `Random`, `CurveRange` and `value_noise`.
//!
//! A tuple of the library is an array here, and an array of the wrong length panics: a color is `[red, green, blue]`, a point is `[x, y]`.
//! `usize` is a `u32`.
//!
//! What is not exported, and why:
//! - `CurveRange` is made from a number, a pair, a `Curve` or two curves with `From` in the library; uniffi cannot overload, so each is a
//!   constructor: `from_number`, `from_pair`, `from_curve` and `new`.
//! - `Blend::Mode(BlendMode::Custom(..))` holds a Rust function, which cannot cross to other languages, as for `BlendMode`.
//! - `Sprites::with_mask`, `with_area` and the rest of an effect are the functions every effect has: see [`effect_methods!`].
//! - `Random::next_u64` is exported as `next_u64`, a `u64`.

use std::sync::{Arc, Mutex};

use abra::abra_core;
use abra::drawing::prelude as drawing;

use crate::effect::effect_methods;
use crate::{BlendMode, Curve, Image};

/// How a stamp is combined with what is under it: any blend mode, or `additive`, which adds its light.
#[derive(Clone, Copy, Debug, uniffi::Object)]
pub struct Blend(drawing::Blend);

impl Blend {
  /// The blend of the library, for other parts of the bindings that draw with one.
  pub(crate) fn core(&self) -> drawing::Blend {
    self.0
  }
}

#[uniffi::export]
impl Blend {
  /// Adds the drawn color's light to what is there, scaled by its opacity.
  #[uniffi::constructor]
  pub fn additive() -> Self {
    Self(drawing::Blend::Additive)
  }

  /// One of the blend modes. `Normal` is plain painting.
  #[uniffi::constructor]
  pub fn mode(mode: BlendMode) -> Self {
    Self(drawing::Blend::Mode(mode.to_core()))
  }
}

/// One stamp: a soft dot or a texture, scaled, turned, tinted and blended, at a place in the image.
#[derive(Clone, Debug, uniffi::Object)]
pub struct Sprite(drawing::Sprite);

#[uniffi::export]
impl Sprite {
  /// A white, fully opaque dot of radius `size` at `x`, `y`.
  #[uniffi::constructor]
  pub fn new(x: f32, y: f32, size: f32) -> Self {
    Self(drawing::Sprite::new(x, y, size))
  }

  pub fn with_opacity(&self, opacity: f32) -> Arc<Sprite> {
    Arc::new(Sprite(self.0.clone().with_opacity(opacity)))
  }

  /// The color, `[red, green, blue]`.
  pub fn with_color(&self, color: Vec<u8>) -> Arc<Sprite> {
    Arc::new(Sprite(self.0.clone().with_color(rgb(&color))))
  }

  /// How hard the edge of a dot is, from 0 (soft) to 1 (hard). A texture does not use it.
  pub fn with_hardness(&self, hardness: f32) -> Arc<Sprite> {
    Arc::new(Sprite(self.0.clone().with_hardness(hardness)))
  }

  /// Degrees, around its middle.
  pub fn with_rotation(&self, degrees: f32) -> Arc<Sprite> {
    Arc::new(Sprite(self.0.clone().with_rotation(degrees)))
  }

  pub fn with_blend(&self, blend: Arc<Blend>) -> Arc<Sprite> {
    Arc::new(Sprite(self.0.clone().with_blend(blend.0)))
  }

  /// The picture the sprite is drawn as, scaled so its longer side is twice `size`.
  pub fn with_texture(&self, texture: Arc<Image>) -> Arc<Sprite> {
    Arc::new(Sprite(self.0.clone().with_texture(texture.core())))
  }

  /// Takes the sprite's color from the image at `x`, `y`, read by the renderer so the pixels never leave it.
  pub fn with_color_from_image_at(&self, x: f32, y: f32) -> Arc<Sprite> {
    Arc::new(Sprite(self.0.clone().with_color_from_image_at(x, y)))
  }
}

/// One place along a trail.
#[derive(Clone, Copy, Debug, uniffi::Object)]
pub struct TrailPoint(drawing::TrailPoint);

#[uniffi::export]
impl TrailPoint {
  /// A point of a trail at `x`, `y` where the line is `width` wide in all.
  #[uniffi::constructor]
  pub fn new(x: f32, y: f32, width: f32) -> Self {
    Self(drawing::TrailPoint::new(x, y, width))
  }

  pub fn with_opacity(&self, opacity: f32) -> Arc<TrailPoint> {
    Arc::new(TrailPoint(self.0.with_opacity(opacity)))
  }

  /// The color, `[red, green, blue]`.
  pub fn with_color(&self, color: Vec<u8>) -> Arc<TrailPoint> {
    Arc::new(TrailPoint(self.0.with_color(rgb(&color))))
  }
}

/// A line through points, from its tail to its head, with a width, opacity and color at each.
#[derive(Clone, Debug, uniffi::Object)]
pub struct SpriteTrail(drawing::SpriteTrail);

#[uniffi::export]
impl SpriteTrail {
  /// A trail through these points, of at least two.
  #[uniffi::constructor]
  pub fn new(points: Vec<Arc<TrailPoint>>) -> Self {
    Self(drawing::SpriteTrail::new(points.iter().map(|point| point.0).collect()))
  }

  /// From 0 (a soft fade) to 1 (a hard edge).
  pub fn with_hardness(&self, hardness: f32) -> Arc<SpriteTrail> {
    Arc::new(SpriteTrail(self.0.clone().with_hardness(hardness)))
  }

  pub fn with_blend(&self, blend: Arc<Blend>) -> Arc<SpriteTrail> {
    Arc::new(SpriteTrail(self.0.clone().with_blend(blend.0)))
  }
}

/// Sprites and trails drawn over one image, trails first, then the sprites in the order they were added. Made by [`sprites`]. It is an
/// effect, so it has `apply`, `apply_deferred` and the rest.
#[derive(Clone, Debug, uniffi::Object)]
pub struct Sprites(drawing::Sprites);

impl Sprites {
  /// Sprites of the library, for other parts of the bindings that make them.
  pub(crate) fn from_core(sprites: drawing::Sprites) -> Arc<Sprites> {
    Arc::new(Sprites(sprites))
  }
}

/// Sprites and trails to draw, of none yet.
#[uniffi::export]
pub fn sprites() -> Arc<Sprites> {
  Arc::new(Sprites(drawing::sprites()))
}

#[uniffi::export]
impl Sprites {
  pub fn with_sprites(&self, sprites: Vec<Arc<Sprite>>) -> Arc<Sprites> {
    Arc::new(Sprites(self.0.clone().with_sprites(sprites.iter().map(|sprite| sprite.0.clone()).collect())))
  }

  /// Adds many sprites that share a blend and a texture in one call, 16 numbers for each: `x`, `y`, `size`, `opacity`, `red`, `green`, `blue`
  /// (0 to 255), `hardness`, `rotation`, then `1` and the point `x`, `y` when its color is read from the image there (`0`, `0`, `0` when not), then
  /// `stretch` (how much longer than wide, 0 for not at all), and the `columns`, `rows` and `frame` of a texture that is a grid of frames (`1`, `1`, `0`
  /// when it is not).
  /// A list that is not a whole number of sprites panics.
  pub fn add_sprite_values(&self, values: Vec<f32>, blend: Arc<Blend>, texture: Option<Arc<Image>>) -> Arc<Sprites> {
    Arc::new(Sprites(self.0.clone().add_sprite_values(&values, blend.0, texture.map(|texture| texture.core()))))
  }

  /// Adds many trails that share a hardness and a blend in one call: `lengths` says how many points each trail has, and `values` has 7 numbers for
  /// each point: `x`, `y`, `width`, `opacity`, `red`, `green`, `blue` (0 to 255). Lengths that do not agree with the values panic.
  pub fn add_trail_values(&self, values: Vec<f32>, lengths: Vec<u32>, hardness: f32, blend: Arc<Blend>) -> Arc<Sprites> {
    Arc::new(Sprites(self.0.clone().add_trail_values(&values, &lengths, hardness, blend.0)))
  }

  pub fn with_trails(&self, trails: Vec<Arc<SpriteTrail>>) -> Arc<Sprites> {
    Arc::new(Sprites(self.0.clone().with_trails(trails.iter().map(|trail| trail.0.clone()).collect())))
  }
}

effect_methods!(Sprites);

/// A seeded random number generator whose numbers are fixed by the library and not by a dependency, so the same seed gives the same numbers
/// everywhere (SplitMix64). It changes as numbers are taken from it.
#[derive(Debug, uniffi::Object)]
pub struct Random(Mutex<abra_core::Random>);

#[uniffi::export]
impl Random {
  #[uniffi::constructor]
  pub fn new(seed: u64) -> Self {
    Self(Mutex::new(abra_core::Random::new(seed)))
  }

  /// The next 64 random bits.
  pub fn next_u64(&self) -> u64 {
    self.0.lock().unwrap().next_u64()
  }

  /// A number from 0 up to, and not including, 1.
  pub fn float(&self) -> f32 {
    self.0.lock().unwrap().float()
  }

  /// `count` numbers from 0 up to 1, in one call.
  pub fn floats(&self, count: u32) -> Vec<f32> {
    self.0.lock().unwrap().floats(count as usize)
  }

  /// A number from `min` up to `max`.
  pub fn range(&self, min: f32, max: f32) -> f32 {
    self.0.lock().unwrap().range(min, max)
  }

  /// A whole number from 0 up to, and not including, `len`, which must be more than 0.
  pub fn index(&self, len: u32) -> u32 {
    self.0.lock().unwrap().index(len as usize) as u32
  }

  /// A point `[x, y]` evenly spread over a circle. The center is `[x, y]`.
  pub fn point_in_circle(&self, center: Vec<f32>, radius: f32) -> Vec<f32> {
    let point = self.0.lock().unwrap().point_in_circle(point_of(&center), radius);
    vec![point.0, point.1]
  }

  /// A point `[x, y]` evenly spread over the ring between two radii, given in either order.
  pub fn point_in_ring(&self, center: Vec<f32>, inner_radius: f32, outer_radius: f32) -> Vec<f32> {
    let point = self.0.lock().unwrap().point_in_ring(point_of(&center), inner_radius, outer_radius);
    vec![point.0, point.1]
  }

  /// A point `[x, y]` evenly spread over a rectangle.
  pub fn point_in_rect(&self, left: f32, top: f32, width: f32, height: f32) -> Vec<f32> {
    let point = self.0.lock().unwrap().point_in_rect(left, top, width, height);
    vec![point.0, point.1]
  }

  /// A point `[x, y]` evenly spread along a line from `[x, y]` to `[x, y]`.
  pub fn point_on_line(&self, from: Vec<f32>, to: Vec<f32>) -> Vec<f32> {
    let point = self.0.lock().unwrap().point_on_line(point_of(&from), point_of(&to));
    vec![point.0, point.1]
  }

  /// `count` points `[x, y]` on the parts of an image at least this light (0 to 1), so things come from its bright places. If no pixel is
  /// that light the points are anywhere on the image.
  pub fn points_in_light(&self, image: Arc<Image>, min_luma: f32, count: u32) -> Vec<Vec<f32>> {
    let points = self.0.lock().unwrap().points_in_light(&image.core(), min_luma, count as usize);
    points.into_iter().map(|point| vec![point.0, point.1]).collect()
  }
}

/// A value that follows a curve over a life, random between two curves. Each thing that uses it picks one place between the `min` and `max`
/// curves and keeps it, so no two are alike. A curve's `x` is the age from 0 (birth) to 255 (end of life) and its `y` is the value.
#[derive(Clone, Debug, uniffi::Object)]
pub struct CurveRange(abra_core::CurveRange);

impl CurveRange {
  /// The range of the library, for other parts of the bindings that take one.
  pub(crate) fn core(&self) -> abra_core::CurveRange {
    self.0.clone()
  }
}

#[uniffi::export]
impl CurveRange {
  /// Between two curves.
  #[uniffi::constructor]
  pub fn new(min: Arc<Curve>, max: Arc<Curve>) -> Self {
    Self(abra_core::CurveRange::new(min.core(), max.core()))
  }

  /// A flat value.
  #[uniffi::constructor]
  pub fn from_number(value: f32) -> Self {
    Self(abra_core::CurveRange::from(value))
  }

  /// A flat range from `min` to `max`.
  #[uniffi::constructor]
  pub fn from_pair(min: f32, max: f32) -> Self {
    Self(abra_core::CurveRange::from((min, max)))
  }

  /// One curve, with no spread.
  #[uniffi::constructor]
  pub fn from_curve(curve: Arc<Curve>) -> Self {
    Self(abra_core::CurveRange::from(curve.core()))
  }

  /// Whether the value is zero at every age, for every pick.
  pub fn is_zero(&self) -> bool {
    self.0.is_zero()
  }

  /// The value at `age` (0 to 1) for the pick `pick` (0 for `min`, 1 for `max`).
  pub fn at(&self, age: f32, pick: f32) -> f32 {
    self.0.at(age, pick)
  }

  /// The value added up from birth to `age`, for a range that is a velocity.
  pub fn area(&self, age: f32, pick: f32) -> f32 {
    self.0.area(age, pick)
  }

  /// `at` for many things in one call: the value at each age for the pick at the same place in `picks`. Lists of different lengths panic.
  pub fn at_many(&self, ages: Vec<f32>, picks: Vec<f32>) -> Vec<f32> {
    self.0.at_many(&ages, &picks)
  }

  /// `area` for many things in one call, as `at_many` is for `at`.
  pub fn area_many(&self, ages: Vec<f32>, picks: Vec<f32>) -> Vec<f32> {
    self.0.area_many(&ages, &picks)
  }
}

/// Smooth noise from -1 to 1 that changes slowly across `x` and `y`. The same `seed` gives the same noise.
#[uniffi::export]
pub fn value_noise(x: f32, y: f32, seed: u64) -> f32 {
  abra_core::value_noise(x, y, seed)
}

/// A color of the library from `[red, green, blue]`. Panics if the array is not three numbers.
fn rgb(color: &[u8]) -> (u8, u8, u8) {
  match color {
    [red, green, blue] => (*red, *green, *blue),
    _ => panic!("a color is [red, green, blue]"),
  }
}

/// A point of the library from `[x, y]`. Panics if the array is not two numbers.
fn point_of(point: &[f32]) -> (f32, f32) {
  match point {
    [x, y] => (*x, *y),
    _ => panic!("a point is [x, y]"),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_same_seed_gives_the_same_numbers_through_the_bridge() {
    assert_eq!(Random::new(7).floats(5), Random::new(7).floats(5));
    assert_eq!(Random::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
  }

  #[test]
  fn sprites_are_drawn_on_an_image() {
    let image = Arc::new(Image::new(20, 20));
    let dot = Sprite::new(10.5, 10.5, 4.0).with_hardness(1.0).with_color(vec![255, 0, 0]);
    sprites().with_sprites(vec![dot]).apply_on_cpu(image.clone());
    assert_eq!(image.get_pixel(10, 10), Some(vec![255, 0, 0, 255]));
    assert_eq!(image.get_pixel(1, 1).unwrap()[3], 0);
  }

  #[test]
  fn a_trail_is_drawn_through_its_points() {
    let image = Arc::new(Image::new(40, 20));
    let points = vec![TrailPoint::new(5.0, 10.5, 6.0).into(), TrailPoint::new(35.0, 10.5, 6.0).into()];
    let trail = SpriteTrail::new(points).with_hardness(1.0);
    sprites().with_trails(vec![trail]).apply_on_cpu(image.clone());
    assert_eq!(image.get_pixel(20, 10).unwrap()[3], 255);
    assert_eq!(image.get_pixel(20, 2).unwrap()[3], 0);
  }

  #[test]
  fn a_range_is_read_by_age_and_pick() {
    let range = CurveRange::from_pair(2.0, 6.0);
    assert_eq!(range.at(0.5, 0.25), 3.0);
    assert!(CurveRange::from_number(0.0).is_zero());
  }

  #[test]
  #[should_panic(expected = "a color is [red, green, blue]")]
  fn a_color_that_is_not_three_numbers_panics() {
    Sprite::new(0.0, 0.0, 1.0).with_color(vec![1, 2]);
  }
}
