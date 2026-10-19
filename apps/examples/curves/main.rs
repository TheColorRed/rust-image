use abra::adjustments::prelude::levels::{CurveChannel, curves};
use abra::adjustments::prelude::*;
use abra::prelude::*;
use abra::tools::prelude::*;
use abra::transform::prelude::{ResizeTarget, resize};

const FILE: &str = "assets/architecture/red-building.jpg";
const OUT_FILE: &str = "assets/out/curves.png";

pub fn main() {
  let mut original = Image::read(FILE).expect("Failed to load image");
  resize(ResizeTarget::FitWidth(600)).apply(&mut original);

  // The original first, then each curve, left to right and top to bottom.
  let mut atlas = atlas().with_dimensions(3);
  atlas.append(original.clone());

  // One shape on all the channels together: the "S" that adds contrast, and the inverse "S" that takes it away.
  atlas.append(curved(&original, curves().with_curve(CurveChannel::Rgb, Curve::ease_in_out(2.0))));
  atlas.append(curved(&original, curves().with_curve(CurveChannel::Rgb, Curve::ease_out_in(2.0))));

  // A curve on one channel gives a color cast.
  atlas.append(curved(
    &original,
    curves()
      .with_curve(CurveChannel::Blue, Curve::ease_out_in(1.5))
      .with_curve(CurveChannel::Red, Curve::ease_in_out(1.3)),
  ));

  // Points, as drawn in a curves dialog: the black point is lifted and the white point lowered for a faded, matte look.
  atlas.append(curved(&original, curves().with_points(CurveChannel::Rgb, [(0, 40), (64, 70), (192, 205), (255, 235)])));

  // The Adobe steps: an S on red and green, an inverse S on blue, and a contrast curve with lifted blacks on all of them.
  atlas.append(curved(
    &original,
    curves()
      .with_curve(CurveChannel::Red, Curve::ease_in_out(1.2))
      .with_curve(CurveChannel::Green, Curve::ease_out_in(1.3))
      .with_curve(CurveChannel::Blue, Curve::ease_out_in(2.0))
      .with_points(CurveChannel::Rgb, [(0, 45), (64, 72), (192, 200), (255, 230)]),
  ));

  // A curve that falls is a negative, as in Photoshop. It is the straight line flipped, or the points (0, 255) and (255, 0).
  atlas.append(curved(&original, curves().with_curve(CurveChannel::Rgb, Curve::identity().inverted())));

  // A curve that rises and falls solarizes: the tones past the peak turn back over, so the lights go dark again.
  atlas.append(curved(&original, curves().with_points(CurveChannel::Rgb, [(0, 0), (140, 255), (255, 0)])));

  // A curve from two handles, as in CSS `cubic-bezier()`: here the CSS `ease-in-out`.
  let bezier = Curve::cubic_bezier(0.42, 0.0, 0.58, 1.0).expect("the handles are inside the box");
  atlas.append(curved(&original, curves().with_curve(CurveChannel::Rgb, bezier)));

  atlas.create().write(OUT_FILE, None).expect("Failed to save image");
}

/// A copy of `p_image` with `p_curves` applied.
fn curved(p_image: &Image, p_curves: impl Effect) -> Image {
  let mut image = p_image.clone();
  p_curves.apply(&mut image);
  image
}
