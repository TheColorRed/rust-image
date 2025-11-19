use abra::{
  Image, Path,
  drawing::{Brush, paint_with_brush, stroke_with_brush},
  {Color, Fill, Gradient},
};

const FILE: &str = "assets/bikini.jpg";

pub fn main() {
  let mut image = Image::new_from_path(FILE);
  // let mut image = Image::new_from_color(512, 1024, Color::black());

  // Create a red brush with soft edges (hardness = 0.0)
  let soft_brush = Brush::new()
    .with_size(20)
    .with_color(Fill::Solid(Color::red()))
    .with_hardness(0.0);

  // Paint with soft brush at a few positions
  paint_with_brush(&mut image, 50.0, 50.0, &soft_brush);
  paint_with_brush(&mut image, 100.0, 50.0, &soft_brush);
  paint_with_brush(&mut image, 150.0, 50.0, &soft_brush);

  // Create a blue hard-edged brush (hardness = 1.0)
  let hard_brush = Brush::new()
    .with_size(15)
    .with_color(Fill::Solid(Color::blue()))
    .with_hardness(1.0);

  // Paint with hard brush at different positions
  paint_with_brush(&mut image, 50.0, 100.0, &hard_brush);
  paint_with_brush(&mut image, 100.0, 100.0, &hard_brush);

  // Create a green brush for path stroking
  let stroke_brush = Brush::new()
    .with_size(20)
    .with_color(Fill::Gradient(Gradient::hue()))
    .with_hardness(0.0);

  // Create a path and stroke it with the brush
  let mut path = Path::new();
  path
    .move_to((50.0, 150.0))
    .line_to((200.0, 150.0))
    .quad_to((225.0, 175.0), (200.0, 200.0))
    .line_to((50.0, 200.0));

  // Stroke the path with the brush to create a continuous line

  stroke_with_brush(&mut image, &path, &stroke_brush);

  image.save("out/brush.png", None);
}
