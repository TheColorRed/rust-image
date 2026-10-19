use abra_core::{
  Curve, Image, PointF,
  image::gpu::{GpuPass, GpuProcessor},
};

use crate::lut::channel_lut_pass;
use options::{Effect, Options};

use rayon::prelude::*;

/// A channel of a [`Curves`] adjustment that a curve can be set for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveChannel {
  /// All of the red, green and blue channels together. It is applied after the curve of each channel on its own.
  Rgb,
  /// The red channel.
  Red,
  /// The green channel.
  Green,
  /// The blue channel.
  Blue,
}

/// A curves adjustment: the tones of the image are remapped by a [`Curve`] for the red, green and blue channels together and for
/// each of them on its own. The input is on the horizontal axis and the output on the vertical, both on the scale of a byte, `0`
/// to `255`. A channel with no curve is left as it is, and the alpha is always left alone. Create one with [`curves`].
///
/// The value of a channel goes through its own curve first, and then through the `Rgb` curve, as in Photoshop.
///
/// The GPU version is a lookup table built from this CPU code, so the two give the same pixels.
#[derive(Clone)]
pub struct Curves {
  rgb: Option<Curve>,
  red: Option<Curve>,
  green: Option<Curve>,
  blue: Option<Curve>,
  options: Options,
}

impl Curves {
  /// Sets the curve of a channel, in place of the one it had.
  /// - `p_channel`: The channel the curve is for.
  /// - `p_curve`: The curve, such as [`Curve::ease_in_out`] or [`Curve::new`] through points.
  pub fn with_curve(mut self, p_channel: CurveChannel, p_curve: Curve) -> Self {
    match p_channel {
      CurveChannel::Rgb => self.rgb = Some(p_curve),
      CurveChannel::Red => self.red = Some(p_curve),
      CurveChannel::Green => self.green = Some(p_curve),
      CurveChannel::Blue => self.blue = Some(p_curve),
    }
    self
  }

  /// Sets the curve of a channel to a smooth line through points, as in [`Curve::new`].
  /// - `p_channel`: The channel the curve is for.
  /// - `p_points`: The points `(input, output)` the curve goes through, each from `0` to `255`.
  pub fn with_points(self, p_channel: CurveChannel, p_points: impl IntoIterator<Item = impl Into<PointF>>) -> Self {
    self.with_curve(p_channel, Curve::new(p_points))
  }

  /// The lookup table of each of the red, green and blue channels.
  fn tables(&self) -> [[u8; 256]; 3] {
    [&self.red, &self.green, &self.blue].map(|own| {
      std::array::from_fn(|index| {
        let mut value = index as f32;
        if let Some(curve) = own {
          value = curve.value(value);
        }
        if let Some(curve) = &self.rgb {
          value = curve.value(value);
        }
        value.round().clamp(0.0, 255.0) as u8
      })
    })
  }
}

/// A curves adjustment that changes nothing until a curve is set for a channel with [`Curves::with_curve`].
pub fn curves() -> Curves {
  Curves {
    rgb: None,
    red: None,
    green: None,
    blue: None,
    options: None,
  }
}

impl GpuProcessor for Curves {
  /// A lookup table built from the CPU code, so the GPU gives exactly the CPU's pixels.
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![channel_lut_pass(|image| self.cpu_processor(image))]
  }
}

impl Effect for Curves {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    0
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let tables = self.tables();
    let colors = p_image.colors();
    let slice = colors.as_slice_mut().expect("Image colors must be contiguous");
    slice.par_chunks_mut(4096).for_each(|chunk| {
      for pixel in chunk.chunks_exact_mut(4) {
        for (channel, table) in pixel[..3].iter_mut().zip(&tables) {
          *channel = table[*channel as usize];
        }
      }
    });
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;

  fn run(p_curves: Curves, p_pixel: [u8; 4]) -> [u8; 4] {
    let mut image = Image::new_from_pixels(1, 1, p_pixel.to_vec(), Channels::RGBA);
    p_curves.apply_on_cpu(&mut image);
    let rgba = image.rgba();
    [rgba[0], rgba[1], rgba[2], rgba[3]]
  }

  #[test]
  fn no_curves_changes_nothing() {
    assert_eq!(run(curves(), [10, 120, 250, 77]), [10, 120, 250, 77]);
  }

  #[test]
  fn a_curve_of_the_straight_line_changes_nothing() {
    let effect = curves().with_curve(CurveChannel::Rgb, Curve::identity()).with_curve(CurveChannel::Blue, Curve::ease_in(1.0));
    assert_eq!(run(effect, [10, 120, 250, 77]), [10, 120, 250, 77]);
  }

  #[test]
  fn a_channel_curve_changes_only_its_own_channel_and_the_alpha_is_kept() {
    let effect = curves().with_points(CurveChannel::Red, [(0.0, 0.0), (128.0, 40.0), (255.0, 255.0)]);
    let [red, green, blue, alpha] = run(effect, [128, 128, 128, 99]);
    assert_eq!((red, green, blue, alpha), (40, 128, 128, 99));
  }

  #[test]
  fn the_rgb_curve_is_applied_to_every_channel_after_the_channel_curve() {
    // Red is made 100 by its own curve, then the Rgb curve halves everything.
    let effect = curves()
      .with_points(CurveChannel::Red, [(0.0, 0.0), (50.0, 100.0), (255.0, 255.0)])
      .with_points(CurveChannel::Rgb, [(0.0, 0.0), (100.0, 50.0), (255.0, 255.0)]);
    let [red, green, blue, _] = run(effect, [50, 100, 100, 255]);
    assert_eq!((red, green, blue), (50, 50, 50));
  }

  #[test]
  fn setting_a_channel_again_replaces_its_curve() {
    let effect = curves()
      .with_points(CurveChannel::Green, [(0.0, 0.0), (128.0, 10.0), (255.0, 255.0)])
      .with_points(CurveChannel::Green, [(0.0, 0.0), (128.0, 200.0), (255.0, 255.0)]);
    assert_eq!(run(effect, [0, 128, 0, 255])[1], 200);
  }

  #[test]
  fn a_curve_that_falls_makes_a_negative_of_the_picture() {
    let flipped = curves().with_curve(CurveChannel::Rgb, Curve::identity().inverted());
    assert_eq!(run(flipped, [10, 120, 250, 77]), [245, 135, 5, 77]);
    let by_points = curves().with_points(CurveChannel::Rgb, [(0, 255), (255, 0)]);
    assert_eq!(run(by_points, [10, 120, 250, 77]), [245, 135, 5, 77]);
  }

  #[test]
  fn an_s_curve_darkens_the_shadows_and_brightens_the_highlights() {
    let effect = curves().with_curve(CurveChannel::Rgb, Curve::ease_in_out(2.0));
    let [dark, _, bright, _] = run(effect, [64, 128, 192, 255]);
    assert!(dark < 64 && bright > 192, "{dark} {bright}");
  }

  #[test]
  fn the_gpu_gives_the_same_pixels_as_the_cpu() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let pixels: Vec<u8> = (0..65536u32).flat_map(|i| [i as u8, (i >> 8) as u8, (i * 37 % 256) as u8, (i * 13 % 256) as u8]).collect();
    let effect = curves()
      .with_curve(CurveChannel::Rgb, Curve::ease_in_out(2.0))
      .with_curve(CurveChannel::Red, Curve::ease_in_out(1.5))
      .with_curve(CurveChannel::Blue, Curve::ease_out_in(2.0));
    let actual = renderer.process(&[effect.passes(256, 256)], 256, 256, &pixels)?;
    let mut expected = Image::new_from_pixels(256, 256, pixels, Channels::RGBA);
    effect.apply_on_cpu(&mut expected);
    assert_eq!(actual, expected.to_rgba_vec());
    Ok(())
  }
}
