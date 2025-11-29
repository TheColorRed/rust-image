use primitives::Channels;
use primitives::Color;
use primitives::Image;

#[test]
fn create_and_set_pixels() {
  let mut img = Image::new(4u32, 4u32);
  img.set_pixel(1, 2, (12u8, 34u8, 56u8, 255u8));
  let p = img.get_pixel(1, 2).unwrap();
  assert_eq!(p, (12u8, 34u8, 56u8, 255u8));
}

#[test]
fn new_from_color_test() {
  let img = Image::new_from_color(2, 3, Color::from_rgba(1, 2, 3, 255));
  let p = img.get_pixel(1, 1).unwrap();
  assert_eq!(p, (1u8, 2u8, 3u8, 255u8));
}

#[test]
fn rgba_vec_roundtrip() {
  let img = Image::new_from_pixels(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8], Channels::RGBA);
  let v = img.to_rgba_vec();
  assert_eq!(v.len(), 8);
}
