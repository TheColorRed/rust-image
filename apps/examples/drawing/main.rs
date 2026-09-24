use abra::drawing::prelude::{Brush, Painter};
use abra::prelude::*;

pub fn main() {
  let mut butterfly_image = Image::new(3840, 2160);
  let blue = Color::from_rgba(34, 108, 196, 220);
  let orange = Color::from_rgba(224, 100, 42, 210);

  let ellipse_brush = Brush::new().with_size(32).with_color(&blue).with_hardness(1.0);
  let start = std::time::Instant::now();
  let ellipse = Path::ellipse((1920.0, 1080.0), 1200.0, 760.0, 256);
  Painter::new(&mut butterfly_image).stroke_with_brush(&ellipse, &ellipse_brush);
  println!("ellipse stroke: {:?}", start.elapsed());

  const ANIMAL_SEGMENTS: usize = 100;
  let mut path = Path::new();
  for segment in 0..=ANIMAL_SEGMENTS {
    let theta = segment as f32 / ANIMAL_SEGMENTS as f32 * std::f32::consts::TAU;
    let wing_shape = 0.45 + 0.55 * (2.0 * theta).cos().abs();
    let x = 1920.0 + theta.cos() * 780.0 * wing_shape;
    let y = 1120.0 + theta.sin() * 610.0 * (0.55 + 0.45 * (2.0 * theta).cos());
    if segment == 0 {
      path.move_to((x, y));
    } else {
      path.line_to((x, y));
    }
  }
  let stroke_brush = Brush::new().with_size(28).with_color(&orange).with_hardness(0.7);
  let start = std::time::Instant::now();
  Painter::new(&mut butterfly_image).stroke_with_brush(&path, &stroke_brush);
  println!("100-segment butterfly stroke: {:?}", start.elapsed());

  let mut fill_path = Path::new();
  fill_path.move_to((620.0, 380.0));
  fill_path.line_to((1540.0, 240.0));
  fill_path.line_to((2290.0, 530.0));
  fill_path.line_to((3140.0, 420.0));
  fill_path.line_to((3370.0, 1220.0));
  fill_path.line_to((2800.0, 1840.0));
  fill_path.line_to((1760.0, 1920.0));
  fill_path.line_to((780.0, 1580.0));
  let area: Area = fill_path.into();
  let fill_brush = Brush::new().with_size(32).with_color(&orange).with_hardness(0.6);
  let mut fill_image = Image::new(3840, 2160);
  let start = std::time::Instant::now();
  Painter::new(&mut fill_image).fill_area_with_brush(&area, &fill_brush);
  println!("brush fill: {:?}", start.elapsed());

  std::fs::create_dir_all("out").expect("failed to create output directory");
  butterfly_image.write("out/butterfly.png", None).expect("Failed to save butterfly image");
  fill_image.write("out/brush-fill.png", None).expect("Failed to save fill image");
}
