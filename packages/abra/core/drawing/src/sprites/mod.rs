//! Draws many small sprites (soft dots or textures that are scaled, turned, tinted and blended) and trails (a line with a width,
//! opacity and color at each point) over an image. Whoever makes the sprites works out where they are; this draws them, on the GPU
//! when it can and on the CPU otherwise, with the same result.

mod draw;
mod gpu;

use crate::Blend;
use abra_core::Image;
use abra_core::image::effect::{Effect, Options};
use abra_core::image::gpu::GpuProcessor;

pub use gpu::{SPRITE_MAX_WORDS, SPRITE_WORDS, TRAIL_POINT_WORDS, TRAIL_WORDS};

// (SPRITE_VALUES and TRAIL_POINT_VALUES, below, are the lists of numbers `add_sprite_values` and `add_trail_values` take.)

/// One sprite: where it is, how big, how opaque and what color, in pixels of the image.
#[derive(Clone, Debug)]
pub struct Sprite {
  x: f32,
  y: f32,
  /// The radius of a dot, or half the longer side of a texture.
  size: f32,
  opacity: f32,
  color: (u8, u8, u8),
  /// How hard the edge of a dot is, from 0.0 (soft) to 1.0 (hard), as a brush's hardness. A texture does not use it.
  hardness: f32,
  /// Degrees, around its middle. A dot does not show it.
  rotation: f32,
  blend: Blend,
  /// The picture it is drawn as, scaled so its longer side is twice `size`. `None` is a soft round dot. The color tints it.
  texture: Option<Image>,
  /// When set, its color is the image's color at this point and `color` is not used.
  birth: Option<(f32, f32)>,
  /// How much longer it is than wide, along the direction of its rotation: 0.0 is as long as wide, 1.0 twice as long. A dot becomes an oval.
  stretch: f32,
  /// A texture that is a grid of frames, `columns` across and `rows` down, shows only the one numbered `frame` (counting along each row, then down).
  columns: u32,
  rows: u32,
  frame: u32,
}

impl Sprite {
  /// A white, fully opaque dot of radius `p_size` at `p_x`, `p_y`.
  pub fn new(p_x: f32, p_y: f32, p_size: f32) -> Self {
    Sprite {
      x: p_x,
      y: p_y,
      size: p_size,
      opacity: 1.0,
      color: (255, 255, 255),
      hardness: 0.0,
      rotation: 0.0,
      blend: Blend::default(),
      texture: None,
      birth: None,
      stretch: 0.0,
      columns: 1,
      rows: 1,
      frame: 0,
    }
  }

  pub fn with_opacity(mut self, p_opacity: f32) -> Self {
    self.opacity = p_opacity;
    self
  }

  pub fn with_color(mut self, p_color: (u8, u8, u8)) -> Self {
    self.color = p_color;
    self
  }

  pub fn with_hardness(mut self, p_hardness: f32) -> Self {
    self.hardness = p_hardness.clamp(0.0, 1.0);
    self
  }

  pub fn with_rotation(mut self, p_degrees: f32) -> Self {
    self.rotation = p_degrees;
    self
  }

  pub fn with_blend(mut self, p_blend: impl Into<Blend>) -> Self {
    self.blend = p_blend.into();
    self
  }

  pub fn with_texture(mut self, p_texture: Image) -> Self {
    self.texture = Some(p_texture);
    self
  }

  /// Makes the sprite longer along the direction of its rotation: `p_stretch` is how much longer, 0.0 for none and 1.0 for twice as long. Turn the
  /// sprite to the way it is going and stretch it by how fast it is going to get a streak of motion.
  pub fn with_stretch(mut self, p_stretch: f32) -> Self {
    self.stretch = p_stretch.max(0.0);
    self
  }

  /// Draws only one frame of a texture that is a grid of `p_columns` x `p_rows` frames of the same size: `p_frame` counts along each row, then down,
  /// and a number past the last frame is held at it. The frames are cut at whole pixels.
  pub fn with_frame(mut self, p_columns: u32, p_rows: u32, p_frame: u32) -> Self {
    let (columns, rows) = (p_columns.max(1), p_rows.max(1));
    (self.columns, self.rows, self.frame) = (columns, rows, p_frame.min(columns * rows - 1));
    self
  }

  /// Takes the sprite's color from the image at `p_x`, `p_y`, read by the renderer so the pixels never have to leave it.
  pub fn with_color_from_image_at(mut self, p_x: f32, p_y: f32) -> Self {
    self.birth = Some((p_x, p_y));
    self
  }

  /// Whether every number of the sprite is real (not NaN, not infinite). A sprite that has one cannot be placed, and a NaN cast to a pixel is 0, so
  /// it would be drawn at the corner of the image instead of nowhere.
  fn is_real(&self) -> bool {
    [self.x, self.y, self.size, self.opacity, self.hardness, self.rotation, self.stretch].iter().all(|value| value.is_finite())
      && self.birth.is_none_or(|(x, y)| x.is_finite() && y.is_finite())
  }

  /// How many pixels out from its middle it can reach: a texture is a rectangle that can turn, so it reaches as far as its corners, and
  /// a dot fades out at its size.
  fn reach(&self) -> i64 {
    ((if self.texture.is_some() { self.size * std::f32::consts::SQRT_2 } else { self.size }) * (1.0 + self.stretch)).ceil() as i64 + 1
  }
}

/// One place along a trail.
#[derive(Clone, Copy, Debug)]
pub struct TrailPoint {
  x: f32,
  y: f32,
  /// The whole width of the line here.
  width: f32,
  opacity: f32,
  color: (u8, u8, u8),
}

impl TrailPoint {
  fn is_real(&self) -> bool {
    [self.x, self.y, self.width, self.opacity].iter().all(|value| value.is_finite())
  }

  pub fn new(p_x: f32, p_y: f32, p_width: f32) -> Self {
    TrailPoint {
      x: p_x,
      y: p_y,
      width: p_width,
      opacity: 1.0,
      color: (255, 255, 255),
    }
  }

  pub fn with_opacity(mut self, p_opacity: f32) -> Self {
    self.opacity = p_opacity;
    self
  }

  pub fn with_color(mut self, p_color: (u8, u8, u8)) -> Self {
    self.color = p_color;
    self
  }
}

/// A line through points, from its tail to its head, drawn with a width, opacity and color that follow the points.
#[derive(Clone, Debug)]
pub struct SpriteTrail {
  points: Vec<TrailPoint>,
  /// From 0.0 (a soft fade) to 1.0 (a hard edge).
  hardness: f32,
  blend: Blend,
}

impl SpriteTrail {
  /// A trail through `p_points`, of at least two.
  pub fn new(p_points: Vec<TrailPoint>) -> Self {
    SpriteTrail {
      points: p_points,
      hardness: 0.5,
      blend: Blend::default(),
    }
  }

  pub fn with_hardness(mut self, p_hardness: f32) -> Self {
    self.hardness = p_hardness.clamp(0.0, 1.0);
    self
  }

  pub fn with_blend(mut self, p_blend: impl Into<Blend>) -> Self {
    self.blend = p_blend.into();
    self
  }
}

/// Sprites and trails drawn over one image, trails first, then the sprites in the order they were added. It is an abra effect:
/// `sprites.apply(&mut image)` draws it now, on the GPU when every blend has a shader and the lists fit, and on the CPU otherwise.
#[derive(Clone, Debug)]
pub struct Sprites {
  sprites: Vec<Sprite>,
  trails: Vec<SpriteTrail>,
  options: Options,
}

pub fn sprites() -> Sprites {
  // The first batch anyone makes has the GPU compile the sprite shader in the background, so drawing it does not wait for that.
  static WARMED: std::sync::Once = std::sync::Once::new();
  WARMED.call_once(|| {
    abra_core::image::gpu::register_warmup(|| sprites().with_sprites(vec![Sprite::new(1.0, 1.0, 1.0)]).passes(2, 2))
  });
  Sprites {
    sprites: Vec::new(),
    trails: Vec::new(),
    options: None,
  }
}

impl Sprites {
  /// The sprites to draw, in order. A sprite with a number that is not real (NaN or infinite: a missing value, a division by zero) is left out, so
  /// one bad value draws nothing and does not put a dot at the corner of the image or spoil the batch.
  pub fn with_sprites(mut self, p_sprites: Vec<Sprite>) -> Self {
    self.sprites = p_sprites.into_iter().filter(Sprite::is_real).collect();
    self
  }

  /// The trails to draw, under the sprites. A trail with a number that is not real is left out, as a sprite is.
  pub fn with_trails(mut self, p_trails: Vec<SpriteTrail>) -> Self {
    self.trails = p_trails.into_iter().filter(|trail| trail.hardness.is_finite() && trail.points.iter().all(TrailPoint::is_real)).collect();
    self
  }
}

/// How many numbers describe one sprite in [`Sprites::add_sprite_values`]: `x`, `y`, `size`, `opacity`, `red`, `green`, `blue` (0 to 255),
/// `hardness`, `rotation`, then `1.0` and the point `x`, `y` when its color is read from the image there (`0.0` and two zeros when not), then
/// `stretch`, and the `columns`, `rows` and `frame` of a texture that is a grid of frames (`1`, `1` and `0` for one that is not).
pub const SPRITE_VALUES: usize = 16;

/// How many numbers describe one trail point in [`Sprites::add_trail_values`]: `x`, `y`, `width`, `opacity`, `red`, `green`, `blue` (0 to 255).
pub const TRAIL_POINT_VALUES: usize = 7;

impl Sprites {
  /// Adds many sprites that share a blend and a texture from one list of numbers, [`SPRITE_VALUES`] for each, so whoever works out thousands of
  /// sprites does not have to make an object for every one. It adds them after the sprites already there, and leaves out a sprite that has a
  /// number that is not real, as [`with_sprites`](Sprites::with_sprites) does.
  ///
  /// # Panics
  /// When the length of `p_values` is not a whole number of sprites.
  pub fn add_sprite_values(mut self, p_values: &[f32], p_blend: impl Into<Blend>, p_texture: Option<Image>) -> Self {
    assert!(p_values.len() % SPRITE_VALUES == 0, "a sprite is {SPRITE_VALUES} numbers");
    let blend = p_blend.into();
    for value in p_values.chunks_exact(SPRITE_VALUES) {
      let mut sprite = Sprite::new(value[0], value[1], value[2])
        .with_opacity(value[3])
        .with_color((value[4].clamp(0.0, 255.0) as u8, value[5].clamp(0.0, 255.0) as u8, value[6].clamp(0.0, 255.0) as u8))
        .with_hardness(value[7])
        .with_rotation(value[8])
        .with_blend(blend);
      if value[9] != 0.0 {
        sprite = sprite.with_color_from_image_at(value[10], value[11]);
      }
      sprite = sprite.with_stretch(value[12]).with_frame(value[13].max(1.0) as u32, value[14].max(1.0) as u32, value[15].max(0.0) as u32);
      sprite.texture = p_texture.clone();
      if sprite.is_real() {
        self.sprites.push(sprite);
      }
    }
    self
  }

  /// Adds many trails that share a hardness and a blend from one list of numbers: `p_lengths` says how many points each trail has, and
  /// `p_values` has [`TRAIL_POINT_VALUES`] for each point, trail after trail. A trail that has a number that is not real is left out.
  ///
  /// # Panics
  /// When the lengths and the values do not agree.
  pub fn add_trail_values(mut self, p_values: &[f32], p_lengths: &[u32], p_hardness: f32, p_blend: impl Into<Blend>) -> Self {
    let total: usize = p_lengths.iter().map(|length| *length as usize).sum();
    assert!(p_values.len() == total * TRAIL_POINT_VALUES, "a trail point is {TRAIL_POINT_VALUES} numbers, and the lengths add up to {total} points");
    let blend = p_blend.into();
    let mut points = p_values.chunks_exact(TRAIL_POINT_VALUES);
    for length in p_lengths {
      let trail_points: Vec<TrailPoint> = points
        .by_ref()
        .take(*length as usize)
        .map(|value| {
          TrailPoint::new(value[0], value[1], value[2])
            .with_opacity(value[3])
            .with_color((value[4].clamp(0.0, 255.0) as u8, value[5].clamp(0.0, 255.0) as u8, value[6].clamp(0.0, 255.0) as u8))
        })
        .collect();
      let trail = SpriteTrail::new(trail_points).with_hardness(p_hardness).with_blend(blend);
      if trail.hardness.is_finite() && trail.points.iter().all(TrailPoint::is_real) {
        self.trails.push(trail);
      }
    }
    self
  }
}

impl Effect for Sprites {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  /// Sprites are placed in pixels of the whole image, so an area crops what is drawn and never moves it.
  fn positional(&self) -> bool {
    true
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    self.gpu_supported().then_some(self)
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    draw::draw_on_cpu(self, p_image);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;

  fn picture() -> Image {
    let pixels: Vec<u8> = (0..16 * 16).flat_map(|i| [(i % 16 * 16) as u8, (i / 16 * 16) as u8, 40, 255]).collect();
    Image::new_from_pixels(16, 16, pixels, Channels::RGBA)
  }

  #[test]
  fn a_sprite_takes_the_color_of_the_image_where_it_was_born() {
    let mut image = picture();
    let born = picture().rgba()[(2 * 16 + 3) * 4..(2 * 16 + 3) * 4 + 3].to_vec();
    // A big opaque dot born at one pixel and drawn at the far corner is that pixel's color at its middle.
    sprites()
      .with_sprites(vec![Sprite::new(12.5, 12.5, 3.0).with_hardness(1.0).with_color_from_image_at(3.5, 2.5)])
      .apply_on_cpu(&mut image);
    let at = (12 * 16 + 12) * 4;
    assert_eq!(&image.rgba()[at..at + 3], born.as_slice());
  }

  #[test]
  fn sprites_and_trails_from_lists_of_numbers_draw_what_the_objects_draw() {
    let by_objects = sprites()
      .with_sprites(vec![
        Sprite::new(4.5, 5.5, 3.0).with_opacity(0.5).with_color((200, 10, 30)).with_hardness(0.5).with_rotation(20.0),
        Sprite::new(11.5, 9.5, 2.0).with_color_from_image_at(3.5, 2.5),
      ])
      .with_trails(vec![
        SpriteTrail::new(vec![TrailPoint::new(1.0, 1.0, 3.0), TrailPoint::new(14.0, 3.0, 5.0).with_opacity(0.6).with_color((9, 99, 199))])
          .with_hardness(0.8),
      ]);
    let by_numbers = sprites()
      .add_sprite_values(
        &[
          4.5, 5.5, 3.0, 0.5, 200.0, 10.0, 30.0, 0.5, 20.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 11.5, 9.5, 2.0, 1.0, 255.0, 255.0, 255.0, 0.0, 0.0, 1.0, 3.5, 2.5,
          0.0, 1.0, 1.0, 0.0,
        ],
        Blend::default(),
        None,
      )
      .add_trail_values(&[1.0, 1.0, 3.0, 1.0, 255.0, 255.0, 255.0, 14.0, 3.0, 5.0, 0.6, 9.0, 99.0, 199.0], &[2], 0.8, Blend::default());
    let (mut a, mut b) = (picture(), picture());
    by_objects.apply_on_cpu(&mut a);
    by_numbers.apply_on_cpu(&mut b);
    assert_eq!(a.rgba(), b.rgba());
  }

  #[test]
  fn a_sprite_or_trail_with_a_number_that_is_not_real_is_left_out() {
    let mut image = picture();
    let good = Sprite::new(8.5, 8.5, 3.0).with_hardness(1.0);
    let bad = [
      Sprite::new(f32::NAN, 8.0, 3.0),
      Sprite::new(8.0, f32::INFINITY, 3.0),
      Sprite::new(8.0, 8.0, f32::NAN),
      Sprite::new(8.0, 8.0, 3.0).with_opacity(f32::NAN),
      Sprite::new(8.0, 8.0, 3.0).with_color_from_image_at(f32::NAN, 0.0),
    ];
    let trails = vec![
      SpriteTrail::new(vec![TrailPoint::new(0.0, 0.0, 4.0), TrailPoint::new(f32::NAN, 10.0, 4.0)]),
      SpriteTrail::new(vec![TrailPoint::new(0.0, 0.0, f32::INFINITY), TrailPoint::new(10.0, 10.0, 4.0)]),
    ];
    let batch = sprites().with_sprites(std::iter::once(good).chain(bad).collect()).with_trails(trails);
    assert_eq!((batch.sprites.len(), batch.trails.len()), (1, 0));
    let before = image.rgba().to_vec();
    batch.apply_on_cpu(&mut image);
    // Only the good sprite drew, and nothing was put at the corner by a bad one.
    assert_ne!(image.rgba(), before.as_slice());
    assert_eq!(image.rgba()[..4], before[..4], "the corner is untouched");
  }

  #[test]
  fn a_tile_lists_every_piece_of_a_trail_that_reaches_a_pixel_of_it() {
    use super::gpu::{TILE, trail_tiles};
    let (width, height) = (96u32, 64u32);
    // Trails that zigzag, cross corners and have a different width at every point.
    let trails: Vec<SpriteTrail> = (0..40u32)
      .map(|seed| {
        let at = |n: u32| ((seed * 7919 + n * 104_729) % 1000) as f32 / 1000.0;
        SpriteTrail::new((0..12u32).map(|n| TrailPoint::new(at(n * 2) * 96.0, at(n * 2 + 1) * 64.0, 1.0 + at(n + 50) * 14.0)).collect())
      })
      .collect();
    for trail in &trails {
      let listed = trail_tiles(trail, width, height);
      let tiles_x = width.div_ceil(TILE) as usize;
      for (piece, pair) in trail.points.windows(2).enumerate() {
        let reach = pair[0].width.max(pair[1].width) / 2.0 + 1.5;
        let line = (pair[1].x - pair[0].x, pair[1].y - pair[0].y);
        let length_squared = line.0 * line.0 + line.1 * line.1;
        for py in 0..height {
          for px in 0..width {
            let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
            let along = if length_squared > 0.0 { (((x - pair[0].x) * line.0 + (y - pair[0].y) * line.1) / length_squared).clamp(0.0, 1.0) } else { 0.0 };
            let (dx, dy) = (x - (pair[0].x + line.0 * along), y - (pair[0].y + line.1 * along));
            if (dx * dx + dy * dy).sqrt() >= reach {
              continue;
            }
            let tile = (py / TILE) as usize * tiles_x + (px / TILE) as usize;
            let (_, first, last, mask) = *listed.iter().find(|(listed_tile, ..)| *listed_tile == tile).expect("a tile a piece reaches is listed");
            assert!(piece as u32 >= first && piece as u32 <= last, "piece {piece} is inside the range of tile {tile}");
            if last - first < 32 {
              assert!(mask >> (piece as u32 - first) & 1 == 1, "piece {piece} is in the mask of tile {tile}");
            }
          }
        }
      }
    }
  }

  #[test]
  fn a_custom_blend_keeps_the_batch_on_the_cpu() {
    fn custom(a: (u8, u8, u8, u8), _b: (u8, u8, u8, u8)) -> (u8, u8, u8, u8) {
      a
    }
    let plain = sprites().with_sprites(vec![Sprite::new(4.0, 4.0, 2.0)]);
    assert!(plain.gpu_processor().is_some());
    let custom =
      sprites().with_sprites(vec![Sprite::new(4.0, 4.0, 2.0).with_blend(abra_core::BlendMode::Custom(custom))]);
    assert!(custom.gpu_processor().is_none());
  }
}

#[cfg(test)]
mod timing {
  use super::*;
  use abra_core::image::gpu::GpuProcessor;

  #[test]
  #[ignore = "a measurement, not a check: run it with --release --ignored --nocapture"]
  fn how_long_the_lists_for_three_thousand_trails_take_to_make() {
    let (width, height) = (4000u32, 3000u32);
    let trails: Vec<SpriteTrail> = (0..3000)
      .map(|i| {
        let (x, y) = ((i % 100) as f32 * 38.0 + 10.0, (i / 100) as f32 * 95.0 + 10.0);
        SpriteTrail::new((0..=24).map(|p| TrailPoint::new(x + p as f32 * 3.0, y + p as f32 * 4.0, 6.0).with_opacity(0.8)).collect())
      })
      .collect();
    let dots: Vec<Sprite> = (0..3000).map(|i| Sprite::new((i % 100) as f32 * 38.0, (i / 100) as f32 * 95.0, 5.0)).collect();
    let batch = sprites().with_trails(trails).with_sprites(dots);
    for _ in 0..3 {
      let started = std::time::Instant::now();
      let passes = batch.passes(width, height);
      println!("lists: {:?} ({} passes)", started.elapsed(), passes.len());
    }
  }
}

#[cfg(test)]
mod gpu_timing {
  use super::*;
  use abra_core::image::effect::Effect;

  #[test]
  #[ignore = "a measurement, not a check: run it with --release --ignored --nocapture"]
  fn how_long_the_gpu_takes_to_draw_three_thousand_trails_on_the_preview() {
    ::gpu::register();
    let (width, height) = (1024u32, 768u32);
    let trails: Vec<SpriteTrail> = (0..3000)
      .map(|i| {
        let (x, y) = ((i % 60) as f32 * 16.0 + 10.0, (i / 60) as f32 * 14.0 + 10.0);
        SpriteTrail::new((0..=24).map(|p| TrailPoint::new(x + p as f32 * 2.0, y + p as f32 * 3.0, 6.0).with_opacity(0.8)).collect())
      })
      .collect();
    let dots: Vec<Sprite> = (0..3000).map(|i| Sprite::new((i % 60) as f32 * 16.0, (i / 60) as f32 * 14.0, 5.0)).collect();
    let batch = sprites().with_trails(trails).with_sprites(dots);
    let mut image = Image::new(width, height);
    for round in 0..4 {
      let lists = std::time::Instant::now();
      let _ = batch.passes(width, height);
      let lists = lists.elapsed();
      let started = std::time::Instant::now();
      batch.apply(&mut image);
      println!("round {round}: lists {lists:?}, whole draw with upload and readback {:?}", started.elapsed());
    }
  }
}
