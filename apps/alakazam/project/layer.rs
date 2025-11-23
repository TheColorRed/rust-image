use abra::{
  combine::blend::{self, RGBA},
  Image,
};

pub struct Layer {
  pub name: String,
  pub image: Image,
  pub x: i32,
  pub y: i32,
  pub blend_mode: Box<dyn Fn(RGBA, RGBA) -> RGBA>,
}

impl Layer {
  pub fn new(name: String, image: Image) -> Layer {
    Layer {
      name,
      image,
      x: 0,
      y: 0,
      blend_mode: Box::new(blend::normal),
    }
  }
}
