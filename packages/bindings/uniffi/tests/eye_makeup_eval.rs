//! A look at the mobile app's eye makeup (`apps/alakazam/mobile/src/lib/eye-makeup.ts`: eyeshadow, brow bone and under eye, and
//! their Fullness slider) on `assets/red-eye/Red-eye-flash.jpg`, without a device. It is the same algorithm, step for
//! step, on the same library handles the app uses: the face parser labels the eyes and eyebrows, each eye's region is made from
//! its own outline spread outwards by the fullness, and a color is put on through it. It writes one contact sheet, a row for each
//! kind of makeup and a column for each fullness, to `assets/out/eye-makeup-eval`. Run with
//! `cargo test -p abra-uniffi --test eye_makeup_eval -- --nocapture`.
//!
//! Keep the constants and the steps in step with `eye-makeup.ts`.

use std::sync::Arc;

use abra::abra_core::{ImageExt, PointF, Rect, Transform};
use abra::tools::prelude::{Tool, atlas_with};
use abra_uniffi::{AiModel, Color, Image, Mask, channel_mix, gaussian_blur};

const LUMA: [f32; 3] = [0.299, 0.587, 0.114];
/// How many widths of one eye the two eyes span; the softness of a region is measured in these.
const EYES_SPAN: f32 = 2.6;
/// How far towards the middle of the face a region may reach from its eye: this fraction of the way.
const TOWARDS_MIDDLE: f32 = 0.75;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
  Eyeliner,
  Eyeshadow,
  BrowBone,
  UnderEye,
}

impl Kind {
  fn blur(self) -> f32 {
    match self {
      Kind::Eyeliner => 0.03,
      Kind::Eyeshadow => 0.12,
      Kind::BrowBone => 0.15,
      Kind::UnderEye => 0.14,
    }
  }
  /// The fullest a region of this kind can be.
  fn fullest(self) -> f32 {
    if self == Kind::Eyeliner { 1.0 } else { FULL_MAX }
  }
}

/// A vertical band with feathered top and bottom edges.
fn feathered_band(p_width: u32, p_height: u32, p_top: f32, p_bottom: f32, p_feather: f32) -> Arc<Mask> {
  let feather = p_feather.max(1.0);
  let mut values = vec![0u8; (p_width * p_height) as usize];
  for y in 0..p_height {
    let from_top = ((y as f32 - p_top) / feather).clamp(0.0, 1.0);
    let from_bottom = ((p_bottom - y as f32) / feather).clamp(0.0, 1.0);
    let value = (255.0 * from_top.min(from_bottom)).round() as u8;
    values[(y * p_width) as usize..((y + 1) * p_width) as usize].fill(value);
  }
  Arc::new(Mask::from_values(p_width, p_height, values))
}

/// The fullest a region can be. The region around an eye is made once at this, as a map of how far each pixel is from the eye, and
/// the fullness slider only chooses how much of the map to show.
const FULL_MAX: f32 = 3.0;
const FULL_MIN: f32 = 0.3;
/// How sharply the edge of the shown part of the map falls off, as a multiple of the map's own steepness.
const EDGE_GAIN: f32 = 2.0;

/// How much of the map is hidden at a fullness, as the value below which it is not shown: all but the nearest part of it at the
/// least fullness, none of it at the most.
fn hidden_below(p_kind: Kind, p_fullness: f32) -> f32 {
  let reach = ((p_fullness - FULL_MIN) / (p_kind.fullest() - FULL_MIN)).clamp(0.0, 1.0);
  255.0 * 0.85 * (1.0 - reach).powf(1.5)
}

/// The map of how near each pixel of the photo is to this eye (255 at the eye's outline, falling to 0 at the fullest reach), and
/// the mask of everything that limits the region (the bands, the brow, the nose). The limits are kept apart from the map so that
/// showing more of the map does not sharpen their soft edges. Made once for an eye; the same whatever the fullness.
fn eye_field(
  p_eyes: &Arc<Mask>, p_brow: Option<&Arc<Mask>>, p_nose: Option<&Arc<Mask>>, p_kind: Kind, p_midline: f32,
) -> Option<(Arc<Mask>, Arc<Mask>)> {
  let eye_bounds = p_eyes.bounds(0)?;
  let fullest = p_kind.fullest();
  let reach = eye_bounds.width.max(eye_bounds.height) * 3.0 * fullest;
  let left = (eye_bounds.x - reach).floor().max(0.0);
  let top = (eye_bounds.y - reach).floor().max(0.0);
  let right = (eye_bounds.x + eye_bounds.width + reach).ceil().min(p_eyes.width() as f32);
  let bottom = (eye_bounds.y + eye_bounds.height + reach).ceil().min(p_eyes.height() as f32);
  let crop = Rect::new((left, top), (right - left, bottom - top));

  let eye = p_eyes.cropped(crop);
  let bounds = eye.bounds(0)?;
  let (width, height) = (eye.width(), eye.height());

  // The eye spread as far as it goes, brought up so that its brightest part is full.
  let blurred = eye.to_image();
  gaussian_blur((bounds.width * EYES_SPAN * p_kind.blur() * fullest).max(1.5) as f64).apply(blurred.clone());
  let peak = Mask::from_image(blurred.clone()).values().iter().copied().max().unwrap_or(1).max(1);
  blurred.mul(255.0 / peak as f32);
  // Never on the eye itself.
  let field = Arc::new(Mask::from_image(blurred)).subtracted(eye.clone());
  let mut limits = Arc::new(Mask::from_values(width, height, vec![255u8; (width * height) as usize]));

  // The highest a region goes: just under the eyebrow, along its lower edge, softened. Columns past the eyebrow take the nearest
  // column's edge.
  let brow = p_brow.map(|brow| brow.cropped(crop)).filter(|brow| brow.bounds(0).is_some());
  if let (Some(brow), true) = (&brow, matches!(p_kind, Kind::Eyeshadow | Kind::BrowBone)) {
    let brow_values = brow.values();
    let mut bottoms: Vec<i32> = (0..width)
      .map(|x| (0..height).rev().find(|y| brow_values[(y * width + x) as usize] > 127).map_or(-1, |y| y as i32))
      .collect();
    for x in 1..bottoms.len() {
      if bottoms[x] < 0 {
        bottoms[x] = bottoms[x - 1];
      }
    }
    for x in (0..bottoms.len().saturating_sub(1)).rev() {
      if bottoms[x] < 0 {
        bottoms[x] = bottoms[x + 1];
      }
    }
    let gap = (bounds.height * 0.25).round() as i32;
    let mut below_values = vec![0u8; (width * height) as usize];
    for (x, bottom) in bottoms.iter().enumerate() {
      for y in (*bottom + gap).clamp(0, height as i32) as u32..height {
        below_values[y as usize * width as usize + x] = 255;
      }
    }
    let below_image = Mask::from_values(width, height, below_values).to_image();
    gaussian_blur((bounds.height * 0.35).max(1.5) as f64).apply(below_image.clone());
    limits = limits.minimum(Arc::new(Mask::from_image(below_image)));
  }
  match p_kind {
    Kind::Eyeliner => {}
    Kind::Eyeshadow => {
      let band_top = if brow.is_some() { 0.0 } else { (bounds.y - bounds.height * 1.1 * fullest).max(0.0) };
      let band_bottom = (bounds.y + bounds.height * 0.45).min(height as f32);
      limits = limits.minimum(feathered_band(width, height, band_top, band_bottom, (bounds.height * 0.5).max(2.0)));
    }
    Kind::BrowBone => {
      brow.as_ref()?;
      let eye_values = eye.values();
      let mut tops: Vec<i32> = (0..width)
        .map(|x| (0..height).find(|y| eye_values[(y * width + x) as usize] > 127).map_or(-1, |y| y as i32))
        .collect();
      for x in 1..tops.len() {
        if tops[x] < 0 {
          tops[x] = tops[x - 1];
        }
      }
      for x in (0..tops.len().saturating_sub(1)).rev() {
        if tops[x] < 0 {
          tops[x] = tops[x + 1];
        }
      }
      let overlap = (bounds.height * 0.1).round() as i32;
      let mut above_values = vec![0u8; (width * height) as usize];
      for (x, top) in tops.iter().enumerate() {
        for y in 0..(*top + overlap).clamp(0, height as i32) as u32 {
          above_values[y as usize * width as usize + x] = 255;
        }
      }
      let above_image = Mask::from_values(width, height, above_values).to_image();
      gaussian_blur((bounds.height * 0.3).max(1.5) as f64).apply(above_image.clone());
      limits = limits.minimum(Arc::new(Mask::from_image(above_image)));
    }
    Kind::UnderEye => {
      let band_top = (bounds.y + bounds.height * 0.45).max(0.0);
      let band_bottom = (bounds.y + bounds.height * (0.45 + 1.7 * fullest)).min(height as f32);
      limits =
        limits.minimum(feathered_band(width, height, band_top, band_bottom, (bounds.height * 0.55 * fullest).max(2.0)));
    }
  }

  // Never over the nose: the nose the parser found, softened, is cut out of the region.
  if let Some(nose) = p_nose.map(|nose| nose.cropped(crop)).filter(|nose| nose.bounds(0).is_some()) {
    let nose_image = nose.to_image();
    gaussian_blur((bounds.height * 0.4).max(1.5) as f64).apply(nose_image.clone());
    nose_image.mul(2.0);
    limits = limits.subtracted(Arc::new(Mask::from_image(nose_image)));
  }

  // Keep it on this eye's side of the middle of the face, stopping short of it, with a soft edge so there is no line.
  let eye_middle = eye_bounds.x + eye_bounds.width / 2.0;
  let before = eye_middle < p_midline;
  let edge = eye_middle + (p_midline - eye_middle) * TOWARDS_MIDDLE - left;
  let soft = (eye_bounds.width * 0.6).max(2.0);
  let mut side_values = vec![0u8; (width * height) as usize];
  for x in 0..width {
    let inside = (((if before { edge - x as f32 } else { x as f32 - edge }) / soft) + 0.5).clamp(0.0, 1.0);
    for y in 0..height {
      side_values[(y * width + x) as usize] = (255.0 * inside).round() as u8;
    }
  }
  limits = limits.minimum(Arc::new(Mask::from_values(width, height, side_values)));
  let at = PointF::new(left, top);
  Some((field.placed(p_eyes.width(), p_eyes.height(), at), limits.placed(p_eyes.width(), p_eyes.height(), at)))
}

/// The part of the map that a fullness shows, as the region to put color through: a few passes over the map, no shape to make.
fn region_at(p_field: &(Arc<Mask>, Arc<Mask>), p_kind: Kind, p_fullness: f32) -> Arc<Mask> {
  let image = p_field.0.to_image();
  image.sub(hidden_below(p_kind, p_fullness));
  image.mul(EDGE_GAIN);
  Arc::new(Mask::from_image(image)).minimum(p_field.1.clone())
}

/// Puts a color (hue as the app's bar gives it, saturation, brightness) on `photo` through the regions of both eyes.
fn apply(
  p_photo: &Arc<Image>, p_regions: &[Arc<Mask>], p_hue: f32, p_strength: f32, p_saturation: f32, p_brightness: f32,
) {
  let rgb: Vec<f32> = Color::from_hsv(360.0 * (1.0 - p_hue), p_saturation, p_brightness)
    .rgb()
    .iter()
    .map(|value| *value as f32 / 255.0)
    .collect();
  let mix: Vec<Vec<f32>> = rgb
    .iter()
    .enumerate()
    .map(|(target, channel)| {
      LUMA
        .iter()
        .enumerate()
        .map(|(source, weight)| {
          (1.0 - p_strength) * if source == target { 1.0 } else { 0.0 } + p_strength * channel * weight
        })
        .collect()
    })
    .collect();
  for region in p_regions {
    channel_mix(mix[0].clone(), mix[1].clone(), mix[2].clone()).with_mask(Some(region.clone())).apply(p_photo.clone());
  }
}

#[test]
fn eye_makeup_contact_sheet() {
  let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");
  let model = concat!(env!("CARGO_MANIFEST_DIR"), "/../../abra/ai/face-parsing/models/resnet18.onnx");
  // Without Git LFS the file is a small pointer, not a model.
  if std::fs::metadata(model).map(|metadata| metadata.len()).unwrap_or(0) < 1_000_000 {
    return;
  }
  let photo = Arc::new(Image::read(format!("{root}/assets/red-eye/Red-eye-flash.jpg")));
  let labels = AiModel::load(model.to_owned()).unwrap().run(photo.duplicate()).unwrap().labels().unwrap();
  let eye = |side: &str| labels.mask_named(vec![format!("{side} eye")]);
  let brow = |side: &str| labels.mask_named(vec![format!("{side} eyebrow")]);
  let sides = [(eye("left"), brow("left")), (eye("right"), brow("right"))];
  let nose = labels.mask_named(vec!["nose".to_owned()]);
  // The middle of the face is halfway between the eyes.
  let centers: Vec<f32> =
    sides.iter().map(|(eyes, _)| eyes.bounds(0).map(|bounds| bounds.x + bounds.width / 2.0).unwrap()).collect();
  let midline = (centers[0] + centers[1]) / 2.0;

  // Around both eyes, with room for the largest regions.
  let both = eye("left").maximum(eye("right")).bounds(0).expect("the parser found the eyes");
  let left = (both.x - both.width * 0.3).max(0.0);
  let top = (both.y - both.width * 0.35).max(0.0);
  let crop = (
    left,
    top,
    (both.width * 1.6).min(photo.width() as f32 - left),
    (both.width * 0.75).min(photo.height() as f32 - top),
  );

  // What to try: name, kind, hue, strength, saturation, brightness (near the app's defaults for that makeup).
  let makeup = [
    ("eyeliner", Kind::Eyeliner, 0.0, 0.5, 0.0, 0.07),
    ("eyeshadow", Kind::Eyeshadow, 1.0 - 260.0 / 360.0, 0.6, 0.5, 0.5),
    ("brow-bone", Kind::BrowBone, 1.0 - 38.0 / 360.0, 0.6, 0.31, 0.87),
    ("under-eye", Kind::UnderEye, 1.0 - 16.0 / 360.0, 0.6, 0.48, 0.91),
  ];
  // How full, as the slider gives it. The first column of each row is the photo as it is.
  let sizes = [0.5, 1.0, 1.5, 2.0, 3.0];

  let mut cells = Vec::new();
  let mut cell = |image: &Arc<Image>| {
    let mut core = image.core();
    core.crop(crop.0, crop.1, crop.2, crop.3);
    cells.push(core);
  };
  for (name, kind, hue, strength, saturation, brightness) in makeup {
    cell(&photo);
    // Each eye's map is made once, however many fullnesses are tried.
    let fields: Vec<(Arc<Mask>, Arc<Mask>)> =
      sides.iter().filter_map(|(eyes, brows)| eye_field(eyes, Some(brows), Some(&nose), kind, midline)).collect();
    for fullness in sizes {
      let painted = photo.duplicate();
      let regions: Vec<Arc<Mask>> = fields.iter().map(|field| region_at(field, kind, fullness)).collect();
      assert!(!regions.is_empty(), "{name} made no region");
      apply(&painted, &regions, hue, strength, saturation, brightness);
      cell(&painted);
    }
  }

  std::fs::create_dir_all(format!("{root}/assets/out/eye-makeup-eval")).unwrap();
  atlas_with(cells)
    .with_cols(sizes.len() + 1)
    .create()
    .write(format!("{root}/assets/out/eye-makeup-eval/Red-eye-flash.png"), None)
    .unwrap();
}
