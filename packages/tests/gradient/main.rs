use abra::{
  color::{Color, Gradient},
  draw::gradient::linear_gradient,
  geometry::path::Path,
  image::Image,
};

const IN_FILE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/gradient.png";

pub fn main() {
  // let (width, height) = image.dimensions();
  let mut image = Image::new(30, 250);
  // let mut image = abra::image::Image::new_from_path(IN_FILE);

  let gradient = Gradient::to_black(Color::green());
  // let path: Path = Path::new(vec![(0, 0), (0, 250)]);
  let path: Path = Path::line(0, 0, 0, 250);

  let start_time = std::time::Instant::now();
  // linear_gradient(&mut image, path, gradient);
  linear_gradient(&mut image, path, gradient);
  println!("Gradient took: {:?}", start_time.elapsed());

  image.save(OUT_FILE, None);

  // let mut image = Image::new(U_SIZE, U_SIZE);
  // gradient::radial_gradient(&mut image, 100, Gradient::rainbow());
  // image.save(FILE, None);

  // let mut left_right_image = Image::new(U_SIZE, U_SIZE);
  // let mut top_bottom_image = Image::new(U_SIZE, U_SIZE);

  // let left_right_colors = Gradient::from_to(Color::white(), Color::blue());
  // let top_bottom_colors = Gradient::from_to(Color::transparent(), Color::black());

  // let left_right_path = Path::new(vec![(0, 0), (SIZE, 0)]);
  // let top_bottom_path = Path::new(vec![(0, 0), (0, SIZE)]);

  // gradient::linear_gradient(&mut left_right_image, left_right_path, left_right_colors);
  // gradient::linear_gradient(&mut top_bottom_image, top_bottom_path, top_bottom_colors);

  // blend::blend_images(&mut left_right_image, &top_bottom_image, blend::normal);

  // left_right_image.save(FILE, None);
}
