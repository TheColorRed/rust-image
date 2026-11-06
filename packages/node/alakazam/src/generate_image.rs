use abra::prelude::Image as AbraImage;
use abra::prelude::*;

use crate::common::*;

pub fn transparent_pattern(width: u32, height: u32, checker_size: u32) -> AbraImage {
  abra::drawing::prelude::fill(Area::rect((0.0, 0.0), (width as f32, height as f32)), Fill::checkerboard(checker_size, (200, 200, 200), (100, 100, 100))).to_image()
}

#[napi]
pub struct Image {
  #[allow(dead_code)]
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
