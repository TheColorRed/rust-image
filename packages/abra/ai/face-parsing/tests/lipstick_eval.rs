//! A look at the mobile app's lipstick (`apps/alakazam/mobile/src/lib/lipstick.ts`, `applyLipstick`) on the photos in
//! `assets/red-eye`, without a device. It finds the lips with the face parser, then puts the same channel-mix lipstick on at a
//! grid of hues and strengths, and writes one contact sheet to `assets/out/lipstick-eval`. Run with
//! `cargo test -p abra-face-parsing --test lipstick_eval -- --nocapture`. Set `LIPSTICK_SHADES` to a file of colors to try them
//! instead (see `SHADES_FILE`).
//!
//! Keep the constants and the mixer in step with `lipstick.ts`.

use abra_core::{Channels, Image, ImageExt, Rect, Transform};
use abra_face_parsing::{FaceParser, FaceRegion};
use tools::{Tool, atlas_with};

const LUMA: [f32; 3] = [0.299, 0.587, 0.114];
const KEEP_LIP_BRIGHTNESS: f32 = 0.25;
const STRENGTH_CURVE: f32 = 0.75;
const LIP_HUE: f32 = 355.0;
const FADE_SPEED: f32 = 3.0;
/// The least of any channel the lipstick has, so black lips still show their shading and shine.
const CHANNEL_FLOOR: f32 = 0.05;
/// Colors brighter than this make the lips lighter, not only keep them as bright as they were, so white is white.
const LIGHTEN_FROM: f32 = 0.45;
const LIGHTEN: f32 = 0.9;
/// How much of the lift a vivid color loses: pale and nude colors are lifted the most, a vivid one of the same brightness less.
const LIGHTEN_VIVID: f32 = 0.6;
/// The colors to try: name, hue in degrees, saturation, brightness.
const SWATCHES: [(&str, f32, f32, f32); 8] = [
  ("nude", 12.0, 0.5, 0.88),
  ("pink", 335.0, 0.55, 1.0),
  ("red", 0.0, 1.0, 0.9),
  ("berry", 330.0, 0.8, 0.5),
  ("burgundy", 350.0, 0.9, 0.4),
  ("brown", 25.0, 0.65, 0.4),
  ("black", 0.0, 0.0, 0.0),
  ("white", 0.0, 0.0, 1.0),
];
const STRENGTHS: [f32; 3] = [0.4, 0.7, 1.0];
/// A file of other colors to try, one per line as `name hue saturation brightness`, in place of the ones above, each at the
/// strength of the second column of the sheet. For a chart of shades, or a color someone is unhappy with.
const SHADES_FILE: &str = "LIPSTICK_SHADES";

/// A color from HSV (hue in degrees, saturation and value from 0 to 1) as red, green and blue from 0 to 1.
fn hsv(p_hue: f32, p_saturation: f32, p_value: f32) -> [f32; 3] {
  let channel = |n: f32| {
    let k = (n + p_hue / 60.0) % 6.0;
    p_value * (1.0 - p_saturation * k.min(4.0 - k).clamp(0.0, 1.0))
  };
  [channel(5.0), channel(3.0), channel(1.0)]
}

/// The lipstick's mixer: row `i` gives how much of each of the lips' red, green and blue makes channel `i`.
fn mixer(p_hue: f32, p_saturation: f32, p_value: f32, p_strength: f32) -> [[f32; 3]; 3] {
  let color = hsv(p_hue, p_saturation, p_value).map(|channel| channel.max(CHANNEL_FLOOR));
  let brightness = LUMA[0] * color[0] + LUMA[1] * color[1] + LUMA[2] * color[2];
  let blend = p_strength.powf(STRENGTH_CURVE);
  let apart = ((p_hue - LIP_HUE).rem_euclid(360.0)).min((LIP_HUE - p_hue).rem_euclid(360.0)) / 180.0;
  // The lips' own color fades as the lipstick comes in: more the farther the hue, and fully for a color with no hue to speak of.
  let gray = (blend * FADE_SPEED).min(1.0) * apart.sqrt().max(1.0 - p_saturation);
  let lighten = 1.0
    + LIGHTEN * ((brightness - LIGHTEN_FROM) / (1.0 - LIGHTEN_FROM)).max(0.0) * (1.0 - LIGHTEN_VIVID * p_saturation);
  let mut rows = [[0.0; 3]; 3];
  for (index, row) in rows.iter_mut().enumerate() {
    for (source, weight) in row.iter_mut().enumerate() {
      let own = (1.0 - gray) * if index == source { 1.0 } else { 0.0 } + gray * LUMA[source];
      *weight =
        (1.0 - blend) * own + blend * (color[index] / brightness.powf(KEEP_LIP_BRIGHTNESS)) * lighten * LUMA[source];
    }
  }
  rows
}

#[test]
fn lipstick_contact_sheet() {
  let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../..");
  let model = concat!(env!("CARGO_MANIFEST_DIR"), "/models/resnet18.onnx");
  if std::fs::metadata(model).map(|metadata| metadata.len()).unwrap_or(0) < 1_000_000 {
    return;
  }
  let parser = FaceParser::load(model).unwrap();
  std::fs::create_dir_all(format!("{root}/assets/out/lipstick-eval")).unwrap();

  for variant in ["swatches"] {
    for photo_name in ["Red-eye-flash.jpg"] {
      let photo = Image::read(format!("{root}/assets/red-eye/{photo_name}")).unwrap();
      let (width, height) = photo.dimensions::<u32>();
      let segmentation = parser.process(&photo).unwrap();
      let lips = segmentation.mask(FaceRegion::UpperLip).maximum(&segmentation.mask(FaceRegion::LowerLip));
      let bounds = lips.bounds(0).expect("the parser found lips");
      println!("{photo_name}: {width}x{height}, lips at {bounds:?}");

      // The mouth, with room around it, so the sheet shows the lips large.
      let margin = bounds.width * 0.6;
      let left = (bounds.x - margin).max(0.0) as u32;
      let top = (bounds.y - margin).max(0.0) as u32;
      let right = ((bounds.x + bounds.width + margin) as u32).min(width);
      let bottom = ((bounds.y + bounds.height + margin) as u32).min(height);
      let (cell_w, cell_h) = (right - left, bottom - top);

      // The mouth cropped out once; each cell of the sheet is a copy of it, with a lipstick on or not.
      let mut mouth = photo.clone();
      mouth.crop(left, top, cell_w, cell_h);
      let mouth_pixels = mouth.rgba().to_vec();
      let mask = lips.cropped(Rect::new((left as f32, top as f32), (cell_w as f32, cell_h as f32)));
      let covered = mask.values();
      let swatches: Vec<(String, f32, f32, f32)> = match std::env::var(SHADES_FILE) {
        Ok(path) => std::fs::read_to_string(path)
          .unwrap()
          .lines()
          .map(|line| {
            let mut parts = line.split(' ');
            let name = parts.next().unwrap().to_owned();
            let mut number = || parts.next().unwrap().parse::<f32>().unwrap();
            (name, number(), number(), number())
          })
          .collect(),
        Err(_) => {
          SWATCHES.iter().map(|(name, hue, saturation, value)| (name.to_string(), *hue, *saturation, *value)).collect()
        }
      };
      let strengths: &[f32] = if std::env::var(SHADES_FILE).is_ok() { &[0.7] } else { &STRENGTHS };
      let shades = std::env::var(SHADES_FILE).is_ok();
      let columns = strengths.len() + 1;

      // One row for each color: the mouth as it is, then the lipstick at each strength. The atlas lays them out in a grid.
      let mut cells = Vec::new();
      for (_, hue, saturation, value) in &swatches {
        for column in (if shades { 1 } else { 0 })..columns {
          let matrix = (column > 0).then(|| mixer(*hue, *saturation, *value, strengths[column - 1]));
          let mut pixels = mouth_pixels.clone();
          if let Some(matrix) = matrix {
            for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
              let covered = covered[index] as f32 / 255.0;
              let own = [pixel[0] as f32, pixel[1] as f32, pixel[2] as f32];
              for channel in 0..3 {
                let mixed: f32 = (0..3).map(|c| matrix[channel][c] * own[c]).sum::<f32>().clamp(0.0, 255.0);
                pixel[channel] = (own[channel] * (1.0 - covered) + mixed * covered) as u8;
              }
            }
          }
          cells.push(Image::new_from_pixels(cell_w, cell_h, pixels, Channels::RGBA));
        }
      }
      atlas_with(cells)
        // A chart of shades is one lipstick each, so it is laid out wide; the grid of settings is a row for each color.
        .with_cols(if shades { 6 } else { columns })
        .create()
        .write(format!("{root}/assets/out/lipstick-eval/{variant}-{photo_name}.png"), None)
        .unwrap();
    }
  }
}
