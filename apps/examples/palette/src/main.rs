use abra::prelude::*;
use abra::tools::prelude::*;

const SQUARE: u32 = 100;
const COLUMNS: usize = 8;

fn main() {
  let palettes = [
    ("comic", Palette::comic()),
    ("silk_screen", Palette::silk_screen()),
    ("neon", Palette::neon()),
    ("pastel", Palette::pastel()),
    ("dusk", Palette::dusk()),
    ("cga", Palette::cga()),
    ("game_boy", Palette::game_boy()),
    ("sepia", Palette::sepia()),
  ];

  for (name, palette) in palettes {
    // One 100x100 square for each color of the palette, laid out left to right, top to bottom.
    let squares = palette.colors().iter().map(|color| Image::new_from_color(SQUARE, SQUARE, *color)).collect();
    let image: Image = atlas_with(squares).with_cols(COLUMNS).into();

    image.write(&format!("assets/out/palette-{name}.png"), None).expect("Failed to write image");
  }
}
