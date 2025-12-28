use abra::prelude::Image as AbraImage;
use abra::prelude::*;

use crate::common::*;

pub fn transparent_pattern(width: u32, height: u32, checker_size: u32) -> AbraImage {
  let mut image = AbraImage::new_from_color(width, height, Color::transparent());

  let light_color = Color::from_rgb(200, 200, 200);
  let dark_color = Color::from_rgb(100, 100, 100);

  for y in 0..height {
    for x in 0..width {
      let checker_x = x / checker_size;
      let checker_y = y / checker_size;
      let is_light_square = (checker_x + checker_y) % 2 == 0;
      let color = if is_light_square { light_color } else { dark_color };
      image.set_pixel(x, y, color.rgba());
    }
  }

  image
}

#[napi]
pub struct Image {
  pub(crate) image: AbraImage,
  pub width: u32,
  pub height: u32,
}

#[napi]
impl Image {
  #[napi(constructor)]
  pub fn new(data: Buffer, width: u32, height: u32) -> Self {
    let image = AbraImage::new_from_pixels(width, height, data.to_vec(), Channels::RGBA);
    image.into()
  }
}

impl From<AbraImage> for Image {
  fn from(image: AbraImage) -> Self {
    let (width, height) = image.dimensions();
    Self { image, width, height }
  }
}
