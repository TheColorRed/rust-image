//! The CPU's way of drawing sprites and trails. The GPU's is in `gpu.rs` and `sprites.wgsl`, which draw the same.

use super::gpu::{TILE, TRAIL_MASK_PIECES, trail_tiles};
use super::{Sprite, SpriteTrail, Sprites};
use crate::Compositor;
use crate::falloff::brush_falloff;
use abra_core::{EdgeMode, Image, Interpolation, sample};
use rayon::prelude::*;
use std::collections::HashMap;

/// Draws the trails, then the sprites, on `p_image`.
pub(super) fn draw_on_cpu(p_sprites: &Sprites, p_image: &mut Image) {
  let (width, height) = p_image.dimensions::<u32>();
  let mut pixels = p_image.to_rgba_vec();
  // The untouched pixels that a sprite takes its color from.
  let source = p_sprites.sprites.iter().any(|sprite| sprite.birth.is_some()).then(|| pixels.clone());
  let colors: Vec<(u8, u8, u8)> = p_sprites
    .sprites
    .iter()
    .map(|sprite| match (sprite.birth, &source) {
      (Some((x, y)), Some(source)) => {
        let at = (((y as i64).clamp(0, height as i64 - 1) as usize) * width as usize + (x as i64).clamp(0, width as i64 - 1) as usize) * 4;
        (source[at], source[at + 1], source[at + 2])
      }
      _ => sprite.color,
    })
    .collect();

  draw_trails(&p_sprites.trails, &mut pixels, width, height);
  draw(&p_sprites.sprites, &colors, &mut pixels, width, height);
  p_image.set_rgba(pixels);
}

/// How far a trail reaches from its line: half its widest place, and the pixel of fade at its edge.
fn trail_reach(p_trail: &SpriteTrail) -> f32 {
  p_trail.points.iter().map(|point| point.width).fold(0.0, f32::max) / 2.0 + 1.5
}

/// Draws the trails onto `p_pixels`, in the order they were given. The image is cut into bands of rows and each band is drawn by one
/// thread with the trails that touch it. A pixel tests only the pieces of a trail that reach into its tile.
fn draw_trails(p_trails: &[SpriteTrail], p_pixels: &mut [u8], p_width: u32, p_height: u32) {
  let trails: Vec<(&SpriteTrail, HashMap<usize, (u32, u32, u32)>)> = p_trails
    .iter()
    .filter(|trail| trail.points.len() >= 2)
    .map(|trail| (trail, trail_tiles(trail, p_width, p_height).into_iter().map(|(tile, first, last, mask)| (tile, (first, last, mask))).collect()))
    .collect();
  if trails.is_empty() || p_width == 0 || p_height == 0 {
    return;
  }
  let tiles_x = p_width.div_ceil(TILE) as usize;
  let rows = (p_height as usize).div_ceil(rayon::current_num_threads() * 4).max(1);
  p_pixels.par_chunks_mut(rows * p_width as usize * 4).enumerate().for_each(|(band, chunk)| {
    let first_row = band * rows;
    let last_row = (first_row + chunk.len() / (p_width as usize * 4)) as i64 - 1;
    for (trail, pieces) in &trails {
      let reach = trail_reach(trail);
      let bounds = trail.points.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |bounds, point| {
        (bounds.0.min(point.x), bounds.1.min(point.y), bounds.2.max(point.x), bounds.3.max(point.y))
      });
      let (left, right) = (((bounds.0 - reach).floor() as i64).max(0), ((bounds.2 + reach).ceil() as i64).min(p_width as i64 - 1));
      let (top, bottom) = (((bounds.1 - reach).floor() as i64).max(first_row as i64), ((bounds.3 + reach).ceil() as i64).min(last_row));
      for py in top..=bottom {
        for px in left..=right {
          let tile = (py as usize / TILE as usize) * tiles_x + px as usize / TILE as usize;
          let Some(&(first, last, mask)) = pieces.get(&tile) else {
            continue;
          };
          let Some((color, coverage)) = shade_trail(trail, first, last, mask, px as f32 + 0.5, py as f32 + 0.5) else {
            continue;
          };
          let at = ((py as usize - first_row) * p_width as usize + px as usize) * 4;
          let (r, g, b, a) = trail.blend.composite(color.0, color.1, color.2, 255, coverage, chunk[at], chunk[at + 1], chunk[at + 2], chunk[at + 3]);
          chunk[at..at + 4].copy_from_slice(&[r, g, b, a]);
        }
      }
    }
  });
}

/// The color and how much of it shows at `(p_x, p_y)` from a trail, or nothing when the trail does not reach it. The pixel finds the closest
/// place on the line among the pieces `p_first` to `p_last` that `p_mask` names (all of them when there are more than the mask holds) and reads the line's width, opacity and color there, mixed between the two points
/// either side of it.
fn shade_trail(p_trail: &SpriteTrail, p_first: u32, p_last: u32, p_mask: u32, p_x: f32, p_y: f32) -> Option<((u8, u8, u8), f32)> {
  let points = &p_trail.points;
  let (mut best, mut best_piece, mut best_along) = (f32::MAX, p_first as usize, 0.0f32);
  for piece in p_first..=p_last.min(points.len() as u32 - 2) {
    if p_last - p_first < TRAIL_MASK_PIECES && p_mask >> (piece - p_first) & 1 == 0 {
      continue;
    }
    let piece = piece as usize;
    let pair = &points[piece..piece + 2];
    let (origin, line) = ((pair[0].x, pair[0].y), (pair[1].x - pair[0].x, pair[1].y - pair[0].y));
    let length_squared = line.0 * line.0 + line.1 * line.1;
    let along = if length_squared > 0.0 { (((p_x - origin.0) * line.0 + (p_y - origin.1) * line.1) / length_squared).clamp(0.0, 1.0) } else { 0.0 };
    let (dx, dy) = (p_x - (origin.0 + line.0 * along), p_y - (origin.1 + line.1 * along));
    if dx * dx + dy * dy < best {
      (best, best_piece, best_along) = (dx * dx + dy * dy, piece, along);
    }
  }
  let (a, b) = (&points[best_piece], &points[best_piece + 1]);
  let mix = |from: f32, to: f32| from + (to - from) * best_along;
  let half_width = mix(a.width / 2.0, b.width / 2.0);
  let distance = best.sqrt();
  if half_width < 0.01 || distance >= half_width + 0.5 {
    return None;
  }
  let opacity = mix(a.opacity, b.opacity);
  let channel = |from: u8, to: u8| mix(from as f32, to as f32).round() as u8;
  let color = (channel(a.color.0, b.color.0), channel(a.color.1, b.color.1), channel(a.color.2, b.color.2));
  // The brush's own falloff, and a one pixel fade at the line's edge so a thin line is not jagged.
  let edge = (half_width + 0.5 - distance).clamp(0.0, 1.0);
  let coverage = brush_falloff(distance, half_width, p_trail.hardness) * opacity * edge;
  (coverage > 0.0).then_some((color, coverage))
}

/// Draws the sprites onto `p_pixels`, each in its color from `p_colors`. The image is cut into bands of rows, and each band is drawn by
/// one thread with the sprites that touch it, in the order they were given, so overlaps come out the same as drawing them one at a time.
fn draw(p_sprites: &[Sprite], p_colors: &[(u8, u8, u8)], p_pixels: &mut [u8], p_width: u32, p_height: u32) {
  if p_sprites.is_empty() || p_width == 0 || p_height == 0 {
    return;
  }
  let rows = (p_height as usize).div_ceil(rayon::current_num_threads() * 4).max(1);
  let mut bands: Vec<Vec<u32>> = vec![Vec::new(); (p_height as usize).div_ceil(rows)];
  for (index, sprite) in p_sprites.iter().enumerate() {
    let reach = sprite.reach();
    let top = (sprite.y as i64).saturating_sub(reach).max(0);
    let bottom = (sprite.y as i64).saturating_add(reach).min(p_height as i64 - 1);
    if top <= bottom {
      for band in top as usize / rows..=bottom as usize / rows {
        bands[band].push(index as u32);
      }
    }
  }
  p_pixels.par_chunks_mut(rows * p_width as usize * 4).zip(bands.par_iter()).enumerate().for_each(|(band, (chunk, list))| {
    for &index in list {
      draw_sprite(&p_sprites[index as usize], p_colors[index as usize], chunk, band * rows, p_width, p_height);
    }
  });
}

/// Draws a sprite in `p_color` onto the rows of the image in `p_band`, which starts at row `p_first_row`.
fn draw_sprite(p_sprite: &Sprite, p_color: (u8, u8, u8), p_band: &mut [u8], p_first_row: usize, p_width: u32, p_height: u32) {
  let texture = p_sprite.texture.as_ref().map(|texture| (texture, texture.dimensions::<u32>()));
  let (x, y, size, opacity, (r, g, b)) = (p_sprite.x, p_sprite.y, p_sprite.size, p_sprite.opacity, p_color);
  let (sin, cos) = p_sprite.rotation.to_radians().sin_cos();
  let reach = p_sprite.reach();
  let last_row = (p_first_row + p_band.len() / (p_width as usize * 4)).min(p_height as usize) as i64 - 1;
  let top = (y as i64).saturating_sub(reach).max(p_first_row as i64);
  let bottom = (y as i64).saturating_add(reach).min(last_row);
  for py in top..=bottom {
    for px in (x as i64).saturating_sub(reach).max(0)..=(x as i64).saturating_add(reach).min(p_width as i64 - 1) {
      let (dx, dy) = (px as f32 + 0.5 - x, py as f32 + 0.5 - y);
      // Turn the pixel back by the sprite's rotation to find where it falls on the sprite.
      let (u, v) = (dx * cos + dy * sin, -dx * sin + dy * cos);
      let (r, g, b, coverage) = match texture {
        None if p_sprite.stretch == 0.0 => (r, g, b, brush_falloff((dx * dx + dy * dy).sqrt(), size, p_sprite.hardness) * opacity),
        None => (r, g, b, brush_falloff((u * u / ((1.0 + p_sprite.stretch) * (1.0 + p_sprite.stretch)) + v * v).sqrt(), size, p_sprite.hardness) * opacity),
        Some((image, (texture_width, texture_height))) => {
          // One frame of a grid of them is cut out at whole pixels, and the sprite is as wide as that frame.
          let (cell_width, cell_height) = ((texture_width / p_sprite.columns).max(1), (texture_height / p_sprite.rows).max(1));
          let longest = cell_width.max(cell_height) as f32;
          let (half_width, half_height) = (size * (1.0 + p_sprite.stretch) * cell_width as f32 / longest, size * cell_height as f32 / longest);
          let (tu, tv) = (u / (2.0 * half_width) + 0.5, v / (2.0 * half_height) + 0.5);
          if !(0.0..1.0).contains(&tu) || !(0.0..1.0).contains(&tv) {
            continue;
          }
          let (left, top) = ((p_sprite.frame % p_sprite.columns) * cell_width, (p_sprite.frame / p_sprite.columns) * cell_height);
          let (x, y) = (left as f32 + tu * cell_width as f32 - 0.5, top as f32 + tv * cell_height as f32 - 0.5);
          // A frame is not mixed with the one beside it.
          let texel = sample(
            image,
            x.clamp(left as f32, (left + cell_width - 1) as f32),
            y.clamp(top as f32, (top + cell_height - 1) as f32),
            Interpolation::Bilinear,
            EdgeMode::Clamp,
          );
          let tint = |texture: u8, sprite: u8| (texture as u32 * sprite as u32 / 255) as u8;
          (tint(texel[0], r), tint(texel[1], g), tint(texel[2], b), texel[3] as f32 / 255.0 * opacity)
        }
      };
      if coverage <= 0.0 {
        continue;
      }
      let at = ((py as usize - p_first_row) * p_width as usize + px as usize) * 4;
      let (new_r, new_g, new_b, new_a) =
        p_sprite.blend.composite(r, g, b, 255, coverage, p_band[at], p_band[at + 1], p_band[at + 2], p_band[at + 3]);
      p_band[at..at + 4].copy_from_slice(&[new_r, new_g, new_b, new_a]);
    }
  }
}
