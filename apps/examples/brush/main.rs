use abra::drawing::prelude::*;
use abra::prelude::*;

const FILE: &str = "assets/bikini.jpg";

pub fn main() {
  let mut image = Image::read(FILE).expect("Failed to load image");
  // let mut image = Image::new_from_color(512, 1024, Color::black());

  // Create a red brush with soft edges (hardness = 0.0)
  let red = Color::red();
  let soft_brush = Brush::new().with_size(20).with_color(&red).with_hardness(0.0);

  // Paint with soft brush at a few positions
  let mut painter = Painter::new(&mut image);
  painter.dab_brush(50.0, 50.0, &soft_brush);
  painter.dab_brush(100.0, 50.0, &soft_brush);
  painter.dab_brush(150.0, 50.0, &soft_brush);

  // Create a blue hard-edged brush (hardness = 1.0)
  let blue = Color::blue();
  let hard_brush = Brush::new().with_size(15).with_color(&blue).with_hardness(1.0);

  // Paint with hard brush at different positions
  painter.dab_brush(50.0, 100.0, &hard_brush);
  painter.dab_brush(100.0, 100.0, &hard_brush);

  // Create a green brush for path stroking
  let hue = Gradient::hue();
  let stroke_brush = Brush::new().with_size(20).with_color(&hue).with_hardness(0.0);

  // Create a path and stroke it with the brush
  let mut path = Path::new();
  path.move_to((50.0, 150.0)).line_to((200.0, 150.0)).quad_to((225.0, 175.0), (200.0, 200.0)).line_to((50.0, 200.0));

  // Stroke the path with the brush to create a continuous line

  painter.stroke_with_brush(&path, &stroke_brush);

  image.write("out/brush.png", None).expect("Failed to save image");
}
