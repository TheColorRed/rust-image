use crate::common::*;
use abra_core::transform::*;
use rayon::prelude::*;

#[derive(Clone)]
pub enum RippleSize {
  Small,
  Medium,
  Large,
}

#[derive(Clone)]
pub enum RippleShape {
  /// Circular ripple pattern radiating from the center.
  Circular,
  /// Square ripple pattern with maximum distance metric.
  Square,
  /// Ripple using a specific angle for displacement direction (0 = vertical, 90 = horizontal).
  Angle(f32),
  /// Random displacement: each pixel is displaced in a random direction by a random amount.
  Random,
}

impl RippleShape {
  /// Calculates the distance from the center based on the shape type.
  /// - `dx`: The horizontal distance from center.
  /// - `dy`: The vertical distance from center.
  fn calculate_distance(&self, dx: f32, dy: f32) -> f32 {
    match self {
      RippleShape::Circular => (dx * dx + dy * dy).sqrt(),
      RippleShape::Square => dx.abs().max(dy.abs()),
      RippleShape::Random => 0.0, // Random shape doesn't use distance calculation
      RippleShape::Angle(angle) => {
        // Convert angle from degrees to radians
        let angle_rad = angle.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        (dx * cos_a + dy * sin_a).abs()
      }
    }
  }
}

fn apply_ripple(p_image: &mut Image, p_amount: f32, p_size: RippleSize, p_shape: RippleShape) {
  if p_amount == 0.0 {
    return;
  }

  let (width, height) = p_image.dimensions();
  let center_x = width as f32 / 2.0;
  let center_y = height as f32 / 2.0;

  // Frequency (wavelength) is determined by size
  let frequency = match p_size {
    RippleSize::Small => 50.0,  // Wide wavelength, fewer ripples
    RippleSize::Medium => 30.0, // Medium wavelength
    RippleSize::Large => 15.0,  // Tight wavelength, many ripples
  };

  // Amplitude is controlled by amount parameter (-1.0 to 1.0)
  // Scale it to reasonable pixel displacement (e.g., up to 50 pixels)
  let amplitude = p_amount.abs() * 50.0;

  let original_image = p_image.clone();
  let mut new_pixels = vec![0u8; (width * height * 4) as usize];

  new_pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x: u32 = (i as u32) % width;
    let y: u32 = (i as u32) / width;

    let dx = x as f32 - center_x;
    let dy = y as f32 - center_y;

    let (src_x, src_y) = match p_shape {
      RippleShape::Angle(angle) => {
        // Convert angle from degrees to radians (0 = vertical, 90 = horizontal)
        let angle_rad = angle.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        // Calculate wave direction based on angle
        // For 0° (vertical): waves run vertically, displace horizontally
        // For 90° (horizontal): waves run horizontally, displace vertically
        // For other angles: interpolate between the two

        // Project position onto the wave direction
        let wave_coord = x as f32 * sin_a + y as f32 * cos_a;
        let perpendicular_coord = x as f32 * cos_a - y as f32 * sin_a;

        let phase = wave_coord * 2.0 * std::f32::consts::PI / frequency;

        // Add smooth pseudo-random variation using multiple sine waves
        let variation1 = (wave_coord * 0.05).sin() * 0.15; // Slow variation
        let variation2 = (wave_coord * 0.13).sin() * 0.08; // Medium variation

        // Apply variations to frequency and amplitude
        let local_freq_mult = 1.0 + variation1;
        let local_amp_mult = 1.0 + variation2;

        let wave = (phase / local_freq_mult).sin();
        let offset = wave * amplitude * local_amp_mult * p_amount.signum();

        // Add "whisps" - subtle perpendicular turbulence
        let whisp = (perpendicular_coord * 0.03 + wave_coord * 0.07).sin() * amplitude * 0.35;

        // Apply displacement perpendicular to wave direction
        let disp_x = offset * sin_a + whisp * cos_a * p_amount.signum();
        let disp_y = offset * cos_a - whisp * sin_a * p_amount.signum();

        let new_x = (x as f32 + disp_x).clamp(0.0, (width - 1) as f32);
        let new_y = (y as f32 + disp_y).clamp(0.0, (height - 1) as f32);
        (new_x, new_y)
      }
      RippleShape::Random => {
        // Random ripples: each pixel gets random displacement based on position hash
        let seed_x = x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663);
        let seed_y = x.wrapping_mul(83492791) ^ y.wrapping_mul(23911681);

        let rand_x = ((seed_x.wrapping_mul(2654435761_u32) % 2000) as f32 / 1000.0) - 1.0;
        let rand_y = ((seed_y.wrapping_mul(2654435761_u32) % 2000) as f32 / 1000.0) - 1.0;

        let offset_x = rand_x * amplitude * p_amount;
        let offset_y = rand_y * amplitude * p_amount;

        let new_x = (x as f32 + offset_x).clamp(0.0, (width - 1) as f32);
        let new_y = (y as f32 + offset_y).clamp(0.0, (height - 1) as f32);
        (new_x, new_y)
      }
      _ => {
        // Circular and Square: use radial displacement
        let distance = p_shape.calculate_distance(dx, dy);
        let offset = (distance / frequency).sin() * amplitude * p_amount;
        let angle = dy.atan2(dx);
        let new_x = (center_x + (distance + offset) * angle.cos()).clamp(0.0, (width - 1) as f32);
        let new_y = (center_y + (distance + offset) * angle.sin()).clamp(0.0, (height - 1) as f32);
        (new_x, new_y)
      }
    };

    // Use bicubic interpolation for smoother, higher quality results
    let pixel = sample_bicubic(&original_image, src_x, src_y);
    chunk.copy_from_slice(&pixel);
  });

  p_image.set_new_pixels(&new_pixels, width, height);
}

/// Applies a ripple distortion effect to the image.
/// - `p_image`: The image to apply the effect to.
/// - `p_amount`: The amount of ripple effect to apply. Positive values create inward ripples, negative values create outward ripples.
/// - `p_size`: The size of the ripple effect.
/// - `p_shape`: The shape of the ripple pattern (Circular, Square, Angle, or Random).
/// - `p_apply_options`: Options to specify for the filter.
pub fn ripple<'a>(
  p_image: impl Into<ImageRef<'a>>, p_amount: f32, p_size: RippleSize, p_shape: RippleShape,
  p_apply_options: impl Into<Options>,
) {
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  let p_amount = p_amount.clamp(-1.0, 1.0);
  apply_filter!(apply_ripple, image, p_apply_options, 1, p_amount, p_size.clone(), p_shape.clone());
}
