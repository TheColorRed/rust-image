use crate::{Image, Point, hsl_to_rgb, rgb_to_hsl};
use rayon::prelude::*;

/// A color with red, green, blue, and alpha channels.
pub type RGBA = (u8, u8, u8, u8);

/// A callback that computes the blend color from destination and source pixels.
pub type BlendMode = fn(RGBA, RGBA) -> RGBA;

/// A blend that has been described but not yet run. Create one with [`blend`], optionally set the offset, opacity,
/// and mode, then composite it onto a destination with [`BlendImage::apply`].
#[derive(Clone, Copy)]
pub struct BlendImage<'a> {
  /// The image composited onto the destination.
  pub source: &'a Image,
  /// Position of the source image in destination pixels.
  pub offset: Point,
  /// Opacity applied to the source alpha before alpha-over compositing.
  pub opacity: f32,
  /// Callback that computes the source blend color.
  pub mode: BlendMode,
}

impl<'a> BlendImage<'a> {
  /// Sets the position of the source in destination pixels. Defaults to `(0, 0)`.
  pub fn with_offset(mut self, p_offset: impl Into<Point>) -> Self {
    self.offset = p_offset.into();
    self
  }

  /// Sets the opacity applied to the source, clamped to `0.0..=1.0`. Defaults to `1.0`.
  pub fn with_opacity(mut self, p_opacity: f32) -> Self {
    self.opacity = p_opacity;
    self
  }

  /// Sets the blend mode, such as [`multiply`] or [`screen`]. Defaults to [`normal`].
  pub fn with_mode(mut self, p_mode: BlendMode) -> Self {
    self.mode = p_mode;
    self
  }

  /// Composites the source onto the destination. The source's alpha, multiplied by the opacity, is composited over
  /// the destination exactly once. Parts of the source outside the destination are ignored.
  pub fn apply(&self, p_destination: &mut Image) {
    let opacity = self.opacity.clamp(0.0, 1.0);
    let (destination_width, _) = p_destination.dimensions::<i32>();
    let (source_width, source_height) = self.source.dimensions::<i32>();
    let (offset_x, offset_y) = self.offset.dimensions();
    let mut pixels = p_destination.empty_pixel_vec();

    pixels.par_chunks_mut(4).enumerate().for_each(|(index, chunk)| {
      let x = index as i32 % destination_width;
      let y = index as i32 / destination_width;
      let destination = p_destination.get_pixel(x as u32, y as u32).expect("destination coordinates are in bounds");
      let source = if x >= offset_x && y >= offset_y && x < offset_x + source_width && y < offset_y + source_height {
        self.source.get_pixel((x - offset_x) as u32, (y - offset_y) as u32)
      } else {
        None
      };

      let color = match source {
        Some(source) => alpha_over(destination, (self.mode)(destination, source), source.3 as f32 / 255.0 * opacity),
        None => destination,
      };
      chunk.copy_from_slice(&[color.0, color.1, color.2, color.3]);
    });

    p_destination.set_rgba_owned(pixels);
  }
}

/// Composite a source image onto a destination image.
/// # Arguments
/// - `p_source`: The image to composite.
///
/// By default the source is placed at `(0, 0)` at full opacity with the [`normal`] blend mode. Change these with
/// [`BlendImage::with_offset`], [`BlendImage::with_opacity`], and [`BlendImage::with_mode`].
pub fn blend(p_source: &Image) -> BlendImage<'_> {
  BlendImage {
    source: p_source,
    offset: Point::default(),
    opacity: 1.0,
    mode: normal,
  }
}

/// Composites a blend-color source over a destination pixel using source alpha.
fn alpha_over(p_destination: RGBA, p_source_color: RGBA, p_source_alpha: f32) -> RGBA {
  let destination_alpha = p_destination.3 as f32 / 255.0;
  let output_alpha = p_source_alpha + destination_alpha * (1.0 - p_source_alpha);
  if output_alpha == 0.0 {
    return (0, 0, 0, 0);
  }

  let blend_channel = |destination: u8, source: u8| {
    ((source as f32 * p_source_alpha + destination as f32 * destination_alpha * (1.0 - p_source_alpha)) / output_alpha)
      .round() as u8
  };
  (
    blend_channel(p_destination.0, p_source_color.0),
    blend_channel(p_destination.1, p_source_color.1),
    blend_channel(p_destination.2, p_source_color.2),
    (output_alpha * 255.0).round() as u8,
  )
}

/// Edits or paints each pixel to make it the result color.
/// This is the default mode.
pub fn normal(_p_destination: RGBA, p_source: RGBA) -> RGBA {
  p_source
}

/// Looks at the color information in each channel and selects the base or blend color—whichever is darker—as the result color.
/// Pixels lighter than the blend color are replaced, and pixels darker than the blend color do not change.
pub fn darken(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = p_a.0.min(p_b.0);
  let green = p_a.1.min(p_b.1);
  let blue = p_a.2.min(p_b.2);
  let alpha = p_a.3.min(p_b.3);
  (red, green, blue, alpha)
}

/// Selects the darker of the two colors.
pub fn darker_color(p_a: RGBA, p_b: RGBA) -> RGBA {
  let lum_a = rgb_to_hsl(p_a.0, p_a.1, p_a.2).2;
  let lum_b = rgb_to_hsl(p_b.0, p_b.1, p_b.2).2;

  if lum_a < lum_b { p_a } else { p_b }
}

/// Selects the lighter of the two colors.
pub fn lighter_color(p_a: RGBA, p_b: RGBA) -> RGBA {
  let lum_a = rgb_to_hsl(p_a.0, p_a.1, p_a.2).2;
  let lum_b = rgb_to_hsl(p_b.0, p_b.1, p_b.2).2;

  if lum_a > lum_b { p_a } else { p_b }
}

/// Increases the lightness of the blend color to create a glowing effect.
pub fn glow(p_a: RGBA, p_b: RGBA) -> RGBA {
  let glow_channel = |p_a: u8, p_b: u8| -> u8 {
    if p_a == 255 { 255 } else { ((p_b as f32 * p_b as f32) / (255.0 - p_a as f32)).min(255.0) as u8 }
  };

  let red = glow_channel(p_a.0, p_b.0);
  let green = glow_channel(p_a.1, p_b.1);
  let blue = glow_channel(p_a.2, p_b.2);
  let alpha = glow_channel(p_a.3, p_b.3);

  (red, green, blue, alpha)
}

/// Creates a fiery, glowing effect similar to phoenix flames.
/// The formula creates intense highlights and deep shadows.
pub fn phoenix(p_a: RGBA, p_b: RGBA) -> RGBA {
  let phoenix_channel = |p_a: u8, p_b: u8| -> u8 {
    let a_f = p_a as f32;
    let b_f = p_b as f32;
    (a_f + b_f - 2.0 * a_f * b_f / 255.0).min(255.0).max(0.0) as u8
  };

  let red = phoenix_channel(p_a.0, p_b.0);
  let green = phoenix_channel(p_a.1, p_b.1);
  let blue = phoenix_channel(p_a.2, p_b.2);
  let alpha = p_a.3; // Preserve base layer alpha

  (red, green, blue, alpha)
}

/// Creates a negative film effect by inverting the color relationship.
/// Produces high-contrast, inverted color results.
pub fn negation(p_a: RGBA, p_b: RGBA) -> RGBA {
  let negation_channel =
    |p_a: u8, p_b: u8| -> u8 { (255.0 - (p_a as f32 - p_b as f32).abs()).min(255.0).max(0.0) as u8 };

  let red = negation_channel(p_a.0, p_b.0);
  let green = negation_channel(p_a.1, p_b.1);
  let blue = negation_channel(p_a.2, p_b.2);
  let alpha = p_a.3; // Preserve base layer alpha

  (red, green, blue, alpha)
}

/// Extracts grain from the base layer using the blend layer.
/// Useful for creating film grain effects and texture overlays.
pub fn grain_extract(p_a: RGBA, p_b: RGBA) -> RGBA {
  let grain_channel = |p_a: u8, p_b: u8| -> u8 { ((p_a as i32 + p_b as i32 - 255) / 2 + 128).clamp(0, 255) as u8 };

  let red = grain_channel(p_a.0, p_b.0);
  let green = grain_channel(p_a.1, p_b.1);
  let blue = grain_channel(p_a.2, p_b.2);
  let alpha = p_a.3; // Preserve base layer alpha

  (red, green, blue, alpha)
}

/// Merges grain patterns from both layers.
/// Creates complex texture effects by combining grain from base and blend layers.
pub fn grain_merge(p_a: RGBA, p_b: RGBA) -> RGBA {
  let grain_channel = |p_a: u8, p_b: u8| -> u8 { ((p_a as i32 + p_b as i32 - 128) / 2).clamp(0, 255) as u8 };

  let red = grain_channel(p_a.0, p_b.0);
  let green = grain_channel(p_a.1, p_b.1);
  let blue = grain_channel(p_a.2, p_b.2);
  let alpha = p_a.3; // Preserve base layer alpha

  (red, green, blue, alpha)
}

/// Averages the two colors.
pub fn average(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = (p_a.0 as i32 + p_b.0 as i32) / 2;
  let green = (p_a.1 as i32 + p_b.1 as i32) / 2;
  let blue = (p_a.2 as i32 + p_b.2 as i32) / 2;
  let alpha = (p_a.3 as i32 + p_b.3 as i32) / 2;
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and multiplies the base color by the blend color.
/// The result color is always a darker color.
/// Multiplying any color with black produces black.
/// Multiplying any color with white leaves the color unchanged.
/// When you’re painting with a color other than black or white, successive strokes with a painting tool produce progressively darker colors.
/// The effect is similar to drawing on the image with multiple marking pens.
pub fn multiply(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = (p_a.0 as i32 * p_b.0 as i32) / 255;
  let green = (p_a.1 as i32 * p_b.1 as i32) / 255;
  let blue = (p_a.2 as i32 * p_b.2 as i32) / 255;
  let alpha = (p_a.3 as i32 * p_b.3 as i32) / 255;
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and darkens the base color to reflect the blend color by increasing the contrast between the two.
/// Blending with white produces no change.
pub fn color_burn(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = if p_b.0 == 0 { 0.0 } else { 255.0 - ((255.0 - p_a.0 as f32) * 255.0 / p_b.0 as f32) };
  let green = if p_b.1 == 0 { 0.0 } else { 255.0 - ((255.0 - p_a.1 as f32) * 255.0 / p_b.1 as f32) };
  let blue = if p_b.2 == 0 { 0.0 } else { 255.0 - ((255.0 - p_a.2 as f32) * 255.0 / p_b.2 as f32) };
  let alpha = if p_b.3 == 0 { 0.0 } else { 255.0 - ((255.0 - p_a.3 as f32) * 255.0 / p_b.3 as f32) };
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and darkens the base color to reflect the blend color by decreasing the brightness.
/// Blending with white produces no change.
pub fn linear_burn(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = (p_a.0 as i32 + p_b.0 as i32 - 255).max(0);
  let green = (p_a.1 as i32 + p_b.1 as i32 - 255).max(0);
  let blue = (p_a.2 as i32 + p_b.2 as i32 - 255).max(0);
  let alpha = (p_a.3 as i32 + p_b.3 as i32 - 255).max(0);
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and selects the base or blend color—whichever is lighter—as the result color.
/// Pixels darker than the blend color are replaced, and pixels lighter than the blend color do not change.
pub fn lighten(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = p_a.0.max(p_b.0);
  let green = p_a.1.max(p_b.1);
  let blue = p_a.2.max(p_b.2);
  let alpha = p_a.3.max(p_b.3);
  (red, green, blue, alpha)
}

/// Looks at each channel’s color information and multiplies the inverse of the blend and base colors.
/// The result color is always a lighter color.
/// Screening with black leaves the color unchanged.
/// Screening with white produces white.
/// The effect is similar to projecting multiple photographic slides on top of each other.
pub fn screen(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = 255.0 - ((255.0 - p_a.0 as f32) * (255.0 - p_b.0 as f32) / 255.0);
  let green = 255.0 - ((255.0 - p_a.1 as f32) * (255.0 - p_b.1 as f32) / 255.0);
  let blue = 255.0 - ((255.0 - p_a.2 as f32) * (255.0 - p_b.2 as f32) / 255.0);
  let alpha = 255.0 - ((255.0 - p_a.3 as f32) * (255.0 - p_b.3 as f32) / 255.0);
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and brightens the base color to reflect the blend color by decreasing contrast between the two.
/// Blending with black produces no change.
pub fn color_dodge(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = if p_b.0 == 255 { 255.0 } else { (p_a.0 as f32 * 255.0 / (255.0 - p_b.0 as f32)).min(255.0) };
  let green = if p_b.1 == 255 { 255.0 } else { (p_a.1 as f32 * 255.0 / (255.0 - p_b.1 as f32)).min(255.0) };
  let blue = if p_b.2 == 255 { 255.0 } else { (p_a.2 as f32 * 255.0 / (255.0 - p_b.2 as f32)).min(255.0) };
  let alpha = if p_b.3 == 255 { 255.0 } else { (p_a.3 as f32 * 255.0 / (255.0 - p_b.3 as f32)).min(255.0) };
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and brightens the base color to reflect the blend color by increasing the brightness.
/// Blending with black produces no change.
pub fn linear_dodge(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = (p_a.0 as i32 + p_b.0 as i32).min(255);
  let green = (p_a.1 as i32 + p_b.1 as i32).min(255);
  let blue = (p_a.2 as i32 + p_b.2 as i32).min(255);
  let alpha = (p_a.3 as i32 + p_b.3 as i32).min(255);
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Multiplies or screens the colors, depending on the base color.
/// Patterns or colors overlay the existing pixels while preserving the highlights and shadows of the base color.
/// The base color is not replaced, but mixed with the blend color to reflect the lightness or darkness of the original color.
pub fn overlay(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = if p_a.0 < 128 {
    (2.0 * p_a.0 as f32 * p_b.0 as f32 / 255.0).round() as u8
  } else {
    (255.0 - 2.0 * (255.0 - p_a.0 as f32) * (255.0 - p_b.0 as f32) / 255.0).round() as u8
  };
  let green = if p_a.1 < 128 {
    (2.0 * p_a.1 as f32 * p_b.1 as f32 / 255.0).round() as u8
  } else {
    (255.0 - 2.0 * (255.0 - p_a.1 as f32) * (255.0 - p_b.1 as f32) / 255.0).round() as u8
  };
  let blue = if p_a.2 < 128 {
    (2.0 * p_a.2 as f32 * p_b.2 as f32 / 255.0).round() as u8
  } else {
    (255.0 - 2.0 * (255.0 - p_a.2 as f32) * (255.0 - p_b.2 as f32) / 255.0).round() as u8
  };
  let alpha = if p_a.3 < 128 {
    (2.0 * p_a.3 as f32 * p_b.3 as f32 / 255.0).round() as u8
  } else {
    (255.0 - 2.0 * (255.0 - p_a.3 as f32) * (255.0 - p_b.3 as f32) / 255.0).round() as u8
  };
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Darkens or lightens the colors, depending on the blend color.
/// The effect is similar to shining a diffused spotlight on the image.
/// If the blend color (light source) is lighter than 50% gray, the image is lightened as if it were dodged.
/// If the blend color is darker than 50% gray, the image is darkened as if it were burned in.
/// Painting with pure black or white produces a distinctly darker or lighter area, but does not result in pure black or white.
pub fn soft_light(p_a: RGBA, p_b: RGBA) -> RGBA {
  let blend_factor = 0.5; // Adjust this factor to control the blending intensity

  let red = if p_b.0 < 128 {
    ((2.0 * p_a.0 as f32 * p_b.0 as f32 / 255.0) * blend_factor + p_a.0 as f32 * (1.0 - blend_factor)) as u8
  } else {
    ((255.0 - 2.0 * (255.0 - p_a.0 as f32) * (255.0 - p_b.0 as f32) / 255.0) * blend_factor
      + p_a.0 as f32 * (1.0 - blend_factor)) as u8
  };
  let green = if p_b.1 < 128 {
    ((2.0 * p_a.1 as f32 * p_b.1 as f32 / 255.0) * blend_factor + p_a.1 as f32 * (1.0 - blend_factor)) as u8
  } else {
    ((255.0 - 2.0 * (255.0 - p_a.1 as f32) * (255.0 - p_b.1 as f32) / 255.0) * blend_factor
      + p_a.1 as f32 * (1.0 - blend_factor)) as u8
  };
  let blue = if p_b.2 < 128 {
    ((2.0 * p_a.2 as f32 * p_b.2 as f32 / 255.0) * blend_factor + p_a.2 as f32 * (1.0 - blend_factor)) as u8
  } else {
    ((255.0 - 2.0 * (255.0 - p_a.2 as f32) * (255.0 - p_b.2 as f32) / 255.0) * blend_factor
      + p_a.2 as f32 * (1.0 - blend_factor)) as u8
  };
  let alpha = if p_b.3 < 128 {
    ((2.0 * p_a.3 as f32 * p_b.3 as f32 / 255.0) * blend_factor + p_a.3 as f32 * (1.0 - blend_factor)) as u8
  } else {
    ((255.0 - 2.0 * (255.0 - p_a.3 as f32) * (255.0 - p_b.3 as f32) / 255.0) * blend_factor
      + p_a.3 as f32 * (1.0 - blend_factor)) as u8
  };

  (red, green, blue, alpha)
}

/// Multiplies or screens the colors, depending on the blend color.
/// The effect is similar to shining a harsh spotlight on the image.
/// If the blend color (light source) is lighter than 50% gray, the image is lightened, as if it were screened.
/// This is useful for adding highlights to an image.
/// If the blend color is darker than 50% gray, the image is darkened, as if it were multiplied.
/// This is useful for adding shadows to an image.
/// Painting with pure black or white results in pure black or white.
pub fn hard_light(p_a: RGBA, p_b: RGBA) -> RGBA {
  let blend_channel = |p_a: u8, p_b: u8| -> u8 {
    if p_b < 128 {
      (2.0 * p_a as f32 * p_b as f32 / 255.0) as u8
    } else {
      (255.0 - 2.0 * (255.0 - p_a as f32) * (255.0 - p_b as f32) / 255.0) as u8
    }
  };

  let red = blend_channel(p_a.0, p_b.0);
  let green = blend_channel(p_a.1, p_b.1);
  let blue = blend_channel(p_a.2, p_b.2);
  let alpha = blend_channel(p_a.3, p_b.3);

  (red, green, blue, alpha)
}

/// Burns or dodges the colors by increasing or decreasing the contrast, depending on the blend color.
/// If the blend color (light source) is lighter than 50% gray, the image is lightened by decreasing the contrast.
/// If the blend color is darker than 50% gray, the image is darkened by increasing the contrast.
pub fn vivid_light(p_a: RGBA, p_b: RGBA) -> RGBA {
  let b_burn =
    ((2.0 * p_b.0 as f32) as u8, (2.0 * p_b.1 as f32) as u8, (2.0 * p_b.2 as f32) as u8, (2.0 * p_b.3 as f32) as u8);
  let b_dodge = (
    (2.0 * (p_b.0 as f32 - 128.0)) as u8,
    (2.0 * (p_b.1 as f32 - 128.0)) as u8,
    (2.0 * (p_b.2 as f32 - 128.0)) as u8,
    (2.0 * (p_b.3 as f32 - 128.0)) as u8,
  );
  let red = if p_b.0 < 128 { color_burn(p_a, b_burn).0 } else { color_dodge(p_a, b_dodge).0 };
  let green = if p_b.1 < 128 { color_burn(p_a, b_burn).1 } else { color_dodge(p_a, b_dodge).1 };
  let blue = if p_b.2 < 128 { color_burn(p_a, b_burn).2 } else { color_dodge(p_a, b_dodge).2 };
  let alpha = if p_b.3 < 128 { color_burn(p_a, b_burn).3 } else { color_dodge(p_a, b_dodge).3 };
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Burns or dodges the colors by decreasing or increasing the brightness, depending on the blend color.
/// If the blend color (light source) is lighter than 50% gray, the image is lightened by increasing the brightness.
/// If the blend color is darker than 50% gray, the image is darkened by decreasing the brightness.
pub fn linear_light(p_a: RGBA, p_b: RGBA) -> RGBA {
  let b_burn =
    ((2.0 * p_b.0 as f32) as u8, (2.0 * p_b.1 as f32) as u8, (2.0 * p_b.2 as f32) as u8, (2.0 * p_b.3 as f32) as u8);
  let b_dodge = (
    (2.0 * (p_b.0 as f32 - 128.0)) as u8,
    (2.0 * (p_b.1 as f32 - 128.0)) as u8,
    (2.0 * (p_b.2 as f32 - 128.0)) as u8,
    (2.0 * (p_b.3 as f32 - 128.0)) as u8,
  );
  let red = if p_b.0 < 128 { linear_burn(p_a, b_burn).0 } else { linear_dodge(p_a, b_dodge).0 };
  let green = if p_b.1 < 128 { linear_burn(p_a, b_burn).1 } else { linear_dodge(p_a, b_dodge).1 };
  let blue = if p_b.2 < 128 { linear_burn(p_a, b_burn).2 } else { linear_dodge(p_a, b_dodge).2 };
  let alpha = if p_b.3 < 128 { linear_burn(p_a, b_burn).3 } else { linear_dodge(p_a, b_dodge).3 };
  (red, green, blue, alpha)
}

/// Replaces the colors, depending on the blend color.
/// If the blend color (light source) is lighter than 50% gray, pixels darker than the blend color are replaced, and pixels lighter than the blend color do not change.
/// If the blend color is darker than 50% gray, pixels lighter than the blend color are replaced, and pixels darker than the blend color do not change.
/// This is useful for adding special effects to an image.
pub fn pin_light(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = if p_b.0 < 128 { darken(p_a, p_b).0 } else { lighten(p_a, p_b).0 };
  let green = if p_b.1 < 128 { darken(p_a, p_b).1 } else { lighten(p_a, p_b).1 };
  let blue = if p_b.2 < 128 { darken(p_a, p_b).2 } else { lighten(p_a, p_b).2 };
  let alpha = if p_b.3 < 128 { darken(p_a, p_b).3 } else { lighten(p_a, p_b).3 };
  (red, green, blue, alpha)
}

/// Adds the red, green and blue channel values of the blend color to the RGB values of the base color.
/// If the resulting sum for a channel is 255 or greater, it receives a value of 255; if less than 255, a value of 0. Therefore, all blended pixels have red, green, and blue channel values of either 0 or 255.
/// This changes all pixels to primary additive colors (red, green, or blue), white, or black.
pub fn hard_mix(p_a: RGBA, p_b: RGBA) -> RGBA {
  let blended = vivid_light(p_a, p_b);
  let red = if blended.0 < 128 { 0 } else { 255 };
  let green = if blended.1 < 128 { 0 } else { 255 };
  let blue = if blended.2 < 128 { 0 } else { 255 };
  let alpha = if blended.3 < 128 { 0 } else { 255 };
  (red, green, blue, alpha)
}

/// Looks at the color information in each channel and subtracts either the blend
/// color from the base color or the base color from the blend color, depending on which has the greater brightness value.
/// Blending with white inverts the base color values; blending with black produces no change.
pub fn difference(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = (p_a.0 as i32 - p_b.0 as i32).abs() as u8;
  let green = (p_a.1 as i32 - p_b.1 as i32).abs() as u8;
  let blue = (p_a.2 as i32 - p_b.2 as i32).abs() as u8;
  (red, green, blue, p_a.3)
}

/// Creates an effect similar to but lower in contrast than the Difference mode.
/// Blending with white inverts the base color values.
/// Blending with black produces no change.
pub fn exclusion(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = p_a.0 as i32 + p_b.0 as i32 - 2 * p_a.0 as i32 * p_b.0 as i32 / 255;
  let green = p_a.1 as i32 + p_b.1 as i32 - 2 * p_a.1 as i32 * p_b.1 as i32 / 255;
  let blue = p_a.2 as i32 + p_b.2 as i32 - 2 * p_a.2 as i32 * p_b.2 as i32 / 255;
  (red as u8, green as u8, blue as u8, p_a.3)
}

/// Looks at the color information in each channel and subtracts the blend color from the base color.
/// In 8- and 16-bit images, any resulting negative values are clipped to zero.
pub fn subtract(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = (p_a.0 as i32 - p_b.0 as i32).max(0) as u8;
  let green = (p_a.1 as i32 - p_b.1 as i32).max(0) as u8;
  let blue = (p_a.2 as i32 - p_b.2 as i32).max(0) as u8;
  (red, green, blue, p_a.3)
}

/// Looks at the color information in each channel and divides the blend color from the base color.
/// If the blend color channel is zero, the result for that channel will be zero.
pub fn divide(p_a: RGBA, p_b: RGBA) -> RGBA {
  let red = if p_b.0 == 0 { 0 } else { (p_a.0 as f32 / p_b.0 as f32 * 255.0).round() as u8 };
  let green = if p_b.1 == 0 { 0 } else { (p_a.1 as f32 / p_b.1 as f32 * 255.0).round() as u8 };
  let blue = if p_b.2 == 0 { 0 } else { (p_a.2 as f32 / p_b.2 as f32 * 255.0).round() as u8 };
  (red, green, blue, p_a.3)
}

/// Creates a result color with the luminance and saturation of the base color and the hue of the blend color.
pub fn hue(p_a: RGBA, p_b: RGBA) -> RGBA {
  let (_, s1, l1) = rgb_to_hsl(p_a.0, p_a.1, p_a.2);
  let (h2, _, _) = rgb_to_hsl(p_b.0, p_b.1, p_b.2);
  let (r, g, p_b) = hsl_to_rgb(h2, s1, l1);
  (r, g, p_b, p_a.3)
}

/// Creates a result color with the luminance and hue of the base color and the saturation of the blend color.
///  Painting with this mode in an area with no (0) saturation (gray) causes no change.
pub fn saturation(p_a: RGBA, p_b: RGBA) -> RGBA {
  let (h1, _, l1) = rgb_to_hsl(p_a.0, p_a.1, p_a.2);
  let (_, s2, _) = rgb_to_hsl(p_b.0, p_b.1, p_b.2);
  let (r, g, p_b) = hsl_to_rgb(h1, s2, l1);
  (r, g, p_b, p_a.3)
}
/// Creates a result color with the luminance of the base color and the hue and saturation of the blend color.
/// This preserves the gray levels in the image and is useful for coloring monochrome images and for tinting color images.
pub fn color(p_a: RGBA, p_b: RGBA) -> RGBA {
  let (_, _, l1) = rgb_to_hsl(p_a.0, p_a.1, p_a.2);
  let (h2, s2, _) = rgb_to_hsl(p_b.0, p_b.1, p_b.2);
  let (r, g, p_b) = hsl_to_rgb(h2, s2, l1);
  (r, g, p_b, p_a.3)
}

/// Creates a result color with the hue and saturation of the base color and the luminance of the blend color.
/// This mode creates the inverse effect of Color mode.
pub fn luminosity(p_a: RGBA, p_b: RGBA) -> RGBA {
  let (h1, s1, _) = rgb_to_hsl(p_a.0, p_a.1, p_a.2);
  let (_, _, l2) = rgb_to_hsl(p_b.0, p_b.1, p_b.2);
  let (r, g, p_b) = hsl_to_rgb(h1, s1, l2);
  (r, g, p_b, p_a.3)
}

/// Reflects the blend color over the base color, creating a shiny, metallic effect.
/// The formula is base^2 / (1 - blend), which amplifies bright areas.
pub fn reflect(p_a: RGBA, p_b: RGBA) -> RGBA {
  let reflect_channel = |p_a: u8, p_b: u8| -> u8 {
    if p_b == 255 { 255 } else { ((p_a as f32 * p_a as f32) / (255.0 - p_b as f32)).min(255.0) as u8 }
  };

  let red = reflect_channel(p_a.0, p_b.0);
  let green = reflect_channel(p_a.1, p_b.1);
  let blue = reflect_channel(p_a.2, p_b.2);
  let alpha = reflect_channel(p_a.3, p_b.3);

  (red, green, blue, alpha)
}

/// Returns the name of the blend mode function.
#[allow(unpredictable_function_pointer_comparisons)]
pub fn blend_mode_name(p_mode: fn(RGBA, RGBA) -> RGBA) -> (&'static str, &'static str) {
  match () {
    _ if p_mode == normal => ("normal", "Normal"),
    _ if p_mode == darken => ("darken", "Darken"),
    _ if p_mode == darker_color => ("darker-color", "Darker Color"),
    _ if p_mode == average => ("average", "Average"),
    _ if p_mode == multiply => ("multiply", "Multiply"),
    _ if p_mode == color_burn => ("color-burn", "Color Burn"),
    _ if p_mode == linear_burn => ("linear-burn", "Linear Burn"),
    _ if p_mode == lighten => ("lighten", "Lighten"),
    _ if p_mode == lighter_color => ("lighter-color", "Lighter Color"),
    _ if p_mode == screen => ("screen", "Screen"),
    _ if p_mode == color_dodge => ("color-dodge", "Color Dodge"),
    _ if p_mode == linear_dodge => ("linear-dodge", "Linear Dodge"),
    _ if p_mode == overlay => ("overlay", "Overlay"),
    _ if p_mode == soft_light => ("soft-light", "Soft Light"),
    _ if p_mode == hard_light => ("hard-light", "Hard Light"),
    _ if p_mode == vivid_light => ("vivid-light", "Vivid Light"),
    _ if p_mode == linear_light => ("linear-light", "Linear Light"),
    _ if p_mode == pin_light => ("pin-light", "Pin Light"),
    _ if p_mode == hard_mix => ("hard-mix", "Hard Mix"),
    _ if p_mode == difference => ("difference", "Difference"),
    _ if p_mode == exclusion => ("exclusion", "Exclusion"),
    _ if p_mode == subtract => ("subtract", "Subtract"),
    _ if p_mode == divide => ("divide", "Divide"),
    _ if p_mode == hue => ("hue", "Hue"),
    _ if p_mode == saturation => ("saturation", "Saturation"),
    _ if p_mode == color => ("color", "Color"),
    _ if p_mode == luminosity => ("luminosity", "Luminosity"),
    _ if p_mode == reflect => ("reflect", "Reflect"),
    _ if p_mode == glow => ("glow", "Glow"),
    _ if p_mode == phoenix => ("phoenix", "Phoenix"),
    _ if p_mode == negation => ("negation", "Negation"),
    _ if p_mode == grain_extract => ("grain-extract", "Grain Extract"),
    _ if p_mode == grain_merge => ("grain-merge", "Grain Merge"),
    _ => ("unknown", "Unknown"),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn image(p_width: u32, p_height: u32, p_pixels: &[u8]) -> Image {
    Image::from_rgba_bytes(p_width, p_height, p_pixels)
  }

  #[test]
  fn blends_source_at_its_offset() {
    let mut destination = image(3, 1, &[0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]);
    let source = image(1, 1, &[255, 255, 0, 255]);

    blend(&source).with_offset(Point::new(1, 0)).apply(&mut destination);

    assert_eq!(destination.rgba(), &[0, 0, 0, 255, 255, 255, 0, 255, 0, 0, 0, 255]);
  }

  #[test]
  fn blends_source_alpha_and_opacity_once() {
    let mut destination = image(1, 1, &[255, 0, 0, 255]);
    let source = image(1, 1, &[0, 0, 255, 128]);

    blend(&source).with_opacity(0.5).apply(&mut destination);

    assert_eq!(destination.rgba(), &[191, 0, 64, 255]);
  }

  #[test]
  fn transparent_source_leaves_destination_unchanged() {
    let mut destination = image(1, 1, &[12, 34, 56, 200]);
    let source = image(1, 1, &[255, 0, 0, 0]);

    blend(&source).apply(&mut destination);

    assert_eq!(destination.rgba(), &[12, 34, 56, 200]);
  }

  #[test]
  fn transparent_destination_receives_source_opacity() {
    let mut destination = image(1, 1, &[0, 0, 0, 0]);
    let source = image(1, 1, &[0, 255, 0, 128]);

    blend(&source).with_opacity(0.5).apply(&mut destination);

    assert_eq!(destination.rgba(), &[0, 255, 0, 64]);
  }
}
