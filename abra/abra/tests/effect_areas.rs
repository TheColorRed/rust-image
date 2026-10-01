//! Every effect must give the same pixels when it is limited to an area as when it runs over the whole image and the
//! result is mixed with the original by the area's weights. That is how a live image limits an effect, and how the
//! framework's `apply_to_image` works, so the two must agree for every effect, including ones that read the pixels
//! around an area.

use abra_core::image::apply_area::{ApplyContext, area_weights, mix_by_weights, weight_to_byte};
use abra_core::image::gpu::Hardware;
use abra_core::{Area, Channels, Image};
use options::{Effect, ApplyOptions};

const SIZE: u32 = 24;

fn picture() -> Image {
  let pixels: Vec<u8> = (0..SIZE * SIZE)
    .flat_map(|i| {
      let (x, y) = (i % SIZE, i / SIZE);
      [(x * 10) as u8, (y * 10) as u8, ((x * y) % 256) as u8, 255]
    })
    .collect();
  Image::new_from_pixels(SIZE, SIZE, pixels, Channels::RGBA)
}

fn area() -> Area {
  Area::rect((5.0, 6.0), (11.0, 9.0)).with_feather(3)
}

/// The effect applied over the whole image, then mixed with the original by the area's weights, in whole numbers.
fn mixed_by_weights<E: Effect>(p_effect: &E) -> Vec<u8> {
  let original = picture().to_rgba_vec();
  let mut whole = picture();
  p_effect.apply_on_cpu(&mut whole);

  let area = area();
  let ctx = ApplyContext {
    area: Some(vec![&area]),
    mask_image: None,
    hardware: Hardware::Cpu,
  };
  let weights = area_weights(SIZE, SIZE, &ctx).into_iter().map(weight_to_byte);
  mix_by_weights(&original, &whole.to_rgba_vec(), weights)
}

fn check<E: Effect + Clone>(p_name: &str, p_effect: E) {
  let expected = mixed_by_weights(&p_effect);
  let mut image = picture();
  p_effect.with_options(ApplyOptions::new().with_area(area())).apply_on_cpu(&mut image);
  let worst = image.to_rgba_vec().iter().zip(&expected).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
  assert_eq!(worst, 0, "{p_name}: limiting to an area differs from mixing by the weights by up to {worst} levels");
}

#[test]
fn point_adjustments_agree() {
  check("brightness", adjustments::levels::brightness(40));
  check("contrast", adjustments::levels::contrast(40));
  check("saturation", adjustments::levels::saturation(40));
  check("exposure", adjustments::levels::exposure(1.0).with_offset(0.05));
  check("vibrance", adjustments::levels::vibrance(40));
  check("grayscale", adjustments::color::grayscale());
  check("invert", adjustments::color::invert());
  check("posterize", adjustments::color::posterize(4));
}

#[test]
fn filters_that_read_neighbouring_pixels_agree() {
  check("gaussian blur", filters::blur::gaussian_blur(3));
  check("box blur", filters::blur::box_blur(2));
  check("blur", filters::blur::blur());
  check("sharpen", filters::sharpen::sharpen());
  check("smooth", filters::smooth::smooth());
  check("median", filters::noise::median(2.0));
  check("median, larger", filters::noise::median(4.0));
  check("despeckle", filters::noise::despeckle(3.0, 10.0));
  check("motion blur", filters::blur::motion_blur(30.0, 9));
  check("glowing edges", filters::edges::glowing_edges().with_edge_width(3));
}

/// These depend on where a pixel is in the image, so they must run over the whole image and not on a crop of the area.
#[test]
fn effects_that_depend_on_position_agree() {
  use abra_core::{Color, Gradient};
  let gradient = Gradient::from_to(Color::from_rgba(255, 0, 0, 255), Color::from_rgba(0, 0, 255, 255));
  check("linear gradient between", adjustments::color::LinearGradientEffect::between(&gradient, (0.0, 0.0), (SIZE as f32, 0.0)));
  check("linear gradient angle", adjustments::color::LinearGradientEffect::angle(&gradient, 33.0).with_opacity(0.6));
  check("pinch", filters::distort::pinch(0.6));
  check("ripple", filters::distort::ripple(0.5));
}
