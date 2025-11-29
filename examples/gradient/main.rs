use abra::abra_core::Heart;
use abra::drawing::prelude::*;
use abra::prelude::*;

const OUT_FILE: &str = "out/gradient.png";

pub fn main() {
  let size = (255, 250);
  let mut image = Image::new(size.0, size.1);

  let gradient_color = Gradient::rainbow();
  gradient_color.with_direction(Path::line((0, 0), (0, size.1)));

  let mut fill_image = Image::new_from_path("assets/nude/gravure-idol-black-hair-chest.jpg");
  fill_image.resize_width(size.0, TransformAlgorithm::EdgeDirectNEDI);

  let area = Heart::new().fit(size);
  let filled_image = fill(area.clone(), &fill_image);

  // let filled_image = area.fill(Color::from_rgba(255, 0, 0, 255));
  let (width, height) = filled_image.dimensions::<i32>();
  // Center image
  let offset_x = (size.0 as i32 - width as i32) / 2;
  let offset_y = (size.1 as i32 - height as i32) / 2;

  image.set_from(&filled_image, (offset_x, offset_y));

  image.save(OUT_FILE, None);
}
