//! Blending one image over another, from `abra-core`: `BlendMode`, the `BlendImage` builder that `blend` starts, and
//! `blended`.
//!
//! `BlendMode::Custom` holds a Rust function, which cannot cross to other languages, so it is not here. `BlendMode::ALL` is
//! `blend_modes()` and `BlendMode::from_name` is `blend_mode_from_name`.
//!
//! `BlendImage::apply` blends into the image it is given. When the GPU can do it (the images are the same size, the source is
//! not offset, and the opacity is not zero) the result is an image whose pixels are made by the GPU when they are first read,
//! the same pixels as the library's CPU blend (see `abra_core::image::recipe`); otherwise it blends on the CPU at once.

use std::sync::Arc;

use abra::abra_core::{self, PointF};

use crate::Image;

// A point in 2D space with floating-point coordinates.
#[uniffi::remote(Record)]
pub struct PointF {
  pub x: f32,
  pub y: f32,
}

/// How a source color combines with the destination color beneath it, before the source's alpha composites the result over
/// the destination. The names and behavior follow the common image-editor blend modes. Same order as the library's list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum BlendMode {
  /// The source color replaces the destination.
  Normal,
  /// The darker of each channel.
  Darken,
  /// The darker of the two colors, by lightness.
  DarkerColor,
  /// The mean of each channel.
  Average,
  /// Multiplies the channels. Always darker.
  Multiply,
  /// Darkens the destination by increasing contrast toward the source.
  ColorBurn,
  /// Darkens the destination by decreasing brightness.
  LinearBurn,
  /// The lighter of each channel.
  Lighten,
  /// The lighter of the two colors, by lightness.
  LighterColor,
  /// Inverts, multiplies and inverts again. Always lighter.
  Screen,
  /// Brightens the destination by decreasing contrast toward the source.
  ColorDodge,
  /// Brightens the destination by increasing brightness.
  LinearDodge,
  /// Multiplies or screens, by the destination.
  Overlay,
  /// A softer overlay, by the source.
  SoftLight,
  /// Multiplies or screens, by the source.
  HardLight,
  /// Burns or dodges, by the source.
  VividLight,
  /// Linear burn or linear dodge, by the source.
  LinearLight,
  /// Darkens or lightens, by the source.
  PinLight,
  /// Pushes each channel to 0 or 255.
  HardMix,
  /// The difference of each channel.
  Difference,
  /// A softer difference.
  Exclusion,
  /// Subtracts the source from the destination.
  Subtract,
  /// Divides the destination by the source.
  Divide,
  /// The source's hue on the destination's saturation and lightness.
  Hue,
  /// The source's saturation on the destination's hue and lightness.
  Saturation,
  /// The source's hue and saturation on the destination's lightness.
  Color,
  /// The source's lightness on the destination's hue and saturation.
  Luminosity,
  /// Reflects the destination by the source.
  Reflect,
  /// Reflect with the destination and the source swapped.
  Glow,
  /// Like exclusion, with the result clamped.
  Phoenix,
  /// Inverts the difference.
  Negation,
  /// Extracts film grain from the destination.
  GrainExtract,
  /// Merges grain from both layers.
  GrainMerge,
}

impl BlendMode {
  /// The library's mode with this one's place in its list.
  pub fn to_core(self) -> abra_core::BlendMode {
    abra_core::BlendMode::ALL[self as usize]
  }
}

impl From<abra_core::BlendMode> for BlendMode {
  fn from(mode: abra_core::BlendMode) -> Self {
    blend_modes().into_iter().find(|m| m.to_core() == mode).unwrap_or(BlendMode::Normal)
  }
}

#[uniffi::export]
impl BlendMode {
  /// The mode's identifier, such as "color-burn".
  pub fn name(&self) -> String {
    self.to_core().name().to_string()
  }

  /// The mode's display label, such as "Color Burn".
  pub fn label(&self) -> String {
    self.to_core().label().to_string()
  }

  /// The mode's number for the blend shader, its place in the library's list.
  pub fn shader_index(&self) -> Option<u32> {
    self.to_core().shader_index()
  }

  /// Blends a source color over a destination color, each `[r, g, b, a]`, returning the blend color as `[r, g, b, a]`. The
  /// source's alpha is applied afterwards.
  pub fn apply(&self, destination: Vec<u8>, source: Vec<u8>) -> Vec<u8> {
    let [dr, dg, db, da]: [u8; 4] = destination.try_into().expect("a destination color takes 4 values");
    let [sr, sg, sb, sa]: [u8; 4] = source.try_into().expect("a source color takes 4 values");
    let (r, g, b, a) = self.to_core().apply((dr, dg, db, da), (sr, sg, sb, sa));
    vec![r, g, b, a]
  }
}

/// Every built-in blend mode, in menu order.
#[uniffi::export]
pub fn blend_modes() -> Vec<BlendMode> {
  use BlendMode::*;
  vec![
    Normal,
    Darken,
    DarkerColor,
    Average,
    Multiply,
    ColorBurn,
    LinearBurn,
    Lighten,
    LighterColor,
    Screen,
    ColorDodge,
    LinearDodge,
    Overlay,
    SoftLight,
    HardLight,
    VividLight,
    LinearLight,
    PinLight,
    HardMix,
    Difference,
    Exclusion,
    Subtract,
    Divide,
    Hue,
    Saturation,
    Color,
    Luminosity,
    Reflect,
    Glow,
    Phoenix,
    Negation,
    GrainExtract,
    GrainMerge,
  ]
}

/// The built-in mode with this name (`BlendMode::name`), or nothing if there is none.
#[uniffi::export]
pub fn blend_mode_from_name(name: String) -> Option<BlendMode> {
  let mode = abra_core::BlendMode::from_name(&name)?;
  blend_modes().into_iter().find(|candidate| candidate.to_core() == mode)
}

/// A blend that has been described but not yet run. Start one with [`blend`], optionally set the offset, opacity and mode,
/// then composite it onto a destination with [`apply`](Self::apply).
#[derive(Debug, uniffi::Object)]
pub struct BlendImage {
  source: Arc<Image>,
  offset: PointF,
  opacity: f32,
  mode: BlendMode,
}

#[uniffi::export]
impl BlendImage {
  /// The same blend with the source placed at this position in the destination. Defaults to (0, 0).
  pub fn with_offset(&self, offset: PointF) -> Arc<BlendImage> {
    Arc::new(BlendImage {
      source: self.source.clone(),
      offset,
      opacity: self.opacity,
      mode: self.mode,
    })
  }

  /// The same blend with this opacity applied to the source, 0 to 1. Defaults to 1.
  pub fn with_opacity(&self, opacity: f32) -> Arc<BlendImage> {
    Arc::new(BlendImage {
      source: self.source.clone(),
      offset: self.offset,
      opacity,
      mode: self.mode,
    })
  }

  /// The same blend with this mode. Defaults to normal.
  pub fn with_mode(&self, mode: BlendMode) -> Arc<BlendImage> {
    Arc::new(BlendImage {
      source: self.source.clone(),
      offset: self.offset,
      opacity: self.opacity,
      mode,
    })
  }

  /// Composites the source onto the destination, which is changed.
  pub fn apply(&self, destination: Arc<Image>) {
    // The source is copied first, so blending an image onto itself does not wait on its own lock.
    let source = self.source.core();
    let mode = self.mode.to_core();
    destination.edit(|image| {
      if self.offset == (PointF { x: 0.0, y: 0.0 }) {
        // Not offset: the GPU can do it when it is there, and `blended` blends on the CPU when it is not.
        *image = abra_core::image::recipe::blended(image, &source, mode, self.opacity);
      } else {
        abra_core::blend::blend(&source)
          .with_offset(self.offset)
          .with_opacity(self.opacity)
          .with_mode(mode)
          .apply(image);
      }
    });
  }
}

/// Starts a blend of `source` over another image. Defaults to the normal mode at full opacity with no offset.
#[uniffi::export]
pub fn blend(source: Arc<Image>) -> Arc<BlendImage> {
  Arc::new(BlendImage {
    source,
    offset: PointF { x: 0.0, y: 0.0 },
    opacity: 1.0,
    mode: BlendMode::Normal,
  })
}

/// `top` blended over `base` as a new image, with `opacity` from 0 to 1 on top of the top's own alpha. Neither is changed. On a
/// GPU the result is made only when its pixels are read.
#[uniffi::export]
pub fn blended(base: Arc<Image>, top: Arc<Image>, mode: BlendMode, opacity: f32) -> Arc<Image> {
  Arc::new(Image::from_core(abra_core::image::recipe::blended(&base.core(), &top.core(), mode.to_core(), opacity)))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{Color, ColorStop, Gradient, radial_gradient};

  fn color(r: u8, g: u8, b: u8, a: u8) -> Arc<Color> {
    Arc::new(Color::from_rgba(r, g, b, a))
  }

  #[test]
  fn the_vignette_a_tool_writes_darkens_the_corners_and_keeps_the_center() {
    let photo = Arc::new(Image::new_from_color(101, 101, color(200, 200, 200, 255)));
    let clear = color(0, 0, 0, 0);
    let gradient = Arc::new(Gradient::new(vec![
      Arc::new(ColorStop::new(clear.clone(), 0.0)),
      Arc::new(ColorStop::new(clear, 0.5)),
      Arc::new(ColorStop::new(color(0, 0, 0, 255), 1.0)),
    ]));
    let reach = 50.5 * std::f32::consts::SQRT_2;
    let edges = radial_gradient(101, 101, gradient, 50.5, 50.5, reach, reach);
    // As a tool writes it: blend the edges onto the photo, which is changed in place.
    blend(edges).apply(photo.clone());
    assert_eq!(photo.get_pixel(50, 50), Some(vec![200, 200, 200, 255]), "the middle is untouched");
    assert!(photo.get_pixel(0, 0).unwrap()[0] < 30, "the corner is nearly black");
  }

  #[test]
  fn the_builder_sets_the_mode_and_opacity() {
    let base = Arc::new(Image::new_from_color(2, 2, color(100, 100, 100, 255)));
    let top = Arc::new(Image::new_from_color(2, 2, color(200, 200, 200, 255)));
    let multiplied = Arc::new(Image::from_core(base.core()));
    blend(top.clone()).with_mode(BlendMode::Multiply).apply(multiplied.clone());
    // 100 * 200 / 255, as the library's multiply does it.
    assert_eq!(multiplied.get_pixel(0, 0), Some(vec![78, 78, 78, 255]));
    let half = Arc::new(Image::from_core(base.core()));
    blend(top).with_opacity(0.5).apply(half.clone());
    assert_eq!(half.get_pixel(0, 0).unwrap()[0], 150);
  }

  #[test]
  fn an_offset_blend_touches_only_the_pixels_under_the_source() {
    let base = Arc::new(Image::new_from_color(3, 3, color(0, 0, 0, 255)));
    let top = Arc::new(Image::new_from_color(1, 1, color(200, 0, 0, 255)));
    let moved = Arc::new(Image::from_core(base.core()));
    blend(top).with_offset(PointF { x: 2.0, y: 1.0 }).apply(moved.clone());
    assert_eq!(moved.get_pixel(2, 1).unwrap()[0], 200);
    assert_eq!(moved.get_pixel(0, 0).unwrap()[0], 0);
  }

  #[test]
  fn blended_makes_a_new_image_and_leaves_both_alone() {
    let base = Arc::new(Image::new_from_color(2, 2, color(100, 100, 100, 255)));
    let top = Arc::new(Image::new_from_color(2, 2, color(200, 200, 200, 255)));
    let made = blended(base.clone(), top, BlendMode::Normal, 1.0);
    assert_eq!(made.get_pixel(1, 1).unwrap()[0], 200);
    assert_eq!(base.get_pixel(1, 1).unwrap()[0], 100);
  }

  #[test]
  fn blending_an_image_onto_itself_does_not_lock_up() {
    let image = Arc::new(Image::new_from_color(2, 2, color(100, 100, 100, 255)));
    blend(image.clone()).with_mode(BlendMode::Multiply).apply(image.clone());
    assert_eq!(image.get_pixel(0, 0).unwrap()[0], 39);
  }

  #[test]
  fn the_modes_are_those_of_the_library_in_its_order() {
    let modes = blend_modes();
    assert_eq!(modes.len(), abra_core::BlendMode::ALL.len());
    for (index, mode) in modes.iter().enumerate() {
      assert_eq!(mode.to_core(), abra_core::BlendMode::ALL[index]);
      assert_eq!(*mode as usize, index);
      assert_eq!(mode.shader_index(), Some(index as u32));
      // "ColorBurn" is "color-burn", and so on.
      assert_eq!(format!("{mode:?}").to_lowercase(), mode.name().replace('-', ""));
      assert_eq!(blend_mode_from_name(mode.name()), Some(*mode));
    }
    assert_eq!(BlendMode::ColorBurn.label(), "Color Burn");
    assert_eq!(blend_mode_from_name("nope".to_string()), None);
  }

  #[test]
  fn a_mode_blends_two_colors() {
    let dark = vec![100, 100, 100, 255];
    let light = vec![200, 200, 200, 255];
    assert_eq!(BlendMode::Darken.apply(dark.clone(), light.clone())[0], 100);
    assert_eq!(BlendMode::Lighten.apply(dark, light)[0], 200);
  }

  #[test]
  #[should_panic(expected = "a source color takes 4 values")]
  fn a_color_of_the_wrong_length_panics() {
    BlendMode::Darken.apply(vec![1, 2, 3, 4], vec![1, 2, 3]);
  }
}
