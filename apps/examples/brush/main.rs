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

  image.write("assets/out/brush.png", None).expect("Failed to save image");

  // A finger-drawn loop: a wobbly, unevenly sampled circle like touch events give. Each point is smoothed by curving through the
  // midpoints between neighbors, then stroked with brushes of different widths and softness.
  let mut loops = Image::read("assets/cat.png").expect("Failed to load image");
  let (width, height): (u32, u32) = loops.dimensions();
  let (cx, cy) = (width as f32 / 2.0, height as f32 / 2.0);
  let radius = cx.min(cy) * 0.55;
  let touches: Vec<(f32, f32)> = (0..=60)
    .map(|i| {
      let t = i as f32 / 60.0 * std::f32::consts::TAU * 1.04;
      let wobble = 1.0 + 0.06 * (t * 3.0).sin() + 0.03 * (t * 7.0).cos();
      (cx + radius * wobble * t.cos(), cy + radius * wobble * t.sin())
    })
    .collect();
  let mut loop_path = Path::new();
  loop_path.move_to(touches[0]);
  for pair in touches.windows(2) {
    let mid = ((pair[0].0 + pair[1].0) / 2.0, (pair[0].1 + pair[1].1) / 2.0);
    loop_path.quad_to(pair[0], mid);
  }
  let white = Color::white();
  let red = Color::red();
  let mut painter = Painter::new(&mut loops);
  painter.stroke_with_brush(&loop_path, &Brush::new().with_size(24).with_color(&red).with_hardness(0.0).with_opacity(0.5));
  painter.stroke_with_brush(&loop_path, &Brush::new().with_size(6).with_color(&white).with_hardness(1.0));
  loops.write("assets/out/brush-loop.png", None).expect("Failed to save image");
}
