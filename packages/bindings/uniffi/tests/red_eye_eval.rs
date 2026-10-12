//! A check of the tap-to-fix red-eye algorithm of the mobile app (`apps/alakazam/mobile/src/lib/red-eye.ts`, `fixRedEyeAt`),
//! run on the photos in `assets/red-eye`. It is the same algorithm, step for step, using the same library handles the app
//! uses, so a change to the limits can be tried here on every photo at once. Each photo has taps that must fix a pupil and
//! taps that must not change anything. Run with `cargo test -p abra-uniffi --test red_eye_eval -- --nocapture`; the pictures
//! it makes go to `assets/out/red-eye-eval`.
//!
//! Keep the constants in step with `red-eye.ts`.

use std::sync::Arc;

use abra::abra_core::{PointF, Rect};
use abra_uniffi::{Area, Color, Image, Mask, channel_mix, gaussian_blur};

const TAP_WINDOWS: [f32; 3] = [0.02, 0.04, 0.07];
const TAP_CORE_HUE: f32 = 350.0;
const TAP_CORE_HUE_RANGE: f32 = 14.0;
const TAP_CORE_MIN_SATURATION: f32 = 0.7;
const TAP_CORE_MIN_VALUE: f32 = 0.2;
const TAP_CORE_FEATHER: f32 = 0.1;
const TAP_BOX_THRESHOLD: u8 = 127;
const TAP_MAX_PUPIL: f32 = 0.06;
const TAP_PUPIL_SHAPE: [f32; 2] = [0.6, 1.5];
const TAP_REFINE_WINDOW: f32 = 1.6;
const TAP_REFINES: usize = 4;
const TAP_MIN_FILL: f32 = 0.25;
const TAP_PUPIL_GROWTH: f32 = 1.35;
const TAP_RIM_HUE_RANGE: f32 = 22.0;
const TAP_RIM_MIN_SATURATION: f32 = 0.5;
const TAP_RIM_MIN_VALUE: f32 = 0.1;
const TAP_RIM_FEATHER: f32 = 0.3;
const SOFT_EDGE: f32 = 0.12;
const PUPIL_LEVEL: f32 = 0.6;
const RED_OF_AVERAGE: f32 = 0.8;

fn inside(p_box: Rect, p_frame: Rect, p_side: f32) -> Option<Rect> {
  let part = p_box.rounded_out().intersect(p_frame);
  (part.width >= p_side && part.height >= p_side).then_some(part)
}

/// What a square around a spot holds of pupil-colored pixels: the box around them, whether they run into a side of the square
/// that is not the edge of the photo (so there is more of them past it), and how full the box is.
struct Look {
  found: Rect,
  cut: bool,
  fill: f32,
}

fn look(p_photo: &Arc<Image>, p_cx: f32, p_cy: f32, p_half: f32, p_color: &Arc<Color>) -> Option<Look> {
  let (width, height) = (p_photo.width() as f32, p_photo.height() as f32);
  let frame = Rect::new((0, 0), (width, height));
  let area = inside(Rect::new((p_cx - p_half, p_cy - p_half), (p_half * 2.0, p_half * 2.0)), frame, 4.0)?;
  let crop = p_photo.cropped(area.x as u32, area.y as u32, area.width as u32, area.height as u32);
  let core = Mask::from_color_range(
    crop,
    p_color.clone(),
    TAP_CORE_HUE_RANGE,
    TAP_CORE_MIN_SATURATION,
    TAP_CORE_MIN_VALUE,
    TAP_CORE_FEATHER,
  );
  let found = core.bounds(TAP_BOX_THRESHOLD)?;
  let cut = (found.x <= 0.0 && area.x > 0.0)
    || (found.y <= 0.0 && area.y > 0.0)
    || (found.x + found.width >= area.width && area.x + area.width < width)
    || (found.y + found.height >= area.height && area.y + area.height < height);
  let fill = core.cropped(found).coverage(TAP_BOX_THRESHOLD);
  Some(Look {
    found: found.translated(area.x, area.y),
    cut,
    fill,
  })
}

/// Finds the pupil near a tap: the pupil-colored pixels in a small square around it, then, while they run into the side of the
/// square, a square around what was found, until the pupil sits wholly inside one.
fn find_pupil(p_photo: &Arc<Image>, p_x: f32, p_y: f32, p_color: &Arc<Color>) -> Option<Look> {
  let shorter = p_photo.width().min(p_photo.height()) as f32;
  let mut current = TAP_WINDOWS.iter().find_map(|fraction| look(p_photo, p_x, p_y, shorter * fraction, p_color))?;
  for _ in 0..TAP_REFINES {
    if !current.cut {
      break;
    }
    let middle = current.found.center();
    current = look(
      p_photo,
      middle.x,
      middle.y,
      current.found.width.max(current.found.height) * TAP_REFINE_WINDOW / 2.0 + 2.0,
      p_color,
    )?;
  }
  if current.cut {
    return None;
  }
  // One more look, centered on the pupil, to get all of it.
  let middle = current.found.center();
  let centered =
    look(p_photo, middle.x, middle.y, current.found.width.max(current.found.height) * TAP_REFINE_WINDOW / 2.0, p_color);
  Some(centered.filter(|l| !l.cut).unwrap_or(current))
}

fn softened(p_pupils: &Arc<Mask>, p_width: f32) -> Arc<Mask> {
  let edge = p_pupils.to_image();
  gaussian_blur((p_width * SOFT_EDGE).max(1.0) as f64).apply(edge.clone());
  let blurred = Arc::new(Mask::from_image(edge));
  blurred.added(blurred.clone())
}

fn correct(p_photo: &Arc<Image>, p_pupils: Arc<Mask>) {
  let red = (PUPIL_LEVEL * RED_OF_AVERAGE) / 2.0;
  channel_mix(vec![0.0, red, red], vec![0.0, PUPIL_LEVEL, 0.0], vec![0.0, 0.0, PUPIL_LEVEL])
    .with_mask(Some(p_pupils))
    .apply(p_photo.clone());
}

/// `fixRedEyeAt`: whether it found a pupil near the tap and fixed it. The pupil it measured is returned too.
fn fix_red_eye_at(p_photo: &Arc<Image>, p_x: f32, p_y: f32) -> Option<Rect> {
  let (width, height) = (p_photo.width() as f32, p_photo.height() as f32);
  let frame = Rect::new((0, 0), (width, height));
  let white = Arc::new(Color::from_rgba(255, 255, 255, 255));
  let red = Arc::new(Color::red());
  let pupil_color = Arc::new(Color::from_hsv(TAP_CORE_HUE, 1.0, 1.0));
  let shorter = width.min(height);

  let found = find_pupil(p_photo, p_x, p_y, &pupil_color)?;
  let roundish = |p_box: Rect| {
    let ratio = p_box.width / p_box.height;
    ratio >= TAP_PUPIL_SHAPE[0] && ratio <= TAP_PUPIL_SHAPE[1]
  };
  let pupil = found.found;
  if pupil.width.max(pupil.height) > shorter * TAP_MAX_PUPIL || !roundish(pupil) || found.fill < TAP_MIN_FILL {
    return None;
  }

  let size = pupil.width.max(pupil.height) * TAP_PUPIL_GROWTH;
  let area = inside(pupil.scaled(TAP_PUPIL_GROWTH * 1.5, TAP_PUPIL_GROWTH * 1.5), frame, 4.0)?;
  let crop = p_photo.cropped(area.x as u32, area.y as u32, area.width as u32, area.height as u32);
  let rim = Mask::from_color_range(
    crop.clone(),
    red,
    TAP_RIM_HUE_RANGE,
    TAP_RIM_MIN_SATURATION,
    TAP_RIM_MIN_VALUE,
    TAP_RIM_FEATHER,
  );
  let center = pupil.center();
  let disc = Arc::new(Area::circle(PointF::new(center.x - area.x, center.y - area.y), (size / 2.0) as f64));
  let discs = Arc::new(Mask::empty(crop.width(), crop.height()));
  discs.draw_area(disc.clone(), white, Some(disc.bounds().origin()));
  let pupils = discs.minimum(Arc::new(rim));
  let soft = softened(&pupils, size);
  correct(p_photo, soft.placed(p_photo.width(), p_photo.height(), PointF::new(area.x, area.y)));
  Some(pupil)
}

/// What each search window sees at a tap, and the color of the pixel there, for finding out why a tap went wrong.
fn describe(p_photo: &Arc<Image>, p_x: f32, p_y: f32) -> String {
  let pupil_color = Arc::new(Color::from_hsv(TAP_CORE_HUE, 1.0, 1.0));
  let shorter = p_photo.width().min(p_photo.height()) as f32;
  let pixel = p_photo.get_pixel(p_x as u32, p_y as u32).unwrap_or_default();
  let (r, g, b) = (pixel[0] as f32 / 255.0, pixel[1] as f32 / 255.0, pixel[2] as f32 / 255.0);
  let (max, min) = (r.max(g).max(b), r.min(g).min(b));
  let d = max - min;
  let hue = if d == 0.0 {
    0.0
  } else if max == r {
    (((g - b) / d + 6.0) % 6.0) * 60.0
  } else if max == g {
    ((b - r) / d + 2.0) * 60.0
  } else {
    ((r - g) / d + 4.0) * 60.0
  };
  let mut text =
    format!("tap pixel h/s/v {:.0}/{:.0}/{:.0};", hue, if max == 0.0 { 0.0 } else { d / max * 100.0 }, max * 100.0);
  match find_pupil(p_photo, p_x, p_y, &pupil_color) {
    Some(l) => {
      text += &format!(
        " found {:.0}x{:.0}@{:.0},{:.0} fill {:.2}, {:.1}% of the shorter side, ratio {:.2}",
        l.found.width,
        l.found.height,
        l.found.x,
        l.found.y,
        l.fill,
        l.found.width.max(l.found.height) / shorter * 100.0,
        l.found.width / l.found.height
      )
    }
    None => text += " found nothing (none, or too large to settle in a window)",
  }
  text
}

struct Photo {
  name: &'static str,
  file: &'static str,
  /// Taps that must find and fix a pupil: the pupil centers, and a few pixels off them, as a finger would land.
  pupils: &'static [(f32, f32)],
  /// Taps that must change nothing: skin, lips, hair, red clothes, and eyes that are not red.
  not_pupils: &'static [(f32, f32)],
}

const PHOTOS: &[Photo] = &[
  Photo {
    name: "girl",
    file: "assets/red-eye/1JMredeye.jpg",
    pupils: &[(104.0, 174.0), (186.0, 162.0)],
    not_pupils: &[
      (150.0, 260.0),
      (60.0, 200.0),
      (230.0, 230.0),
      (140.0, 215.0),
      (100.0, 120.0),
      (40.0, 330.0),
    ],
  },
  Photo {
    name: "flash",
    file: "assets/red-eye/Red-eye-flash.jpg",
    pupils: &[(331.0, 148.0), (481.0, 150.0)],
    not_pupils: &[
      (250.0, 230.0),
      (480.0, 260.0),
      (390.0, 325.0),
      (160.0, 170.0),
      (400.0, 90.0),
      (340.0, 380.0),
    ],
  },
  Photo {
    name: "child",
    file: "assets/red-eye/red-eye-effect-min.jpg.webp",
    pupils: &[(138.0, 174.0), (218.0, 175.0)],
    not_pupils: &[
      (178.0, 268.0),
      (150.0, 278.0),
      (110.0, 220.0),
      (250.0, 215.0),
      (180.0, 130.0),
      (178.0, 250.0),
    ],
  },
  Photo {
    name: "party",
    file: "assets/red-eye/FB_IMG_1762049916951.jpg",
    // Nobody in this photo has red-eye, so every tap must change nothing, including on the eyes of both women.
    pupils: &[],
    not_pupils: &[
      (467.0, 351.0),
      (498.0, 349.0),
      (338.0, 377.0),
      (364.0, 377.0),
      (482.0, 357.0),
      (350.0, 380.0),
      (480.0, 380.0),
      (330.0, 230.0),
      (480.0, 200.0),
      (300.0, 600.0),
      (480.0, 520.0),
      (400.0, 640.0),
    ],
  },
];

/// Offsets from a pupil center, in pixels of a photo about 400 pixels across, that a finger could land on.
const OFFSETS: [(f32, f32); 5] = [(0.0, 0.0), (3.0, 0.0), (-3.0, 3.0), (0.0, -4.0), (5.0, 2.0)];

#[test]
fn the_tap_fixes_pupils_and_leaves_everything_else_alone() {
  let root = format!("{}/../../..", env!("CARGO_MANIFEST_DIR"));
  let out = format!("{root}/assets/out/red-eye-eval");
  // The photos are not part of every checkout; without them there is nothing to check.
  if !std::path::Path::new(&format!("{root}/assets/red-eye")).exists() {
    println!("skipped: assets/red-eye has no photos");
    return;
  }
  std::fs::create_dir_all(&out).expect("the folder for the pictures can be made");
  let (mut missed, mut false_fixes) = (0, 0);
  for photo in PHOTOS {
    let path = format!("{root}/{}", photo.file);
    let load = || Arc::new(Image::read(path.clone()));
    let scale = (load().width() as f32 / 400.0).max(1.0);
    println!("--- {} ({}x{})", photo.name, load().width(), load().height());

    // Every pupil tap on its own, on a fresh copy, and then all of them on one copy to look at.
    let combined = load();
    for (x, y) in photo.pupils {
      let mut line = format!("  pupil ({x},{y}):");
      for (dx, dy) in OFFSETS {
        let fixed = fix_red_eye_at(&load(), x + dx * scale, y + dy * scale).is_some();
        line += &format!(" {}", if fixed { "fixed" } else { "MISSED" });
        if !fixed {
          println!("    missed at offset ({dx},{dy}): {}", describe(&load(), x + dx * scale, y + dy * scale));
        }
        missed += !fixed as u32;
      }
      println!("{line}");
      println!("    centre: {}", describe(&load(), *x, *y));
      fix_red_eye_at(&combined, *x, *y);
    }
    if !photo.pupils.is_empty() {
      combined.write(format!("{out}/{}-fixed.jpg", photo.name), None);
    }

    // Every tap that is not a pupil, on its own: nothing may change. Then all of them on one copy to look at.
    let damaged = load();
    for (x, y) in photo.not_pupils {
      let fresh = load();
      let pupil_color = Arc::new(Color::from_hsv(TAP_CORE_HUE, 1.0, 1.0));
      if let Some(l) = find_pupil(&fresh, *x, *y, &pupil_color) {
        println!("  non-pupil ({x},{y}) finds a candidate: {}", describe(&fresh, *x, *y));
        let _ = l;
      }
      let changed = fix_red_eye_at(&fresh, *x, *y);
      if let Some(pupil) = changed {
        println!("  NOT a pupil ({x},{y}) was FIXED, as a pupil {pupil:?}: {}", describe(&load(), *x, *y));
        false_fixes += 1;
      }
      fix_red_eye_at(&damaged, *x, *y);
    }
    println!("  {} taps that are not pupils", photo.not_pupils.len());
    if !photo.not_pupils.is_empty() {
      damaged.write(format!("{out}/{}-not-pupils.jpg", photo.name), None);
    }
  }
  println!("=== missed pupils: {missed}, false fixes: {false_fixes}");
  assert_eq!((missed, false_fixes), (0, 0));
}
