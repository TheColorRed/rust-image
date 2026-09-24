use abra::prelude::*;
use abra::tools::prelude::*;

const FILE: &str = "assets/nude/freckles.jpg";
// const FILE: &str = "assets/nude/freckles-blonde.jpeg";

fn main() {
  // Load image
  let mut image = Image::read(FILE).expect("Failed to read image");

  // Build the stamp once, then place it wherever it is needed.
  remover(RemoverShape::rectangle(28, 28))
    .with_position((338, 1694))
    .with_position((613, 1767))
    .with_position((250, 1388))
    .with_position((680, 868))
    .apply(&mut image);

  remover(RemoverShape::circle(5)).with_path([(894, 156), (909, 141), (918, 132), (933, 123)]).apply(&mut image);

  // remover(RemoverShape::circle(45)).with_position((630, 880)).apply(&mut image);

  image.write("out/remover.png", None).expect("Failed to write image");
}
