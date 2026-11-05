use abra_core::blend::blend as blend_images;
use abra_core::image::effect::deferred;
use abra_core::image::recipe::{gpu_ready, lay_over};
use abra_core::layers::{scale_alpha, silhouette};
use abra_core::{BlendMode, Color, Fill, Image, PointF};

use filters::Effect;
use filters::blur::gaussian_blur;
use rayon::prelude::*;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Debug)]
/// Options for configuring a drop shadow effect.
pub struct DropShadow<'a> {
  /// The color of the shadow in RGBA format.
  pub fill: Fill<'a>,
  /// The blend mode used to combine the shadow with the layer.
  pub blend_mode: BlendMode,
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

impl<'a> DropShadow<'a> {
  /// Creates a new DropShadowOptions with default settings.
  /// Default values:
  /// - distance: 5.0 pixels
  /// - angle: 45.0 degrees
  /// - blur_radius: 5.0 pixels
  /// - color: black with 60% opacity (0, 0, 0, 153)
  pub fn new() -> Self {
    DropShadow {
      fill: Fill::Solid(Color::black()),
      blend_mode: BlendMode::Normal,
      opacity: 0.35,
      angle: 45.0,
      distance: 5.0,
      spread: 0.0,
      size: 5.0,
    }
  }

  /// Sets the distance of the shadow from the object.
  pub fn with_distance(mut self, p_distance: impl Into<f64>) -> Self {
    self.distance = p_distance.into() as f32;
    self
  }

  /// Sets the angle of the shadow in degrees.
  pub fn with_angle(mut self, p_angle: impl Into<f64>) -> Self {
    self.angle = p_angle.into() as f32;
    self
  }

  /// Sets the size of the shadow blur.
  pub fn with_size(mut self, p_size: impl Into<f64>) -> Self {
    self.size = p_size.into() as f32;
    self
  }

  /// Sets the spread of the shadow between 0.0 and 1.0
  pub fn with_spread(mut self, p_spread: impl Into<f64>) -> Self {
    self.spread = p_spread.into().max(0.0).min(1.0) as f32;
    self
  }

  /// Sets the color of the shadow in RGBA format.
  pub fn with_fill(mut self, p_fill: impl Into<Fill<'a>>) -> Self {
    self.fill = p_fill.into();
    self
  }

  /// Sets the opacity of the shadow (0.0 to 1.0).
  pub fn with_opacity(mut self, p_opacity: impl Into<f64>) -> Self {
    self.opacity = p_opacity.into() as f32;
    self
  }

  /// Sets the blend mode used to combine the shadow with the layer.
  pub fn with_blend_mode(mut self, p_blend_mode: BlendMode) -> Self {
    self.blend_mode = p_blend_mode;
    self
  }
}

/// Variant of apply_drop_shadow that returns both the final image and the padding offset
/// used to position the original content within the padded image (padding_left, padding_top).
pub(crate) fn apply_drop_shadow_with_offset(p_image: Arc<Image>, p_options: &DropShadow) -> (Arc<Image>, (i32, i32)) {
  let _duration = Instant::now();

  // Skip if blur radius is 0 (no visible shadow)
  if p_options.size <= 0.0 {
    return (p_image, (0, 0));
  }

  let original_image = p_image.as_ref();
  let (width, height) = original_image.dimensions::<usize>();

  // The same steps as a chain for the GPU, with nothing copied back to the CPU in between. The spread (which grows or shrinks the shape a
  // pixel at a time) has no shader, so a shadow with one is made on the CPU.
  if gpu_ready() && p_options.spread <= 0.0 {
    let color = match &p_options.fill {
      Fill::Solid(color) => *color,
      _ => Color::black(),
    };
    let shape = silhouette(original_image, [color.r, color.g, color.b], 128);
    let (offset_x, offset_y) = shadow_offset(p_options);
    let (padding_left, padding_top, canvas_width, canvas_height) = shadow_room(p_options, (width as u32, height as u32), (offset_x, offset_y));
    let mut composite = Image::new(canvas_width, canvas_height);
    lay_over(&mut composite, &shape, (padding_left + offset_x, padding_top + offset_y), p_options.blend_mode, 1.0);
    gaussian_blur(p_options.size).apply(deferred(&mut composite));
    scale_alpha(&mut composite, p_options.opacity);
    lay_over(&mut composite, original_image, (padding_left, padding_top), BlendMode::Normal, 1.0);
    return (Arc::new(composite), (padding_left, padding_top));
  }

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
  colorize_image(&mut shadow_image, p_options.fill.clone(), p_options.opacity);

  // Apply the alpha channel to create the shadow shape
  // Write the alpha channel back into the shadow image using a mutable slice
  if let Some(shadow_pixels_mut) = shadow_image.colors().as_slice_mut() {
    for (i, &alpha) in alpha_channel.iter().enumerate() {
      if i * 4 + 3 < shadow_pixels_mut.len() {
        shadow_pixels_mut[i * 4 + 3] = alpha;
      }
    }
  }

  // Apply spread if needed (spread expands or contracts the shadow)
  if p_options.spread > 0.0 {
    apply_spread(&mut shadow_image, p_options.spread);
  }

  // Calculate offset from distance and angle
  let (offset_x, offset_y) = shadow_offset(p_options);
  let (padding_left, padding_top, canvas_width, canvas_height) = shadow_room(p_options, (width as u32, height as u32), (offset_x, offset_y));

  // Position shadow at offset
  let shadow_x = padding_left + offset_x;
  let shadow_y = padding_top + offset_y;

  // Create an expanded image to contain shadow offset
  let mut composite = Image::new(canvas_width, canvas_height);
  let empty_pixels = vec![0u8; (canvas_width * canvas_height * 4) as usize];
  composite.set_rgba(empty_pixels);

  // Composite shadow at offset position with the configured blend mode and opacity
  blend_images(&shadow_image)
    .with_offset(PointF::new(shadow_x, shadow_y))
    .with_mode(p_options.blend_mode)
    .apply(&mut composite);

  // Blur the shadow area in the composite
  gaussian_blur(p_options.size).apply(&mut composite);

  // Reapply opacity to the blurred shadow (blur operation may have increased alpha)
  if let Some(composite_pixels) = composite.colors().as_slice_mut() {
    for chunk in composite_pixels.chunks_mut(4) {
      chunk[3] = ((chunk[3] as f32) * p_options.opacity) as u8;
    }
  }

  // Composite original at padding position
  blend_images(&original_image).with_offset(PointF::new(padding_left, padding_top)).apply(&mut composite);

  // DebugEffects::DropShadow(options.clone(), duration.elapsed()).log();

  (Arc::new(composite), (padding_left, padding_top))
}

/// How far the shadow is displaced, from its distance and angle.
fn shadow_offset(p_options: &DropShadow) -> (i32, i32) {
  let angle_rad = p_options.angle.to_radians();
  ((p_options.distance * angle_rad.cos()).round() as i32, (p_options.distance * angle_rad.sin()).round() as i32)
}

/// The room the picture needs for the shadow: the padding on the left and at the top, and the size of the picture with it. A positive offset
/// moves the shadow that way, so the room is on the opposite side, and the blur needs room all round to keep its edges.
fn shadow_room(p_options: &DropShadow, p_size: (u32, u32), p_offset: (i32, i32)) -> (i32, i32, u32, u32) {
  let blur_padding = p_options.size as i32;
  let padding_left = (-p_offset.0).max(0) + blur_padding;
  let padding_top = (-p_offset.1).max(0) + blur_padding;
  let padding_right = p_offset.0.max(0) + blur_padding;
  let padding_bottom = p_offset.1.max(0) + blur_padding;
  (padding_left, padding_top, p_size.0 + padding_left as u32 + padding_right as u32, p_size.1 + padding_top as u32 + padding_bottom as u32)
}

/// Converts an image to a single color while preserving and applying opacity to the alpha channel.
fn colorize_image<'a>(p_image: &mut Image, p_fill: impl Into<Fill<'a>>, p_opacity: impl Into<f64>) {
  let pixels = p_image.rgba();

  let p_fill = p_fill.into();
  let p_opacity = p_opacity.into();
  let colorized: Vec<u8> = pixels
    .par_chunks(4)
    .flat_map_iter(|pixel| {
      let color = match &p_fill {
        Fill::Solid(c) => *c,
        _ => Color::black(),
      };
      // Preserve the alpha channel from the original, apply opacity and color's alpha
      let original_alpha = pixel[3] as f32 / 255.0;
      let shadow_alpha = (color.a as f32 / 255.0) * original_alpha * p_opacity as f32;

      vec![color.r, color.g, color.b, (shadow_alpha * 255.0) as u8]
    })
    .collect();

  p_image.set_rgba(colorized);
}

/// Applies spread to the shadow by dilating or eroding the alpha channel.
/// Spread between 0.0 and 1.0 where values > 0.5 expand and values < 0.5 contract.
fn apply_spread(p_image: &mut Image, p_spread: impl Into<f32>) {
  let p_spread = p_spread.into();
  let (width, height) = p_image.dimensions::<u32>();
  let width = width as usize;
  let height = height as usize;
  let src = p_image.rgba();
  let pixels = src.to_vec();

  // Spread > 0.5 means dilate (expand), < 0.5 means erode (contract)
  // Strength is based on distance from 0.5, clamped to reasonable values
  let strength = ((p_spread - 0.5).abs() * 2.0).ceil() as usize;

  if p_spread > 0.5 {
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
    p_image.set_rgba(result);
  } else if p_spread < 0.5 {
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
    p_image.set_rgba(result);
  }
}
