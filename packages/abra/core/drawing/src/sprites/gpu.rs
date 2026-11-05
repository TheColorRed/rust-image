//! The GPU's way of drawing sprites and trails: they are handed to the shader as a list of words, with the image cut into squares and the
//! sprites and trails that touch each one in the order they are drawn. See `sprites.wgsl` for what the words are.

use super::{SpriteTrail, Sprites, TrailPoint};
use abra_core::image::gpu::{GpuAux, GpuPass, GpuProcessor};
use abra_core::{BLEND_MODES_WGSL, Image};
use rayon::prelude::*;
use std::sync::{Arc, OnceLock};

/// How many words wide the texture that carries the sprites to the GPU is. A word is 32 bits and a texel holds one.
const AUX_WIDTH: u32 = 4096;
/// The most rows that texture may have, which is the usual largest side of a texture.
const AUX_ROWS: u32 = 8192;
/// The most words a batch may need and still go to the GPU. Counting the lists of what touches each tile, it stays inside the texture.
pub const SPRITE_MAX_WORDS: usize = 24_000_000;
/// The words one sprite, one trail and one point of a trail take.
pub const SPRITE_WORDS: usize = 21;
pub const TRAIL_WORDS: usize = 8;
pub const TRAIL_POINT_WORDS: usize = 8;
/// How many pieces after its first a trail's entry in a tile's list can name in its mask: a mask is one word.
pub(super) const TRAIL_MASK_PIECES: u32 = 32;
/// The side, in pixels, of the squares the image is cut into, each with the list of what touches it.
pub(super) const TILE: u32 = 16;
/// Set in the blend word of a sprite whose color is the picture's at its birth point.
const FROM_PIXEL: u32 = 0x1_0000;
/// The texture word of a sprite that is a dot.
const NO_SPRITE: u32 = u32::MAX;
/// Set in an item of a tile's list that is a trail and not a sprite.
const TRAIL_ITEM: u32 = 0x8000_0000;

impl Sprites {
  /// Whether the GPU can draw the batch: every blend needs a shader (a custom blend is a Rust function), and the lists must fit in a
  /// texture. Anything else is drawn by the CPU.
  pub(crate) fn gpu_supported(&self) -> bool {
    let blends = self.sprites.iter().map(|sprite| sprite.blend).chain(self.trails.iter().map(|trail| trail.blend));
    blends.into_iter().all(|blend| blend.shader_code().is_some()) && self.gpu_words() <= SPRITE_MAX_WORDS
  }

  /// The words the sprites, trails and textures take, without the lists of what touches each tile.
  fn gpu_words(&self) -> usize {
    let mut seen: Vec<u64> = Vec::new();
    let mut texture_words = 0usize;
    for sprite in &self.sprites {
      if let Some(texture) = &sprite.texture
        && !seen.contains(&texture.version())
      {
        seen.push(texture.version());
        let (width, height) = texture.dimensions::<usize>();
        texture_words = texture_words.saturating_add(width.saturating_mul(height));
      }
    }
    let points: usize = self.trails.iter().map(|trail| trail.points.len()).sum();
    self.sprites.len() * SPRITE_WORDS + self.trails.len() * TRAIL_WORDS + points * TRAIL_POINT_WORDS + texture_words
  }
}

impl GpuProcessor for Sprites {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    vec![gpu_pass(self, p_width, p_height)]
  }
}

/// The shader, joined after the blend modes it uses. It is made once, so the renderer can tell it is the same shader every time.
fn gpu_shader() -> &'static str {
  static SHADER: OnceLock<String> = OnceLock::new();
  SHADER.get_or_init(|| format!("{BLEND_MODES_WGSL}{}", include_str!("./sprites.wgsl"))).as_str()
}

/// Calls `p_each` with the number of every tile that the pixels from `p_left` to `p_right` and `p_top` to `p_bottom` touch.
pub(super) fn for_each_tile(p_left: i64, p_top: i64, p_right: i64, p_bottom: i64, p_width: u32, p_height: u32, mut p_each: impl FnMut(usize)) {
  let (left, top) = (p_left.max(0), p_top.max(0));
  let (right, bottom) = (p_right.min(p_width as i64 - 1), p_bottom.min(p_height as i64 - 1));
  if left > right || top > bottom {
    return;
  }
  let tiles_x = p_width.div_ceil(TILE) as i64;
  for tile_y in top / TILE as i64..=bottom / TILE as i64 {
    for tile_x in left / TILE as i64..=right / TILE as i64 {
      p_each((tile_y * tiles_x + tile_x) as usize);
    }
  }
}

/// Whether the segment from `p_a` to `p_b` comes within `p_reach` pixels of the square with corners `p_square` (left, top, right, bottom). Two shapes
/// that do not overlap are closest at a corner of one of them, so it is enough to test the ends of the segment against the square and the corners of
/// the square against the segment, after a test for the segment crossing the square.
fn segment_reaches_square(p_a: (f32, f32), p_b: (f32, f32), p_square: (f32, f32, f32, f32), p_reach: f32) -> bool {
  let (left, top, right, bottom) = p_square;
  let reach_squared = p_reach * p_reach;
  let to_square = |(x, y): (f32, f32)| {
    let (dx, dy) = ((left - x).max(0.0).max(x - right), (top - y).max(0.0).max(y - bottom));
    dx * dx + dy * dy
  };
  if to_square(p_a) <= reach_squared || to_square(p_b) <= reach_squared {
    return true;
  }
  let line = (p_b.0 - p_a.0, p_b.1 - p_a.1);
  let length_squared = line.0 * line.0 + line.1 * line.1;
  let to_segment = |(x, y): (f32, f32)| {
    let along = if length_squared > 0.0 { (((x - p_a.0) * line.0 + (y - p_a.1) * line.1) / length_squared).clamp(0.0, 1.0) } else { 0.0 };
    let (dx, dy) = (x - (p_a.0 + line.0 * along), y - (p_a.1 + line.1 * along));
    dx * dx + dy * dy
  };
  if [(left, top), (right, top), (left, bottom), (right, bottom)].into_iter().any(|corner| to_segment(corner) <= reach_squared) {
    return true;
  }
  // Neither end is near the square and no corner is near the line, so the line is either clear of the square or goes through it: clip it to the square.
  let (mut enter, mut leave) = (0.0f32, 1.0f32);
  for (start, delta, low, high) in [(p_a.0, line.0, left, right), (p_a.1, line.1, top, bottom)] {
    if delta == 0.0 {
      if start < low || start > high {
        return false;
      }
    } else {
      let (near, far) = ((low - start) / delta, (high - start) / delta);
      enter = enter.max(near.min(far));
      leave = leave.min(near.max(far));
    }
  }
  enter <= leave
}

/// The tiles a trail touches, each with the first and last of the trail's pieces (one between each two points) that reach into it and a mask of
/// which of them do (bit `n` is the piece `n` after the first), in order of tile. A piece reaches as far as its widest end, and a pixel past that
/// far from every piece gets no color from the trail. A trail that doubles back is in a tile at both ends of its line and not between, so the
/// first and last alone would have every pixel test the whole line; the mask has it test only the pieces that reach the tile. A range of more than
/// [`TRAIL_MASK_PIECES`] pieces has no room in the mask, so every piece of it is tested.
pub(super) fn trail_tiles(p_trail: &SpriteTrail, p_width: u32, p_height: u32) -> Vec<(usize, u32, u32, u32)> {
  let mut touched: Vec<(usize, u32)> = Vec::new();
  for (piece, pair) in p_trail.points.windows(2).enumerate() {
    let (a, b): (&TrailPoint, &TrailPoint) = (&pair[0], &pair[1]);
    let reach = a.width.max(b.width) / 2.0 + 1.5;
    let (left, right) = (a.x.min(b.x) - reach, a.x.max(b.x) + reach);
    let (top, bottom) = (a.y.min(b.y) - reach, a.y.max(b.y) + reach);
    // The tiles in the piece's box, less the ones its line (with the width it has) does not come near: a long piece that goes across a tile corner to
    // corner has a box much bigger than the places it reaches.
    let tiles_x = p_width.div_ceil(TILE) as usize;
    for_each_tile(left.floor() as i64, top.floor() as i64, right.ceil() as i64, bottom.ceil() as i64, p_width, p_height, |tile| {
      let (x, y) = ((tile % tiles_x) as f32 * TILE as f32, (tile / tiles_x) as f32 * TILE as f32);
      if segment_reaches_square((a.x, a.y), (b.x, b.y), (x, y, x + TILE as f32, y + TILE as f32), reach) {
        touched.push((tile, piece as u32));
      }
    });
  }
  touched.sort_unstable();
  let mut ranges: Vec<(usize, u32, u32, u32)> = Vec::new();
  for (tile, piece) in touched {
    match ranges.last_mut() {
      Some((last_tile, first, last, mask)) if *last_tile == tile => {
        *last = piece;
        if piece - *first < TRAIL_MASK_PIECES {
          *mask |= 1 << (piece - *first);
        }
      }
      _ => ranges.push((tile, piece, piece, 1)),
    }
  }
  ranges
}

/// The pass that draws the batch on an image of `p_width` x `p_height` pixels.
fn gpu_pass(p_batch: &Sprites, p_width: u32, p_height: u32) -> GpuPass {
  let (sprites, trails) = (&p_batch.sprites, &p_batch.trails);
  // Each texture is written once, however many sprites use it: its image, where its pixels start among the texture words, and its size.
  let mut textures: Vec<(Image, u32, u32, u32)> = Vec::new();
  let mut texture_words = 0u32;
  let texture_places: Vec<Option<(u32, u32, u32)>> = sprites
    .iter()
    .map(|sprite| {
      let image = sprite.texture.as_ref()?;
      if let Some((_, start, width, height)) = textures.iter().find(|(known, ..)| known.version() == image.version()) {
        return Some((*start, *width, *height));
      }
      let (width, height) = image.dimensions::<u32>();
      textures.push((image.clone(), texture_words, width, height));
      texture_words += width * height;
      Some((texture_words - width * height, width, height))
    })
    .collect();

  let vertex_words: usize = trails.iter().map(|trail| trail.points.len() * TRAIL_POINT_WORDS).sum();
  let trails_at = (sprites.len() * SPRITE_WORDS) as u32;
  let vertices_at = trails_at + (trails.len() * TRAIL_WORDS) as u32;
  let textures_at = vertices_at + vertex_words as u32;
  let tiles_at = textures_at + texture_words;
  let (tiles_x, tiles_y) = (p_width.div_ceil(TILE), p_height.div_ceil(TILE));
  let list_at = tiles_at + tiles_x * tiles_y + 1;

  let blend_code = |blend: crate::Blend| blend.shader_code().expect("a batch with a blend that has no shader is not drawn on the GPU");
  let mut words: Vec<u32> = Vec::with_capacity(list_at as usize);
  for (sprite, place) in sprites.iter().zip(&texture_places) {
    let (texture_at, texture_width, texture_height) = match place {
      Some((start, width, height)) => (textures_at + start, *width, *height),
      None => (NO_SPRITE, 0, 0),
    };
    let mut blend = blend_code(sprite.blend);
    let (birth_x, birth_y) = match sprite.birth {
      Some(born) => {
        blend |= FROM_PIXEL;
        born
      }
      None => (0.0, 0.0),
    };
    let (sin, cos) = sprite.rotation.to_radians().sin_cos();
    words.extend([
      sprite.x.to_bits(),
      sprite.y.to_bits(),
      sprite.size.to_bits(),
      sprite.opacity.to_bits(),
      (sprite.color.0 as f32).to_bits(),
      (sprite.color.1 as f32).to_bits(),
      (sprite.color.2 as f32).to_bits(),
      sin.to_bits(),
      cos.to_bits(),
      sprite.reach().clamp(0, i32::MAX as i64) as i32 as u32,
      blend,
      texture_at,
      texture_width,
      texture_height,
      birth_x.to_bits(),
      birth_y.to_bits(),
      sprite.hardness.to_bits(),
      sprite.stretch.to_bits(),
      sprite.columns,
      sprite.rows,
      sprite.frame,
    ]);
  }
  let mut first_point = 0u32;
  for trail in trails {
    let points = trail.points.len() as u32;
    words.extend([first_point, points, trail.hardness.to_bits(), blend_code(trail.blend), 0, 0, 0, 0]);
    first_point += points;
  }
  for trail in trails {
    for point in &trail.points {
      words.extend([
        point.x.to_bits(),
        point.y.to_bits(),
        (point.width / 2.0).to_bits(),
        point.opacity.to_bits(),
        (point.color.0 as f32).to_bits(),
        (point.color.1 as f32).to_bits(),
        (point.color.2 as f32).to_bits(),
        0,
      ]);
    }
  }
  for (image, ..) in &textures {
    words.extend(image.rgba().chunks_exact(4).map(|pixel| u32::from_le_bytes([pixel[0], pixel[1], pixel[2], pixel[3]])));
  }

  // The trails are drawn first, under every sprite, so they are listed first in each tile. A trail is listed in a tile as four words: the trail, the first and
  // last of its pieces that touch the tile, and a mask of which of them do, so a pixel tests only those and not the whole line. Which tiles each trail touches is
  // worked out on all the cores; the lists are then laid out by counting how many words each tile gets, so no tile has a list of its own to grow.
  let tile_count = (tiles_x * tiles_y) as usize;
  let touched: Vec<Vec<(usize, u32, u32, u32)>> = trails.par_iter().map(|trail| trail_tiles(trail, p_width, p_height)).collect();
  let mut counts = vec![0u32; tile_count];
  for ranges in &touched {
    for (tile, ..) in ranges {
      counts[*tile] += 4;
    }
  }
  for sprite in sprites {
    let (x, y) = (sprite.x as i64, sprite.y as i64);
    let reach = sprite.reach();
    for_each_tile(x.saturating_sub(reach), y.saturating_sub(reach), x.saturating_add(reach), y.saturating_add(reach), p_width, p_height, |tile| {
      counts[tile] += 1
    });
  }
  let mut offset = 0u32;
  let mut next_free: Vec<u32> = Vec::with_capacity(tile_count);
  for count in &counts {
    words.push(offset);
    next_free.push(offset);
    offset += count;
  }
  words.push(offset);
  let list_start = words.len();
  words.resize(list_start + offset as usize, 0);
  for (index, ranges) in touched.iter().enumerate() {
    for (tile, first, last, mask) in ranges {
      let at = list_start + next_free[*tile] as usize;
      words[at..at + 4].copy_from_slice(&[TRAIL_ITEM | index as u32, *first, *last, *mask]);
      next_free[*tile] += 4;
    }
  }
  for (index, sprite) in sprites.iter().enumerate() {
    let (x, y) = (sprite.x as i64, sprite.y as i64);
    let reach = sprite.reach();
    for_each_tile(x.saturating_sub(reach), y.saturating_sub(reach), x.saturating_add(reach), y.saturating_add(reach), p_width, p_height, |tile| {
      words[list_start + next_free[tile] as usize] = index as u32;
      next_free[tile] += 1;
    });
  }

  let rows = (words.len() as u32).div_ceil(AUX_WIDTH).clamp(1, AUX_ROWS);
  words.resize((rows * AUX_WIDTH) as usize, 0);
  let mut rgba: Vec<u8> = Vec::with_capacity(words.len() * 4);
  for word in &words {
    rgba.extend_from_slice(&word.to_le_bytes());
  }
  let uniforms: Vec<u8> = [AUX_WIDTH, 0, trails_at, vertices_at, tiles_at, list_at, TILE, tiles_x, 0, 0, 0, 0]
    .iter()
    .flat_map(|word| word.to_le_bytes())
    .collect();
  GpuPass::new(gpu_shader(), uniforms).with_aux(GpuAux { width: AUX_WIDTH, height: rows, rgba: Arc::from(rgba) }).with_window()
}
