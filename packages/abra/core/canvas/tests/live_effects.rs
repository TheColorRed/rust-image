use abra_core::image::gpu::LiveEffect;
use abra_core::{Channels, Color, Gradient, Image};
use adjustments::levels::brightness;
use canvas::Canvas;
use adjustments::color::LinearGradientEffect;
use filters::blur::gaussian_blur;
use std::sync::Arc;

const SIZE: u32 = 48;

fn source() -> Image {
  let mut pixels = Vec::new();
  for y in 0..SIZE {
    for x in 0..SIZE {
      let value = if (x / 6 + y / 6) % 2 == 0 { 200 } else { 40 };
      pixels.extend_from_slice(&[value, (x * 4) as u8, (y * 4) as u8, 255]);
    }
  }
  Image::new_from_pixels(SIZE, SIZE, pixels, Channels::RGBA)
}

fn gradient() -> Gradient {
  Gradient::from_to(Color::from_rgba(255, 0, 0, 255), Color::from_rgba(0, 0, 255, 255))
}

fn assert_close(p_actual: &[u8], p_expected: &[u8]) {
  assert_eq!(p_actual.len(), p_expected.len());
  for (index, (a, e)) in p_actual.iter().zip(p_expected).enumerate() {
    assert!(a.abs_diff(*e) <= 3, "byte {index}: canvas {a} vs cpu {e}");
  }
}

fn effects() -> Vec<Arc<dyn LiveEffect>> {
  vec![
    Arc::new(brightness(20)),
    Arc::new(gaussian_blur(3)),
    Arc::new(LinearGradientEffect::angle(&gradient(), 30.0).with_opacity(0.6)),
  ]
}

#[test]
fn live_effects_on_a_layer_match_the_cpu_chain() {
  gpu::register();
  let canvas = Canvas::new_blank("live", SIZE, SIZE);
  let layer = canvas.add_layer_from_image("photo", source(), None);
  layer.set_live_effects(effects());

  let mut expected = source();
  for effect in effects() {
    effect.apply_cpu(&mut expected);
  }
  assert_close(canvas.as_image().rgba(), expected.rgba());
}

#[test]
fn changing_a_parameter_recomposes_without_touching_the_source() {
  gpu::register();
  let canvas = Canvas::new_blank("live", SIZE, SIZE);
  let layer = canvas.add_layer_from_image("photo", source(), None);
  let original = source().rgba().to_vec();

  layer.set_live_effects(vec![Arc::new(LinearGradientEffect::angle(&gradient(), 0.0))]);
  let first = canvas.as_image().rgba().to_vec();
  layer.set_live_effects(vec![Arc::new(LinearGradientEffect::angle(&gradient(), 90.0))]);
  let second = canvas.as_image().rgba().to_vec();

  assert_ne!(first, second);
  assert_eq!(layer.as_image().rgba(), original.as_slice());

  layer.set_live_effects(Vec::new());
  assert_eq!(canvas.as_image().rgba(), original.as_slice());
}
