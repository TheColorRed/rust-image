use abra::prelude::*;
use abra::tools::prelude::*;

const FILE: &str = "assets/architecture/central-chambers-angled.jpg";

fn main() {
  let image = Image::read(FILE).expect("Failed to read image");

  // The corners of the facade: the tops of the outer pilasters at the eaves, and the feet of the outer piers.
  let mut area = Area::new();
  area
    .move_to(PointF::new(518, 1000))
    .line_to(PointF::new(2900, 450))
    .line_to(PointF::new(3105, 2540))
    .line_to(PointF::new(250, 2520));

  let mut whole = image.clone();
  perspective(area.clone()).with_crop(false).apply(&mut whole);
  whole.write("out/perspective.png", None).expect("Failed to write image");

  let mut cropped = image;
  perspective(area).with_crop(true).apply(&mut cropped);
  cropped.write("out/perspective-cropped.png", None).expect("Failed to write image");
}
