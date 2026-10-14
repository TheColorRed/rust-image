//! The effects that look at the pixels around the one they change give the same picture when only a part of it is asked for, at
//! any zoom: the part is what the whole picture has there (see `Effect::for_level`).

use abra_core::image::image_ext::ImageExt;
use abra_core::{Channels, Image};
use options::Effect;

use crate::blur::box_blur;
use crate::noise::{despeckle, median};
use crate::sharpen::sharpen;
use crate::smooth::smooth;

/// A `p_width` x `p_height` image with detail in every channel, so an effect that read the wrong pixels shows.
fn busy_image(p_width: u32, p_height: u32) -> Image {
  let mut pixels = Vec::new();
  for y in 0..p_height {
    for x in 0..p_width {
      pixels.extend_from_slice(&[(x * 37 + y * 11) as u8, (x * 5 ^ y * 29) as u8, (x * y) as u8, 255]);
    }
  }
  Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA)
}

/// `p_image` with `p_effect` put on it to be made later.
fn deferred_of(p_effect: impl Effect + Clone + 'static, p_image: &Image) -> Image {
  let mut image = p_image.clone();
  p_effect.apply(options::deferred(&mut image));
  image
}

/// Every effect here with the name it is told by.
fn effects() -> Vec<(&'static str, Box<dyn Fn(&mut Image)>, Box<dyn Fn(&mut Image)>)> {
  // Each is the effect as an ordinary one, and the same effect as a deferred one.
  fn both<E: Effect + Clone + 'static>(p_name: &'static str, p_effect: E) -> (&'static str, Box<dyn Fn(&mut Image)>, Box<dyn Fn(&mut Image)>) {
    let now = p_effect.clone();
    (p_name, Box::new(move |image| now.apply_on_cpu(image)), Box::new(move |image| p_effect.apply(options::deferred(image))))
  }
  vec![
    both("box blur", box_blur(4)),
    both("median", median(2.0)),
    both("despeckle", despeckle(2.0, 13.0)),
    both("sharpen", sharpen()),
    both("smooth", smooth()),
  ]
}

#[test]
fn a_part_of_a_deferred_effect_is_the_same_as_the_effect_on_the_whole_there_even_at_the_edges() {
  let image = busy_image(64, 48);
  for (name, now, later) in effects() {
    let mut whole = image.clone();
    now(&mut whole);
    let mut deferred = image.clone();
    later(&mut deferred);
    assert!(deferred.deferred().is_some(), "{name} is made later");
    for (left, top, width, height) in [(20, 10, 16, 12), (0, 0, 9, 9), (55, 40, 9, 8), (-3, 20, 8, 6), (60, 44, 10, 10)] {
      assert_eq!(
        deferred.region(0, left, top, width, height),
        whole.region(0, left, top, width, height),
        "{name}: the part at {left},{top} {width}x{height}"
      );
    }
    assert!(deferred.deferred().is_some(), "{name}: only the parts were made, not the picture");
  }
}

#[test]
fn a_zoomed_out_part_of_a_deferred_effect_is_the_effect_worked_out_for_the_zoomed_out_picture() {
  let image = busy_image(64, 48);
  // A picture a quarter as wide has an effect a quarter as wide.
  let for_quarter: Vec<(&str, Box<dyn Fn(&mut Image)>)> = vec![
    ("box blur", Box::new(|image| box_blur(1).apply_on_cpu(image))),
    ("median", Box::new(|image| median(1.0).apply_on_cpu(image))),
    ("despeckle", Box::new(|image| despeckle(1.0, 13.0).apply_on_cpu(image))),
    ("sharpen", Box::new(|image| sharpen().apply_on_cpu(image))),
    ("smooth", Box::new(|image| smooth().apply_on_cpu(image))),
  ];
  let wide = [
    deferred_of(box_blur(4), &image),
    deferred_of(median(4.0), &image),
    deferred_of(despeckle(4.0, 13.0), &image),
    deferred_of(sharpen(), &image),
    deferred_of(smooth(), &image),
  ];
  for ((name, quarter), deferred) in for_quarter.iter().zip(wide) {
    let mut expected = Image::new_from_pixels(16, 12, image.region(2, 0, 0, 16, 12), Channels::RGBA);
    quarter(&mut expected);
    assert_eq!(deferred.region(2, 3, 2, 8, 6), expected.region(0, 3, 2, 8, 6), "{name}");
  }
}
