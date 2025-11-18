use crate::color::{Color, Fill};
use crate::combine::blend::{RGBA, normal};
use crate::{
  combine::blend::{self, blend_images_at_with_opacity},
  filters::blur::gaussian_blur,
  image::Image,
  utils::debug::DebugEffects,
};

use rayon::prelude::*;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Debug)]
/// Options for configuring a drop shadow effect.
pub struct DropShadow {
  /// The color of the shadow in RGBA format.
  pub fill: Fill,
  /// The blend mode used to combine the shadow with the layer.
  pub blend_mode: fn(RGBA, RGBA) -> RGBA,
  /// The opacity of the shadow (0.0 to 1.0).
  pub opacity: f32,
  /// The angle of the shadow in degrees.
  pub angle: f32,
  /// The distance of the shadow from the object.
  pub distance: f32,
  /// The spread of the shadow between 0.0 and 1.0
  pub spread: f32,
  /// The blur radius of the shadow.
  pub size: f32,
}

impl DropShadow {
  /// Creates a new DropShadowOptions with default settings.
  /// Default values:
  /// - distance: 5.0 pixels
  /// - angle: 45.0 degrees
  /// - blur_radius: 5.0 pixels
  /// - color: black with 60% opacity (0, 0, 0, 153)
  pub fn new() -> Self {
    DropShadow {
      fill: Fill::Solid(Color::black()),
      blend_mode: normal,
      opacity: 0.35,
      angle: 45.0,
      distance: 5.0,
      spread: 0.0,
      size: 5.0,
    }
  }

  /// Sets the distance of the shadow from the object.
  pub fn with_distance(mut self, distance: f32) -> Self {
    self.distance = distance;
    self
  }

  /// Sets the angle of the shadow in degrees.
  pub fn with_angle(mut self, angle: f32) -> Self {
    self.angle = angle;
    self
  }

  /// Sets the size of the shadow blur.
  pub fn with_size(mut self, size: f32) -> Self {
    self.size = size;
    self
  }

  /// Sets the spread of the shadow between 0.0 and 1.0
  pub fn with_spread(mut self, spread: f32) -> Self {
    self.spread = spread.max(0.0).min(1.0);
    self
  }

  /// Sets the color of the shadow in RGBA format.
  pub fn with_fill(mut self, fill: Fill) -> Self {
    self.fill = fill;
    self
  }

  /// Sets the opacity of the shadow (0.0 to 1.0).
  pub fn with_opacity(mut self, opacity: f32) -> Self {
    self.opacity = opacity;
    self
  }

  /// Sets the blend mode used to combine the shadow with the layer.
  pub fn with_blend_mode(mut self, blend_mode: fn(RGBA, RGBA) -> RGBA) -> Self {
    self.blend_mode = blend_mode;
    self
  }
}

/// Applies a drop shadow effect to an image by creating a composition with shadow and blur.
///
/// Creates a new image with:
/// 1. A shadow layer (colorized, blurred, and offset)
/// 2. The original layer on top
///
/// Returns the composed image with the shadow effect applied.
///
/// # Arguments
/// * `image` - The image to apply the drop shadow to
/// * `options` - Configuration options for the drop shadow effect
pub(crate) fn apply_drop_shadow(image: Arc<Image>, options: &DropShadow) -> Arc<Image> {
  let duration = Instant::now();

  // Skip if blur radius is 0 (no visible shadow)
  if options.size <= 0.0 {
    return image;
  }

  let original_image = image.as_ref();
  let (width, height) = original_image.dimensions::<usize>();

  // Create shadow by copying the original image
  let mut shadow_image = original_image.clone();

  // Extract alpha channel from original (or create one if the image has no alpha)
  // This will be used to create the shadow shape
  let shadow_pixels = shadow_image.rgba();
  let alpha_channel: Vec<u8> = shadow_pixels
    .chunks(4)
    .map(|pixel| {
      let alpha = pixel[3];
      // If alpha is mostly opaque, set it to fully opaque for the shadow mask
      if alpha > 128 { 255 } else { alpha }
    })
    .collect();

  // Colorize to the shadow color with opacity applied
  colorize_image(&mut shadow_image, options.fill.clone(), options.opacity);

  // Apply the alpha channel to create the shadow shape
  let mut shadow_pixels = shadow_image.rgba();
  for (i, &alpha) in alpha_channel.iter().enumerate() {
    if i * 4 + 3 < shadow_pixels.len() {
      shadow_pixels[i * 4 + 3] = alpha;
    }
  }
  shadow_image.set_rgba(shadow_pixels);

  // Apply spread if needed (spread expands or contracts the shadow)
  if options.spread > 0.0 {
    apply_spread(&mut shadow_image, options.spread);
  }

  // Calculate offset from distance and angle
  let angle_rad = options.angle.to_radians();
  let offset_x = (options.distance * angle_rad.cos()).round() as i32;
  let offset_y = (options.distance * angle_rad.sin()).round() as i32;

  // Determine padding needed for the expanded canvas
  // Positive offset means shadow is displaced in that direction, so we need padding on the opposite side
  // Also add padding for the blur radius to prevent blur artifacts at the edges
  let blur_padding = options.size as i32;
  let padding_left = (-offset_x).max(0) + blur_padding;
  let padding_top = (-offset_y).max(0) + blur_padding;
  let padding_right = offset_x.max(0) + blur_padding;
  let padding_bottom = offset_y.max(0) + blur_padding;

  // Create an expanded canvas to contain shadow offset
  let canvas_width = width as u32 + padding_left as u32 + padding_right as u32;
  let canvas_height = height as u32 + padding_top as u32 + padding_bottom as u32;

  // Position shadow at offset
  let shadow_x = padding_left + offset_x;
  let shadow_y = padding_top + offset_y;

  // Create an expanded image to contain shadow offset
  let mut composite = Image::new(canvas_width, canvas_height);
  let empty_pixels = vec![0u8; (canvas_width * canvas_height * 4) as usize];
  composite.set_rgba(empty_pixels);

  // Composite shadow at offset position with the configured blend mode and opacity
  blend_images_at_with_opacity(&mut composite, &shadow_image, 0, 0, shadow_x, shadow_y, options.blend_mode, 1.0);

  // Blur the shadow area in the composite
  gaussian_blur(&mut composite, options.size as u32);

  // Reapply opacity to the blurred shadow (blur operation may have increased alpha)
  let mut composite_pixels = composite.rgba();
  for chunk in composite_pixels.chunks_mut(4) {
    chunk[3] = ((chunk[3] as f32) * options.opacity) as u8;
  }
  composite.set_rgba(composite_pixels);

  // Composite original at padding position
  blend_images_at_with_opacity(&mut composite, &original_image, 0, 0, padding_left, padding_top, blend::normal, 1.0);

  DebugEffects::DropShadow(options.clone(), duration.elapsed()).log();

  Arc::new(composite)
}

/// Converts an image to a single color while preserving and applying opacity to the alpha channel.
fn colorize_image(image: &mut crate::Image, fill: Fill, opacity: f32) {
  let pixels = image.rgba();

  let colorized: Vec<u8> = pixels
    .par_chunks(4)
    .flat_map_iter(|pixel| {
      let color = match &fill {
        Fill::Solid(c) => c,
        _ => &Color::black(),
      };
      // Preserve the alpha channel from the original, apply opacity and color's alpha
      let original_alpha = pixel[3] as f32 / 255.0;
      let shadow_alpha = (color.a as f32 / 255.0) * original_alpha * opacity;

      vec![color.r, color.g, color.b, (shadow_alpha * 255.0) as u8]
    })
    .collect();

  image.set_rgba(colorized);
}

/// Applies spread to the shadow by dilating or eroding the alpha channel.
/// Spread between 0.0 and 1.0 where values > 0.5 expand and values < 0.5 contract.
fn apply_spread(image: &mut crate::Image, spread: f32) {
  let (width, height) = image.dimensions::<u32>();
  let width = width as usize;
  let height = height as usize;
  let pixels = image.rgba().to_vec();

  // Spread > 0.5 means dilate (expand), < 0.5 means erode (contract)
  // Strength is based on distance from 0.5, clamped to reasonable values
  let strength = ((spread - 0.5).abs() * 2.0).ceil() as usize;

  if spread > 0.5 {
    // Dilate: expand opaque regions
    let mut result = pixels.clone();
    for _ in 0..strength {
      let current = result.clone();
      for y in 0..height {
        for x in 0..width {
          let idx = (y * width + x) * 4;
          let current_alpha = current[idx + 3];

          // Check neighbors and dilate if any neighbor is more opaque
          if current_alpha < 255 {
            let mut max_alpha = current_alpha;
            for dy in -1..=1 {
              for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                  continue;
                }
                let nx = (x as i32 + dx).clamp(0, width as i32 - 1) as usize;
                let ny = (y as i32 + dy).clamp(0, height as i32 - 1) as usize;
                let n_idx = (ny * width + nx) * 4;
                max_alpha = max_alpha.max(current[n_idx + 3]);
              }
            }
            result[idx + 3] = max_alpha;
          }
        }
      }
    }
    image.set_rgba(result);
  } else if spread < 0.5 {
    // Erode: contract opaque regions
    let mut result = pixels.clone();
    for _ in 0..strength {
      let current = result.clone();
      for y in 0..height {
        for x in 0..width {
          let idx = (y * width + x) * 4;
          let current_alpha = current[idx + 3];

          // Check neighbors and erode if any neighbor is less opaque
          if current_alpha > 0 {
            let mut min_alpha = current_alpha;
            for dy in -1..=1 {
              for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                  continue;
                }
                let nx = (x as i32 + dx).clamp(0, width as i32 - 1) as usize;
                let ny = (y as i32 + dy).clamp(0, height as i32 - 1) as usize;
                let n_idx = (ny * width + nx) * 4;
                min_alpha = min_alpha.min(current[n_idx + 3]);
              }
            }
            result[idx + 3] = min_alpha;
          }
        }
      }
    }
    image.set_rgba(result);
  }
}
