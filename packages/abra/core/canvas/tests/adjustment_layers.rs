use abra_core::image::gpu::LiveEffect;
use abra_core::{Channels, Image};
use adjustments::levels::brightness;
use canvas::{AdjustmentLayerType, Canvas};
use std::sync::Arc;

const SIZE: u32 = 8;

fn solid(p_rgba: [u8; 4]) -> Image {
  let pixels = p_rgba.repeat((SIZE * SIZE) as usize);
  Image::new_from_pixels(SIZE, SIZE, pixels, Channels::RGBA)
}

#[test]
fn an_adjustment_layer_changes_only_what_is_under_it() {
  let canvas = Canvas::new_blank("adjust", SIZE, SIZE);
  canvas.add_layer_from_image("base", solid([100, 100, 100, 255]), None);
  let adjustment = canvas.add_adjustment_layer("lighten", AdjustmentLayerType::BrightnessContrast);
  adjustment.set_live_effects(vec![Arc::new(brightness(40))]);
  canvas.add_layer_from_image("above", solid([10, 20, 30, 255]), None);

  // The layer above covers everything, so it must be untouched by the adjustment.
  assert_eq!(&canvas.as_image().rgba()[..4], &[10, 20, 30, 255]);

  // Hide the top layer: the base shows, lightened.
  let lightened = {
    let mut expected = solid([100, 100, 100, 255]);
    brightness(40).apply_cpu(&mut expected);
    expected.rgba()[..4].to_vec()
  };
  canvas.layers().last().unwrap().set_visible(false);
  assert_eq!(&canvas.as_image().rgba()[..4], lightened.as_slice());
}

fn lightened_canvas() -> (Canvas<'static>, canvas::Layer<'static>) {
  let canvas = Canvas::new_blank("adjust", SIZE, SIZE);
  canvas.add_layer_from_image("base", solid([100, 100, 100, 255]), None);
  let adjustment = canvas.add_adjustment_layer("lighten", AdjustmentLayerType::BrightnessContrast);
  adjustment.set_live_effects(vec![Arc::new(brightness(40))]);
  (canvas, adjustment)
}

#[test]
fn a_mask_limits_where_the_adjustment_applies() {
  let (canvas, adjustment) = lightened_canvas();
  let mut values = vec![0u8; (SIZE * SIZE) as usize];
  for y in 0..SIZE {
    for x in 0..SIZE / 2 {
      values[(y * SIZE + x) as usize] = 255;
    }
  }
  adjustment.set_mask(Some(mask::Mask::from_values(SIZE, SIZE, values)));

  let image = canvas.as_image();
  let rgba = image.rgba();
  assert_eq!(&rgba[..4], &[140, 140, 140, 255]);
  let right = ((SIZE - 1) * 4) as usize;
  assert_eq!(&rgba[right..right + 4], &[100, 100, 100, 255]);
}

#[test]
fn opacity_blends_the_adjustment_in() {
  let (canvas, adjustment) = lightened_canvas();
  adjustment.set_opacity(0.5);
  assert_eq!(&canvas.as_image().rgba()[..4], &[120, 120, 120, 255]);
}
