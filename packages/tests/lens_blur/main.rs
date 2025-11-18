#![allow(unused_imports)]
use abra::{
  filters::blur::{
    ApertureShape, IrisOptions, LensBlurOptions, NoiseDistribution, NoiseOptions, SpecularOptions, lens_blur,
  },
  image::Image,
};

const FILE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/lens_blur.png";

fn main() {
  let mut image = Image::new_from_path(FILE);

  let options = LensBlurOptions {
    iris: IrisOptions {
      shape: ApertureShape::Hexagon,
      radius: 15,
      blade_curvature: 30.0,
      rotation: 0.0,
    },
    noise: Some(NoiseOptions {
      amount: 0.0,
      distribution: NoiseDistribution::Gaussian,
    }),
    ..Default::default()
  };

  let start_time = std::time::Instant::now();
  lens_blur(&mut image, options);
  println!("Lens blur took: {:?}", start_time.elapsed());

  image.save(OUT_FILE, None);
}
