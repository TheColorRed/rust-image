//! Pop art made the way Photoshop's guides make it: the subject is cut out, the background is a solid color, the skin, the hair and the
//! clothes are each filled with a flat color, and a black and white threshold of the photo is multiplied over them to bring back the
//! detail. The parts are found by the body parser, which tells the background, the hair, the skin, the clothes and the accessories apart.

use crate::{Dots, Rgb, halftone_region};
use abra::abra_core::blend::blend;
use abra::abra_core::BlendMode;
use abra::adjustments::prelude::{Effect, color};
use abra::filters::prelude::edges::color_edges;
use abra::filters::prelude::halftone::halftone;
use abra::filters::prelude::{noise, smooth};
use abra::mask::prelude::Mask;
use abra::prelude::*;
use abra_body_segmentation::prelude::{BodyParser, BodyRegion, ImageAi};
use abra_matting::Matting;

const BODY_MODEL: &str = "packages/ai/body-segmentation/models/selfie_multiclass_256x256.onnx";
const MATTING_MODEL: &str = "packages/ai/matting/models/rvm_mobilenetv3_fp32.onnx";

/// How a picture is made. Colors are RGB and sizes are shares of the photo's shorter side.
#[derive(Clone, Copy)]
pub struct Parts {
  /// The solid color of the background.
  pub background: Rgb,
  /// The ink of small dots over the background, or none for a solid color.
  pub backdrop_dots: Option<Rgb>,
  pub skin: Rgb,
  /// The ink of the halftone dots over the skin, or none for flat skin.
  pub skin_dots: Option<Rgb>,
  pub hair: Rgb,
  pub clothes: Rgb,
  pub accessories: Rgb,
  /// Where the picture is black: the threshold of the guide.
  pub detail: Detail,
  /// How black that is, 0 to 255: the opacity of the threshold layer. The second guide sets it to about 30%, which is 77.
  pub detail_opacity: u8,
  /// Black halftone dots over the whole subject, bigger where the tone is darker, as the second guide's Halftone Pattern makes them.
  pub black_dots: bool,
  /// The thickness of the lines between the parts, or none.
  pub outlines: Option<f32>,
  /// The width of lines traced around each part as smooth paths, or none: the edge of each mask is found (`Mask::contours`), simplified and
  /// smoothed, and stroked with a width that tapers to a point.
  pub lines: Option<f32>,
  pub dots: Dots,
}

/// Where the tone of the photo is dark enough for the picture to be black.
#[derive(Clone, Copy)]
pub enum Detail {
  /// Darker than this, 0 to 255, as the slider of the Threshold adjustment is set.
  Level(u8),
  /// The darkest part of the subject, this share of it, from 0 to 1: a level that suits the photo, since a fixed one turns a dark subject
  /// black and a pale one white.
  Share(f32),
}

/// The masks of the parts the model finds, each followed to the edges of the photo.
struct Found {
  /// The person, from the matting model: a soft alpha, `255` on the person and `0` on the background.
  subject: Mask,
  hair: Mask,
  skin: Mask,
  clothes: Mask,
  accessories: Mask,
  /// How big the person is in the photo: the side of a square of the same area as their bounds.
  size: f32,
}

fn find_parts(p_photo: &Image) -> Found {
  let (width, height) = p_photo.dimensions::<u32>();
  // The person is cut out by the matting model, and each part is kept to the inside of the cutout.
  let subject = Matting::load(MATTING_MODEL).expect("Failed to load the matting model").process(p_photo).expect("Failed to matte");
  // The parts model is run on the part of the photo with the person in it, with a margin, since a person who is small in the photo is
  // only a few pixels to a model that sees a small copy of what it is given.
  let parser = BodyParser::load(BODY_MODEL).expect("Failed to load the body model");
  let (parts, size) = match subject.bounds(128) {
    Some(bounds) => {
      let (left, top, right, bottom) = bounds.edges::<f32>();
      (parser.process_within(p_photo, bounds, 0.1), ((right - left) * (bottom - top)).sqrt())
    }
    None => (parser.process(p_photo), (width * height) as f32),
  };
  let parts = parts.expect("Failed to run the body model");
  // The model's edges are rough. The guided filter makes each mask follow the edges of the photo.
  let radius = (size * 0.012).round().max(2.0) as u32;
  let follow = |mask: Mask| smooth::guided_filter(&mask, p_photo, radius, 0.002).multiplied(&subject);
  Found {
    hair: follow(parts.mask(BodyRegion::Hair)),
    skin: follow(parts.mask_of(&[BodyRegion::BodySkin, BodyRegion::FaceSkin])),
    clothes: follow(parts.mask(BodyRegion::Clothes)),
    accessories: follow(parts.mask(BodyRegion::Accessories)),
    subject,
    size,
  }
}

pub fn pop_art(p_photo: &Image, p_look: Parts) -> Image {
  let (width, height) = p_photo.dimensions::<u32>();
  let short = width.min(height) as f32;
  let rgb = |(red, green, blue): Rgb| Color::from_rgb(red, green, blue);
  let found = find_parts(p_photo);
  let subject = found.subject.clone();

  // The solid background, then each part filled with its flat color. Clothes and accessories go first and the skin last, since the
  // masks of neighbors can overlap a little where they meet.
  let mut print = Image::new_from_color(width, height, rgb(p_look.background));
  for (mask, ink) in [
    (&found.clothes, p_look.clothes),
    (&found.accessories, p_look.accessories),
    (&found.hair, p_look.hair),
    (&found.skin, p_look.skin),
  ] {
    mask.fill_image(&mut print, rgb(ink));
  }

  // The threshold of the guide: the tone of the photo, evened out, with the subject kept and the rest left out. Where it is dark the
  // picture will be black. A little smoothing first keeps the black shapes from breaking into specks of noise.
  let mut tone = p_photo.clone();
  noise::median((short * 0.004).max(1.0)).apply(&mut tone);
  color::grayscale().apply(&mut tone);
  color::auto_tone().apply(&mut tone);
  let level = match p_look.detail {
    Detail::Level(level) => level,
    Detail::Share(share) => subject.brightness_percentile(&tone, share, 127),
  };
  let detail = Mask::from_brightness_range(&tone, 0, level, 0).multiplied(&subject);

  // The lines between the parts come from the flat colors, before the dots and the black go on, so that they do not make edges of
  // their own.
  let outlines = p_look.outlines.map(|thickness| {
    color_edges((short * thickness).round().max(2.0) as u32).with_tolerance(1).applied(&print)
  });

  // The dots are sized by the person and not by the photo, so that a person who is small in the photo has dots that suit them.
  let cell = (found.size * p_look.dots.spacing).round().max(4.0);
  if let Some(ink) = p_look.skin_dots {
    halftone_region(&mut print, &tone, &found.skin, rgb(ink), rgb(p_look.skin), cell);
  }
  if let Some(ink) = p_look.backdrop_dots {
    // Dots all the same size are a halftone of a flat gray: a quarter of the cell is ink where the gray is a quarter dark.
    let flat_gray = Image::new_from_color(width, height, Color::from_rgb(192, 192, 192));
    halftone_region(&mut print, &flat_gray, &subject.inverted(), rgb(ink), rgb(p_look.background), cell);
  }

  // The black of the threshold goes over everything, as a Multiply layer does, and then the lines.
  // Black halftone dots over the subject, multiplied: a white layer with dots that are bigger the darker the tone, as the second guide's
  // Halftone Pattern makes them.
  if p_look.black_dots {
    let mut dots = halftone(cell).with_ink(Color::from_rgb(12, 12, 20)).applied(&tone);
    // Only the subject has dots: the rest is white, which multiplies to nothing.
    subject.inverted().fill_image(&mut dots, Color::white());
    blend(&dots).with_mode(BlendMode::Multiply).apply(&mut print);
  }
  detail.fill_image(&mut print, Color::from_rgba(12, 12, 20, p_look.detail_opacity));
  if let Some(edges) = outlines {
    blend(&edges).with_mode(BlendMode::Multiply).apply(&mut print);
  }
  if let Some(line_width) = p_look.lines {
    // The lines along each part and the detail, as one mask: each shape is traced, simplified to within a pixel and a half of the person's
    // size, smoothed through the points that are left, stroked, and drawn, all in the one call.
    let tolerance = (found.size * 0.003).max(1.0);
    let mut lines = Mask::empty(width, height);
    for mask in [&found.hair, &found.skin, &found.clothes, &found.accessories, &detail] {
      lines = lines.maximum(&mask.outlined(128, tolerance, 50.0, short * line_width));
    }
    lines.fill_image(&mut print, Color::from_rgb(12, 12, 20));
  }
  print
}
