use abra::{
  Color, Image,
  canvas::Canvas,
  canvas::{DropShadow, Stroke},
};

const FILE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/layer-effects.png";

pub fn main() {
  let (width, height) = (512 + 100, 1024 + 100);
  let white_image = Image::new_from_color(width, height, Color::white());
  let canvas = Canvas::new("Layer Effects Test")
    .add_layer_from_image("White Background", white_image, None)
    .add_layer_from_path("Image", FILE, None);

  if let Some(layer) = canvas.get_layer_by_name("Image") {
    layer
      .effects()
      .with_drop_shadow(DropShadow::new().with_size(40.0))
      .with_stroke(Stroke::new().with_size(20).with_opacity(0.5));
  }

  canvas.save(OUT_FILE, None);
}
