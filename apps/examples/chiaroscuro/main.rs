use abra::adjustments::prelude::Effect;
use abra::adjustments::prelude::levels::{contrast, exposure, highlights_shadows};
use abra::filters::prelude::blur::gaussian_blur;
use abra::abra_core::blend::blend;
use abra::prelude::*;
use std::time::Instant;

/// Tone width and shadow radius fraction used by the Alakazam Chiaroscuro effect at amount 0.5.
const TONE: f32 = 0.8;
const RADIUS_FRACTION: f32 = 0.05;
/// Color smoothing blur as a fraction of width, matching COLOR_SMOOTH in the app.
const COLOR_SMOOTH: f32 = 0.01;

fn srgb_to_linear(value: f32) -> f32 {
  if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
}

fn linear_to_srgb(value: f32) -> f32 {
  if value <= 0.0031308 { value * 12.92 } else { 1.055 * value.powf(1.0 / 2.4) - 0.055 }
}

/// End-weighted falloff, the same as Abra's shadow and highlight weights.
fn weight(value: f32, tone: f32) -> f32 {
  let across = (value / tone).clamp(0.0, 1.0);
  1.0 - across * across * (3.0 - 2.0 * across)
}

/// The same tone curve as Abra's highlights and shadows adjustment.
fn tone(value: f32, highlights: f32, shadows: f32) -> f32 {
  let inverse = 1.0 - value;
  value + 0.9 * value * inverse * (shadows * inverse * inverse - highlights * value * value)
}

/// Separable Gaussian blur in f32 with clamped edges, without 8-bit rounding.
fn blur(values: &[f32], width: usize, height: usize, radius: usize) -> Vec<f32> {
  if radius == 0 {
    return values.to_vec();
  }
  let sigma = radius as f64 / 2.0;
  let raw: Vec<f64> = (-(radius as i64)..=radius as i64)
    .map(|k| (-((k * k) as f64) / (2.0 * sigma * sigma)).exp())
    .collect();
  let total: f64 = raw.iter().sum();
  let weights: Vec<f32> = raw.iter().map(|w| (w / total) as f32).collect();
  let r = radius as i64;

  let mut horizontal = vec![0.0f32; values.len()];
  for y in 0..height {
    for x in 0..width {
      let mut sum = 0.0;
      for (i, w) in weights.iter().enumerate() {
        let nx = (x as i64 + i as i64 - r).clamp(0, width as i64 - 1) as usize;
        sum += values[y * width + nx] * w;
      }
      horizontal[y * width + x] = sum;
    }
  }
  let mut out = vec![0.0f32; values.len()];
  for y in 0..height {
    for x in 0..width {
      let mut sum = 0.0;
      for (i, w) in weights.iter().enumerate() {
        let ny = (y as i64 + i as i64 - r).clamp(0, height as i64 - 1) as usize;
        sum += horizontal[ny * width + x] * w;
      }
      out[y * width + x] = sum;
    }
  }
  out
}

/// The shadow pass computed in f32 with the same gain limit. The blurred context is not quantized to 8 bits.
fn reference(photo: &Image, width: usize, height: usize, radius: usize) -> Vec<u8> {
  let pixels = photo.rgba();
  let decode: [f32; 256] = std::array::from_fn(|i| srgb_to_linear(i as f32 / 255.0));
  let count = width * height;
  let mut luminance = vec![0.0f32; count];
  let mut value = vec![0.0f32; count];
  for i in 0..count {
    let p = &pixels[i * 4..i * 4 + 4];
    let lum = 0.2126 * decode[p[0] as usize] + 0.7152 * decode[p[1] as usize] + 0.0722 * decode[p[2] as usize];
    luminance[i] = lum;
    value[i] = linear_to_srgb(lum);
  }
  let context = blur(&value, width, height, radius);

  let mut out = pixels.to_vec();
  for i in 0..count {
    if luminance[i] == 0.0 {
      continue;
    }
    let p = &pixels[i * 4..i * 4 + 4];
    let rgb = [decode[p[0] as usize], decode[p[1] as usize], decode[p[2] as usize]];
    let shadow_weight = weight(value[i], TONE).min(weight(context[i], TONE));
    let highlight_weight = weight(1.0 - value[i], TONE).min(weight(1.0 - context[i], TONE));
    let adjusted = tone(value[i], 0.1 * highlight_weight, -0.5 * shadow_weight).clamp(0.0, 1.0);
    let target = srgb_to_linear(adjusted);
    let gain = (target / luminance[i]).min(1.0 / rgb[0].max(rgb[1]).max(rgb[2]));
    for channel in 0..3 {
      out[i * 4 + channel] = (linear_to_srgb(rgb[channel] * gain) * 255.0).round().clamp(0.0, 255.0) as u8;
    }
  }
  out
}

/// Runs the app's Chiaroscuro steps at amount 0.5 and prints the time for each one.
fn run_app_pipeline(photo: &Image, radius: u32, write_outputs: bool) {
  let mut timings: Vec<(&str, f64)> = Vec::new();
  let total = Instant::now();

  let mut toned = photo.clone();
  let step = Instant::now();
  contrast(30).apply(&mut toned);
  let _ = toned.rgba();
  timings.push(("contrast", step.elapsed().as_secs_f64() * 1000.0));
  if write_outputs {
    toned.write("assets/out/trace-1-contrast.png", None).expect("Failed to write image");
  }

  let step = Instant::now();
  exposure(-0.25).with_gamma(1.25).apply(&mut toned);
  let _ = toned.rgba();
  timings.push(("exposure", step.elapsed().as_secs_f64() * 1000.0));
  if write_outputs {
    toned.write("assets/out/trace-2-exposure.png", None).expect("Failed to write image");
  }

  let step = Instant::now();
  highlights_shadows(10, -50).with_tone(TONE).with_radius(radius).apply(&mut toned);
  let _ = toned.rgba();
  timings.push(("shadows", step.elapsed().as_secs_f64() * 1000.0));

  if write_outputs {
    toned.write("assets/out/chiaroscuro-app-toned.png", None).expect("Failed to write image");
  }

  let step = Instant::now();
  let mut smooth_color = photo.clone();
  gaussian_blur((photo.dimensions::<usize>().0 as f32 * COLOR_SMOOTH).max(2.0)).apply(&mut smooth_color);
  blend(&toned).with_mode(BlendMode::Luminosity).apply(&mut smooth_color);
  let mut result = photo.clone();
  blend(&smooth_color).apply(&mut result);
  let _ = result.rgba();
  timings.push(("luminosity blend", step.elapsed().as_secs_f64() * 1000.0));

  if write_outputs {
    result.write("assets/out/chiaroscuro-app.png", None).expect("Failed to write image");
  }

  println!("app pipeline steps (ms):");
  for (name, ms) in &timings {
    println!("  {name:<18}{ms:>9.1}");
  }
  println!("  {:<18}{:>9.1}", "total", total.elapsed().as_secs_f64() * 1000.0);
}

fn main() {
  let photo = Image::read("assets/aletta-ocean.jpg").expect("Failed to read image");
  let (width, height) = photo.dimensions::<usize>();
  let radius = (width as f32 * RADIUS_FRACTION).round() as usize;
  let count = width * height;

  let mut abra_out = photo.clone();
  highlights_shadows(10, -50).with_tone(TONE).with_radius(radius as u32).apply(&mut abra_out);
  abra_out.write("assets/out/chiaroscuro-abra.png", None).expect("Failed to write image");
  let abra_pixels = abra_out.rgba().to_vec();

  let reference_pixels = reference(&photo, width, height, radius);
  let mut reference_out = photo.clone();
  reference_out.set_rgba(reference_pixels.clone());
  reference_out.write("assets/out/chiaroscuro-f32.png", None).expect("Failed to write image");

  let source = photo.rgba();
  let mut diff_image = vec![255u8; count * 4];
  let mut max_diff = 0i32;
  let mut total_diff = 0u64;
  let mut dark_max = 0i32;
  let mut dark_total = 0u64;
  let mut dark_count = 0u64;
  for i in 0..count {
    let is_dark = source[i * 4].max(source[i * 4 + 1]).max(source[i * 4 + 2]) < 64;
    let mut pixel_diff = 0i32;
    for channel in 0..3 {
      let d = (abra_pixels[i * 4 + channel] as i32 - reference_pixels[i * 4 + channel] as i32).abs();
      pixel_diff = pixel_diff.max(d);
      total_diff += d as u64;
      if is_dark {
        dark_total += d as u64;
        dark_max = dark_max.max(d);
      }
    }
    if is_dark {
      dark_count += 1;
    }
    max_diff = max_diff.max(pixel_diff);
    let shade = (pixel_diff * 32).min(255) as u8;
    diff_image[i * 4..i * 4 + 3].copy_from_slice(&[shade, shade, shade]);
  }
  let mut diff_out = photo.clone();
  diff_out.set_rgba(diff_image);
  diff_out.write("assets/out/chiaroscuro-diff-x32.png", None).expect("Failed to write image");

  println!("image {width}x{height}, shadow radius {radius}px");
  println!("all pixels: max diff {max_diff}/255, mean diff {:.3}", total_diff as f64 / (count as f64 * 3.0));
  println!(
    "dark pixels ({dark_count}): max diff {dark_max}/255, mean diff {:.3}",
    dark_total as f64 / (dark_count as f64 * 3.0).max(1.0)
  );

  run_app_pipeline(&photo, radius as u32, false);
  println!("second run (warm):");
  run_app_pipeline(&photo, radius as u32, true);
}
