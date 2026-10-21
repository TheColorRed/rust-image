mod parts;

use abra::abra_core::blend::blend;
use abra::abra_core::{Area, BlendMode, Fill};
use abra::adjustments::prelude::{Effect, color, levels};
use abra::drawing::prelude::fill;
use abra::filters::prelude::{blur, edges::color_edges, halftone::halftone, noise};
use abra::mask::prelude::Mask;
use abra::prelude::*;
use abra::tools::prelude::*;
use abra::transform::prelude::{ResizeTarget, resize};
use std::sync::Arc;

const FILE: &str = "assets/aletta-ocean.jpg";
const OUT_FILE: &str = "assets/out/quantize.png";

/// How different two neighboring colors may be and still be smoothed together.
const SMOOTH_LIMIT: u8 = 30;

/// The lightness, in the Lab color space, of the skin under the dots, and of the dots over it.
const SKIN_LIGHTNESS: f32 = 78.0;
const DOT_LIGHTNESS: f32 = 56.0;

/// Below this saturation, 0 to 1, a color is taken as a gray and has no hue to keep.
const GRAY: f32 = 0.2;

pub type Rgb = (u8, u8, u8);

/// The settings of the pop art look, each as the app will have it. Sizes are shares of the photo's shorter side.
#[derive(Clone, Copy)]
struct PopArt {
  /// The most flat colors.
  colors: u32,
  /// How different two colors may be and still be merged into one, in Lab distance.
  merge: f32,
  /// How much a difference in brightness counts, when deciding how alike two colors are: less than a difference in color, so that the
  /// lit and the shaded skin merge, and a pale gray goes to the white and not to a tan.
  lightness: f32,
  /// How far the shadows are lifted before the merge, 0 to 100.
  shadows: f64,
  /// The size of the edge-keeping smoothing.
  smoothing: f32,
  /// The size of the median clean-up.
  clean: f32,
  /// The thickness of the outlines, or none.
  outlines: Option<f32>,
  /// What the flat colors become.
  recolor: Recolor,
  /// How much contrast the tone has before it sets the size of the dots, so the dots vary more, -100 to 100.
  tone_contrast: i32,
  /// Halftone dots over the skin, or none.
  dots: Option<Dots>,
  /// The ink of the lips, or none to leave them as the merge made them.
  lips: Option<Rgb>,
  /// Where the tone is darker than this, 0 to 255, in the face it is drawn black: the eyes, the brows and the nostrils. None leaves
  /// them as the merge made them.
  black: Option<u8>,
  /// The ink of small dots over the bright background, or none.
  backdrop: Option<Rgb>,
}

/// What the flat colors found in the photo become. The grays, which have no hue, are left alone by the last two.
#[derive(Clone, Copy)]
enum Recolor {
  /// Each color takes the nearest color of this palette, whatever its own color was.
  Palette(fn() -> Palette),
  /// Color to color: each flat color becomes a bolder version of itself, with the same hue and at least this much color and
  /// brightness, 0 to 1.
  Vivid { saturation: f32, value: f32 },
  /// Color to color: each flat color has its hue turned by this many degrees.
  Hue(f32),
}

/// A pattern of dots, as in a printed comic book: dots of one ink on a paper color, in rows turned 45 degrees.
#[derive(Clone, Copy)]
pub struct Dots {
  /// The distance between two dots in a row, as a share of the photo's shorter side.
  pub spacing: f32,
  /// The ink of the dots. None takes a deeper, more colorful version of the skin's color, as the look made it.
  pub ink: Option<Rgb>,
  /// The skin under the dots. None takes the skin's color, as the look made it, at the lightness of lit skin.
  pub paper: Option<Rgb>,
}

impl Default for PopArt {
  fn default() -> Self {
    PopArt {
      colors: 10,
      merge: 16.0,
      lightness: 0.6,
      shadows: 100.0,
      smoothing: 0.008,
      clean: 0.003,
      outlines: Some(0.003),
      recolor: Recolor::Palette(Palette::silk_screen),
      tone_contrast: 0,
      dots: None,
      lips: None,
      black: None,
      backdrop: None,
    }
  }
}

/// Run with the photo to use and the name of the picture to write, or with nothing to use the default photo. Run with `palettes` to
/// write a picture of the colors of each ready made palette instead.
pub fn main() {
  if std::env::args().nth(1).as_deref() == Some("palettes") {
    let palettes = [
      ("comic", Palette::comic()),
      ("silk-screen", Palette::silk_screen()),
      ("sepia", Palette::sepia()),
      ("neon", Palette::neon()),
      ("pastel", Palette::pastel()),
      ("cga", Palette::cga()),
      ("game-boy", Palette::game_boy()),
    ];
    for (name, palette) in palettes {
      // A palette of tones has a row for each ink and a column for each tone, and the others are one row.
      let columns = match name {
        "comic" => 5,
        "silk-screen" => 8,
        _ => palette.len().min(16),
      };
      let mut swatches = atlas().with_dimensions(columns);
      for color in palette.colors() {
        // A swatch is the color on a dark frame, so that colors that are alike are easy to tell apart.
        let mut swatch = Image::new_from_color(88, 88, Color::from_rgb(30, 30, 30));
        fill(Area::rect((4.0, 4.0), (80.0, 80.0)), Fill::solid(*color)).apply(&mut swatch);
        swatches.append(swatch);
      }
      swatches.create().write(format!("assets/out/palette-{name}.png"), None).expect("Failed to save image");
      println!("palette-{name}.png: {} colors in {columns} columns", palette.len());
    }
    return;
  }

  // `parts` makes the pictures the way Photoshop's guides do, from the parts of the photo the body model finds.
  let mut arguments = std::env::args().skip(1).peekable();
  let by_parts = arguments.next_if(|argument| argument == "parts").is_some();
  let file = arguments.next().unwrap_or(FILE.to_string());
  let out_file = arguments.next().unwrap_or(OUT_FILE.to_string());
  let mut original = Image::read(&file).expect("Failed to load image");
  resize(ResizeTarget::FitWidth(600)).apply(&mut original);

  if by_parts {
    let dots = Dots { spacing: 0.04, ink: None, paper: None };
    let guide = parts::Parts {
      background: (255, 214, 0),
      backdrop_dots: None,
      skin: (255, 190, 150),
      skin_dots: None,
      hair: (20, 20, 24),
      clothes: (20, 60, 200),
      accessories: (230, 30, 40),
      detail: parts::Detail::Share(0.3),
      detail_opacity: 255,
      black_dots: false,
      outlines: None,
      lines: None,
      dots,
    };
    // The look of the second guide: the colors it paints, the threshold at 30%, and black halftone dots over the whole subject.
    let second = parts::Parts {
      background: (0, 174, 239),
      skin: (252, 200, 182),
      hair: (230, 221, 118),
      clothes: (255, 255, 255),
      accessories: (255, 255, 255),
      detail: parts::Detail::Share(0.6),
      detail_opacity: 77,
      black_dots: true,
      outlines: None,
      ..guide
    };
    // The original first, then each look, left to right and top to bottom.
    let looks: [(&str, parts::Parts); 7] = [
      ("first guide: flat colors and the threshold", guide),
      ("first guide, with colored dots on the skin", parts::Parts { skin_dots: Some((225, 40, 50)), backdrop_dots: Some((20, 60, 200)), background: (0, 174, 239), ..guide }),
      ("first guide, outlines from the pixels", parts::Parts { outlines: Some(0.004), ..guide }),
      ("first guide, traced smooth lines", parts::Parts { lines: Some(0.006), ..guide }),
      ("second guide, finer dots", parts::Parts { dots: Dots { spacing: 0.03, ..dots }, ..second }),
      ("second guide, finer dots, yellow background", parts::Parts { dots: Dots { spacing: 0.03, ..dots }, background: (255, 214, 0), ..second }),
      ("second guide, fine dots, pink background", parts::Parts { dots: Dots { spacing: 0.022, ..dots }, background: (255, 120, 190), ..second }),
    ];
    let mut atlas = atlas().with_dimensions(3);
    atlas.append(original.clone());
    for (name, look) in looks {
      let start = std::time::Instant::now();
      atlas.append(parts::pop_art(&original, look));
      println!("{name}: {:?}", start.elapsed());
    }
    atlas.create().write(&out_file, None).expect("Failed to save image");
    return;
  }

  // The original first, then each look, left to right and top to bottom.
  let skin_dots = Dots { spacing: 0.04, ink: None, paper: None };
  let red_dots = Dots { ink: Some((225, 40, 50)), ..skin_dots };
  // The dots over the skin take the skin's color, as the look made it, and the background dots a light blue.
  let own = PopArt { dots: Some(skin_dots), black: Some(70), backdrop: Some((150, 185, 215)), outlines: Some(0.005), ..PopArt::default() };
  let looks: [(&str, PopArt); 5] = [
    ("skin and dots from the skin", PopArt { recolor: Recolor::Vivid { saturation: 0.6, value: 0.8 }, backdrop: Some((20, 80, 200)), ..own }),
    ("skin with red dots", PopArt { recolor: Recolor::Vivid { saturation: 0.6, value: 0.8 }, dots: Some(red_dots), backdrop: Some((20, 80, 200)), ..own }),
    ("comic palette, merged first", PopArt { recolor: Recolor::Palette(Palette::comic), backdrop: Some((20, 80, 200)), ..own }),
    ("comic palette, red dots", PopArt { recolor: Recolor::Palette(Palette::comic), dots: Some(red_dots), backdrop: Some((20, 80, 200)), ..own }),
    ("silk screen palette", PopArt { recolor: Recolor::Palette(Palette::silk_screen), ..own }),
  ];
  let mut atlas = atlas().with_dimensions(3);
  atlas.append(original.clone());
  for (name, look) in looks {
    let start = std::time::Instant::now();
    atlas.append(pop_art(&original, look));
    println!("{name}: {:?}", start.elapsed());
  }

  atlas.create().write(&out_file, None).expect("Failed to save image");
}

/// The same steps as the app's Pop Art, on the CPU, so a look can be seen without building the app.
fn pop_art(p_photo: &Image, p_look: PopArt) -> Image {
  let (width, height) = p_photo.dimensions::<u32>();
  let short = width.min(height) as f32;
  let rgb = |(red, green, blue): Rgb| Color::from_rgb(red, green, blue);

  let mut print = p_photo.clone();
  levels::highlights_shadows(0.0, p_look.shadows).with_tone(0.5).with_radius(width as f64 * 0.04).apply(&mut print);
  blur::surface_blur((short * p_look.smoothing).round().max(2.0) as u32, SMOOTH_LIMIT).apply(&mut print);
  noise::median((short * p_look.clean).max(1.0)).apply(&mut print);
  let smoothed = print.clone();

  // The tone of the photo, as it is before the colors are merged: the merge makes the skin one color, but the dots are bigger where
  // the skin is darker, so they need the tone from before.
  let mut tone = smoothed.clone();
  color::grayscale().apply(&mut tone);
  color::auto_tone().apply(&mut tone);
  levels::contrast(p_look.tone_contrast).apply(&mut tone);
  // With no colors asked for the photo is not merged first, and a palette is put straight onto its colors.
  let merge = color::quantize(p_look.colors).with_tolerance(p_look.merge).with_lightness(p_look.lightness);
  // The flat colors the merge finds, which a color-to-color look replaces one by one.
  let found = if p_look.colors == 0 { Vec::new() } else { merge.palette(&print) };
  if p_look.colors > 0 {
    merge.apply(&mut print);
  }
  // The skin is picked out by its color while the colors are still the photo's own: skin is orange, and the grays of the hair,
  // the wall and the window have no hue and are left out by the least saturation. Taking away the specks, and the lips, which are
  // picked out in the same way by their pinker hue, leaves the skin.
  let lips = Mask::from_color_range(&smoothed, Color::from_rgb(200, 80, 100), 16.0, 0.3, 0.2, 0.2);
  let skin = Mask::from_color_range(&print, Color::from_rgb(190, 110, 60), 28.0, 0.2, 0.15, 0.1).opened(1).subtracted(&lips);

  // The color of skin as the flat color of the photo is, which is darker than the skin looks since it is the average of the lit and the
  // shaded skin, and the color of lit skin, which a palette is asked for since the shaded average is nearer a gray than a skin color.
  let skin_reference = Color::from_rgb(190, 110, 60);
  let lit_skin = Color::from_rgb(235, 185, 150);
  let skin_color;
  // The color of the bright background, as the look made it.
  let window_color;
  if found.is_empty() {
    // Not merged: the palette goes straight onto the photo's colors, and the skin's color is the palette's nearest to it.
    let palette = match p_look.recolor {
      Recolor::Palette(make) => make(),
      _ => Palette::new(Vec::<Color>::new()),
    };
    color::quantize(0).with_palette(&palette).apply(&mut print);
    skin_color = palette.nearest(lit_skin).unwrap_or(skin_reference);
    window_color = palette.nearest(Color::white()).unwrap_or(Color::white());
  } else {
    // The new color for each flat color found, in the same order: each flat color is replaced by the one that goes with it.
    let palette = match p_look.recolor {
      Recolor::Palette(make) => Some(make()),
      _ => None,
    };
    let targets: Vec<Color> = found
      .iter()
      .map(|color| {
        match p_look.recolor {
          Recolor::Palette(_) => palette.as_ref().and_then(|palette| palette.nearest(*color)).unwrap_or(*color),
          // The grays have no hue to keep, so they are left as they are.
          Recolor::Vivid { .. } | Recolor::Hue(_) if color.hsv().1 < GRAY => *color,
          Recolor::Vivid { saturation, value } => color.with_least_saturation(saturation).with_least_value(value),
          Recolor::Hue(degrees) => color.with_hue_rotated(degrees),
        }
      })
      .collect();
    color::quantize(0).with_sources(found.clone()).with_palette(targets.clone()).apply(&mut print);
    // The skin's color, as the look made it: the new color of the flat color that is nearest to the color of skin.
    let flat_colors = Palette::new(found.clone());
    skin_color = match &palette {
      Some(palette) => palette.nearest(lit_skin).unwrap_or(skin_reference),
      None => targets[flat_colors.nearest_index(skin_reference).unwrap_or(0)],
    };
    window_color = targets[flat_colors.nearest_index(Color::white()).unwrap_or(0)];
  }
  if let Some(ink) = p_look.lips {
    lips.fill_image(&mut print, rgb(ink));
  }
  if let Some(limit) = p_look.black {
    // The dark parts of the face: closing the skin (growing it and shrinking it back) fills the holes of the eyes, and the tone
    // darker than the limit inside it is the eyes, the brows and the nostrils.
    let gap = (short * 0.012).round().max(2.0) as u32;
    let face = skin.closed(gap);
    let dark = Mask::from_brightness_range(&tone, 0, limit, 0);
    dark.multiplied(&face).fill_image(&mut print, Color::from_rgb(12, 12, 20));
  }

  // The outlines are worked out from the flat colors, before the dots, so that the dots do not make edges of their own.
  let outlines = p_look.outlines.map(|thickness| {
  color_edges((short * thickness).round().max(2.0) as u32).with_tolerance(1).applied(&print)
  });

  if let Some(dots) = p_look.dots {
    let cell = (short * dots.spacing).round().max(4.0);
    // The skin under the dots is the skin's color, at the lightness of lit skin, and the dots are a deeper, more colorful version of
    // it: the skin as the flat color is the average of the lit and the shaded skin, which is darker than the skin looks.
    let (_, skin_a, skin_b) = skin_color.lab();
    let paper = dots.paper.map(rgb).unwrap_or(Color::from_lab(SKIN_LIGHTNESS, skin_a, skin_b));
    let ink = dots.ink.map(rgb).unwrap_or(Color::from_lab(DOT_LIGHTNESS, skin_a * 1.7, skin_b * 1.4));
    halftone_region(&mut print, &tone, &skin, ink, paper, cell);
    // Small dots over the bright background, as the sky of a comic: the light parts that are not skin or lips.
    if let Some(backdrop) = p_look.backdrop {
      let region = Mask::from_brightness_range(&tone, 216, 255, 0).multiplied(&skin.inverted()).multiplied(&lips.inverted());
      // Dots all the same size are a halftone of a flat gray: a quarter of the cell is ink where the gray is a quarter dark.
      let flat_gray = Image::new_from_color(width, height, Color::from_rgb(192, 192, 192));
      halftone_region(&mut print, &flat_gray, &region, rgb(backdrop), window_color, cell);
    }
  }

  if let Some(edges) = outlines {
    blend(&edges).with_mode(BlendMode::Multiply).apply(&mut print);
  }
  print
}

/// Paints a region with the library's halftone screen of the photo's tone: dots of the ink on the paper, as big as the tone is dark.
pub fn halftone_region(p_print: &mut Image, p_tone: &Image, p_region: &Mask, p_ink: Color, p_paper: Color, p_cell: f32) {
  let layer = halftone(p_cell).with_ink(p_ink).with_paper(p_paper).applied(p_tone);
  p_region.fill_image(p_print, Fill::image(Arc::new(layer)));
}
