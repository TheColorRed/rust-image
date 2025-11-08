use crate::{
  color::{to_hsl::rgb_to_hsl, to_rgb::hsl_to_rgb},
  image::Image,
};
use rayon::prelude::*;

/// A color with red, green, blue, and alpha channels.
pub type RGBA = (u8, u8, u8, u8);

/// Combine two images using a blend mode at the given position.
pub fn blend_images_at(bottom_image: &mut Image, top_image: &Image, x1: i32, y1: i32, x2: i32, y2: i32, mode: fn(RGBA, RGBA) -> RGBA) {
  // get the width of the image including the x position
  let (bottom_width, bottom_height) = bottom_image.dimensions::<i32>();
  let (top_width, top_height) = top_image.dimensions::<i32>();
  let mut pixels = bottom_image.empty_pixel_vec();

  pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = (i as i32) % bottom_width;
    let y = (i as i32) / bottom_width;

    let a = if x >= x1 && y >= y1 && x < x1 + bottom_width && y < y1 + bottom_height {
      bottom_image.get_pixel((x - x1) as u32, (y - y1) as u32)
    } else {
      None
    };
    let b = if x >= x2 && y >= y2 && x < x2 + top_width && y < y2 + top_height {
      top_image.get_pixel((x - x2) as u32, (y - y2) as u32)
    } else {
      None
    };

    let color = match (a, b) {
      (Some(a), Some(b)) => mode(a, b),
      (Some(a), None) => a,
      (None, Some(b)) => b,
      (None, None) => (0, 0, 0, 0),
    };

    chunk[0] = color.0;
    chunk[1] = color.1;
    chunk[2] = color.2;
    chunk[3] = color.3;
  });

  bottom_image.set_rgba(pixels);
}

/// Combines two images using the given blend mode.
/// This blends the images at (0, 0) and returns the result.
pub fn blend_images(bottom_image: &mut Image, top_image: &Image, mode: fn(RGBA, RGBA) -> RGBA) {
  blend_images_at(bottom_image, top_image, 0, 0, 0, 0, mode);
}

/// Combine two images using a blend mode at the given position with opacity support.
/// The opacity is applied during blending, not to the layer data.
pub fn blend_images_at_with_opacity(
  bottom_image: &mut Image,
  top_image: &Image,
  _x1: i32,
  _y1: i32,
  x2: i32,
  y2: i32,
  mode: fn(RGBA, RGBA) -> RGBA,
  opacity: f32,
) {
  let opacity = opacity.clamp(0.0, 1.0);

  // get the width of the image including the x position
  let (bottom_width, _bottom_height) = bottom_image.dimensions::<i32>();
  let (top_width, top_height) = top_image.dimensions::<i32>();
  let mut pixels = bottom_image.empty_pixel_vec();

  pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
    let x = (i as i32) % bottom_width;
    let y = (i as i32) / bottom_width;

    // Bottom image (canvas) pixel - always valid since x,y are within canvas bounds
    let a = bottom_image.get_pixel(x as u32, y as u32);

    // Top image (layer) pixel - check if this canvas position contains part of the layer
    let b = if x >= x2 && y >= y2 && x < x2 + top_width && y < y2 + top_height {
      top_image.get_pixel((x - x2) as u32, (y - y2) as u32)
    } else {
      None
    };

    let color = match (a, b) {
      (Some(a), Some(b)) => {
        // Apply blend mode first
        let blended = mode(a, b);

        // Then apply opacity to the result's alpha channel
        let (r, g, b, alpha) = blended;
        let new_alpha = (alpha as f32 * opacity) as u8;

        // Composite the opacity-adjusted blended layer onto the base layer
        // This implements proper alpha compositing
        let a_r = a.0 as f32 / 255.0;
        let a_g = a.1 as f32 / 255.0;
        let a_b = a.2 as f32 / 255.0;
        let a_a = a.3 as f32 / 255.0;

        let b_r = r as f32 / 255.0;
        let b_g = g as f32 / 255.0;
        let b_b = b as f32 / 255.0;
        let b_a = new_alpha as f32 / 255.0;

        let out_a = b_a + a_a * (1.0 - b_a);
        let out_r = if out_a > 0.0 {
          (b_r * b_a + a_r * a_a * (1.0 - b_a)) / out_a
        } else {
          0.0
        };
        let out_g = if out_a > 0.0 {
          (b_g * b_a + a_g * a_a * (1.0 - b_a)) / out_a
        } else {
          0.0
        };
        let out_b = if out_a > 0.0 {
          (b_b * b_a + a_b * a_a * (1.0 - b_a)) / out_a
        } else {
          0.0
        };

        (
          (out_r * 255.0) as u8,
          (out_g * 255.0) as u8,
          (out_b * 255.0) as u8,
          (out_a * 255.0) as u8,
        )
      }
      (Some(a), None) => a,
      (None, Some(b)) => {
        // Apply opacity to the top layer when bottom layer is transparent
        let (r, g, b, alpha) = b;
        let new_alpha = (alpha as f32 * opacity) as u8;
        (r, g, b, new_alpha)
      }
      (None, None) => (0, 0, 0, 0),
    };

    chunk[0] = color.0;
    chunk[1] = color.1;
    chunk[2] = color.2;
    chunk[3] = color.3;
  });

  bottom_image.set_rgba(pixels);
}

/// Edits or paints each pixel to make it the result color.
/// This is the default mode.
pub fn normal(a: RGBA, b: RGBA) -> RGBA {
  let r1 = a.0 as f32 / 255.0;
  let g1 = a.1 as f32 / 255.0;
  let b1 = a.2 as f32 / 255.0;
  let a1 = a.3 as f32 / 255.0;

  let r2 = b.0 as f32 / 255.0;
  let g2 = b.1 as f32 / 255.0;
  let b2 = b.2 as f32 / 255.0;
  let a2 = b.3 as f32 / 255.0;

  let red = (r2 * a2 + r1 * a1 * (1.0 - a2)) * 255.0;
  let green = (g2 * a2 + g1 * a1 * (1.0 - a2)) * 255.0;
  let blue = (b2 * a2 + b1 * a1 * (1.0 - a2)) * 255.0;
  let alpha = (a2 + a1 * (1.0 - a2)) * 255.0;

  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and selects the base or blend color—whichever is darker—as the result color.
/// Pixels lighter than the blend color are replaced, and pixels darker than the blend color do not change.
pub fn darken(a: RGBA, b: RGBA) -> RGBA {
  let red = a.0.min(b.0);
  let green = a.1.min(b.1);
  let blue = a.2.min(b.2);
  let alpha = a.3.min(b.3);
  (red, green, blue, alpha)
}

/// Averages the two colors.
pub fn average(a: RGBA, b: RGBA) -> RGBA {
  let red = (a.0 as i32 + b.0 as i32) / 2;
  let green = (a.1 as i32 + b.1 as i32) / 2;
  let blue = (a.2 as i32 + b.2 as i32) / 2;
  let alpha = (a.3 as i32 + b.3 as i32) / 2;
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and multiplies the base color by the blend color.
/// The result color is always a darker color.
/// Multiplying any color with black produces black.
/// Multiplying any color with white leaves the color unchanged.
/// When you’re painting with a color other than black or white, successive strokes with a painting tool produce progressively darker colors.
/// The effect is similar to drawing on the image with multiple marking pens.
pub fn multiply(a: RGBA, b: RGBA) -> RGBA {
  let red = (a.0 as i32 * b.0 as i32) / 255;
  let green = (a.1 as i32 * b.1 as i32) / 255;
  let blue = (a.2 as i32 * b.2 as i32) / 255;
  let alpha = (a.3 as i32 * b.3 as i32) / 255;
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and darkens the base color to reflect the blend color by increasing the contrast between the two.
/// Blending with white produces no change.
pub fn color_burn(a: RGBA, b: RGBA) -> RGBA {
  let red = if b.0 == 0 {
    0.0
  } else {
    255.0 - ((255.0 - a.0 as f32) * 255.0 / b.0 as f32)
  };
  let green = if b.1 == 0 {
    0.0
  } else {
    255.0 - ((255.0 - a.1 as f32) * 255.0 / b.1 as f32)
  };
  let blue = if b.2 == 0 {
    0.0
  } else {
    255.0 - ((255.0 - a.2 as f32) * 255.0 / b.2 as f32)
  };
  let alpha = if b.3 == 0 {
    0.0
  } else {
    255.0 - ((255.0 - a.3 as f32) * 255.0 / b.3 as f32)
  };
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and darkens the base color to reflect the blend color by decreasing the brightness.
/// Blending with white produces no change.
pub fn linear_burn(a: RGBA, b: RGBA) -> RGBA {
  let red = if b.0 == 0 {
    0.0
  } else {
    255.0 - ((255.0 - a.0 as f32) * 255.0 / b.0 as f32)
  };
  let green = if b.1 == 0 {
    0.0
  } else {
    255.0 - ((255.0 - a.1 as f32) * 255.0 / b.1 as f32)
  };
  let blue = if b.2 == 0 {
    0.0
  } else {
    255.0 - ((255.0 - a.2 as f32) * 255.0 / b.2 as f32)
  };
  let alpha = if b.3 == 0 {
    0.0
  } else {
    255.0 - ((255.0 - a.3 as f32) * 255.0 / b.3 as f32)
  };
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and selects the base or blend color—whichever is lighter—as the result color.
/// Pixels darker than the blend color are replaced, and pixels lighter than the blend color do not change.
pub fn lighten(a: RGBA, b: RGBA) -> RGBA {
  let red = a.0.max(b.0);
  let green = a.1.max(b.1);
  let blue = a.2.max(b.2);
  let alpha = a.3.max(b.3);
  (red, green, blue, alpha)
}

/// Looks at each channel’s color information and multiplies the inverse of the blend and base colors.
/// The result color is always a lighter color.
/// Screening with black leaves the color unchanged.
/// Screening with white produces white.
/// The effect is similar to projecting multiple photographic slides on top of each other.
pub fn screen(a: RGBA, b: RGBA) -> RGBA {
  let red = 255.0 - ((255.0 - a.0 as f32) * (255.0 - b.0 as f32) / 255.0);
  let green = 255.0 - ((255.0 - a.1 as f32) * (255.0 - b.1 as f32) / 255.0);
  let blue = 255.0 - ((255.0 - a.2 as f32) * (255.0 - b.2 as f32) / 255.0);
  let alpha = 255.0 - ((255.0 - a.3 as f32) * (255.0 - b.3 as f32) / 255.0);
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and brightens the base color to reflect the blend color by decreasing contrast between the two.
/// Blending with black produces no change.
pub fn color_dodge(a: RGBA, b: RGBA) -> RGBA {
  let red = if b.0 == 255 {
    255.0
  } else {
    (a.0 as f32 * 255.0 / (255.0 - b.0 as f32)).min(255.0)
  };
  let green = if b.1 == 255 {
    255.0
  } else {
    (a.1 as f32 * 255.0 / (255.0 - b.1 as f32)).min(255.0)
  };
  let blue = if b.2 == 255 {
    255.0
  } else {
    (a.2 as f32 * 255.0 / (255.0 - b.2 as f32)).min(255.0)
  };
  let alpha = if b.3 == 255 {
    255.0
  } else {
    (a.3 as f32 * 255.0 / (255.0 - b.3 as f32)).min(255.0)
  };
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Looks at the color information in each channel and brightens the base color to reflect the blend color by increasing the brightness.
/// Blending with black produces no change.
pub fn linear_dodge(a: RGBA, b: RGBA) -> RGBA {
  let red = (a.0 as i32 + b.0 as i32).min(255);
  let green = (a.1 as i32 + b.1 as i32).min(255);
  let blue = (a.2 as i32 + b.2 as i32).min(255);
  let alpha = (a.3 as i32 + b.3 as i32).min(255);
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Multiplies or screens the colors, depending on the base color.
/// Patterns or colors overlay the existing pixels while preserving the highlights and shadows of the base color.
/// The base color is not replaced, but mixed with the blend color to reflect the lightness or darkness of the original color.
pub fn overlay(a: RGBA, b: RGBA) -> RGBA {
  let red = if a.0 < 128 {
    (2.0 * a.0 as f32 * b.0 as f32 / 255.0).round() as u8
  } else {
    (255.0 - 2.0 * (255.0 - a.0 as f32) * (255.0 - b.0 as f32) / 255.0).round() as u8
  };
  let green = if a.1 < 128 {
    (2.0 * a.1 as f32 * b.1 as f32 / 255.0).round() as u8
  } else {
    (255.0 - 2.0 * (255.0 - a.1 as f32) * (255.0 - b.1 as f32) / 255.0).round() as u8
  };
  let blue = if a.2 < 128 {
    (2.0 * a.2 as f32 * b.2 as f32 / 255.0).round() as u8
  } else {
    (255.0 - 2.0 * (255.0 - a.2 as f32) * (255.0 - b.2 as f32) / 255.0).round() as u8
  };
  let alpha = if a.3 < 128 {
    (2.0 * a.3 as f32 * b.3 as f32 / 255.0).round() as u8
  } else {
    (255.0 - 2.0 * (255.0 - a.3 as f32) * (255.0 - b.3 as f32) / 255.0).round() as u8
  };
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Darkens or lightens the colors, depending on the blend color.
/// The effect is similar to shining a diffused spotlight on the image.
/// If the blend color (light source) is lighter than 50% gray, the image is lightened as if it were dodged.
/// If the blend color is darker than 50% gray, the image is darkened as if it were burned in.
/// Painting with pure black or white produces a distinctly darker or lighter area, but does not result in pure black or white.
pub fn soft_light(a: RGBA, b: RGBA) -> RGBA {
  let blend_factor = 0.5; // Adjust this factor to control the blending intensity

  let red = if b.0 < 128 {
    ((2.0 * a.0 as f32 * b.0 as f32 / 255.0) * blend_factor + a.0 as f32 * (1.0 - blend_factor)) as u8
  } else {
    ((255.0 - 2.0 * (255.0 - a.0 as f32) * (255.0 - b.0 as f32) / 255.0) * blend_factor + a.0 as f32 * (1.0 - blend_factor)) as u8
  };
  let green = if b.1 < 128 {
    ((2.0 * a.1 as f32 * b.1 as f32 / 255.0) * blend_factor + a.1 as f32 * (1.0 - blend_factor)) as u8
  } else {
    ((255.0 - 2.0 * (255.0 - a.1 as f32) * (255.0 - b.1 as f32) / 255.0) * blend_factor + a.1 as f32 * (1.0 - blend_factor)) as u8
  };
  let blue = if b.2 < 128 {
    ((2.0 * a.2 as f32 * b.2 as f32 / 255.0) * blend_factor + a.2 as f32 * (1.0 - blend_factor)) as u8
  } else {
    ((255.0 - 2.0 * (255.0 - a.2 as f32) * (255.0 - b.2 as f32) / 255.0) * blend_factor + a.2 as f32 * (1.0 - blend_factor)) as u8
  };
  let alpha = if b.3 < 128 {
    ((2.0 * a.3 as f32 * b.3 as f32 / 255.0) * blend_factor + a.3 as f32 * (1.0 - blend_factor)) as u8
  } else {
    ((255.0 - 2.0 * (255.0 - a.3 as f32) * (255.0 - b.3 as f32) / 255.0) * blend_factor + a.3 as f32 * (1.0 - blend_factor)) as u8
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
pub fn hard_light(a: RGBA, b: RGBA) -> RGBA {
  let blend_channel = |a: u8, b: u8| -> u8 {
    if b < 128 {
      (2.0 * a as f32 * b as f32 / 255.0) as u8
    } else {
      (255.0 - 2.0 * (255.0 - a as f32) * (255.0 - b as f32) / 255.0) as u8
    }
  };

  let red = blend_channel(a.0, b.0);
  let green = blend_channel(a.1, b.1);
  let blue = blend_channel(a.2, b.2);
  let alpha = blend_channel(a.3, b.3);

  (red, green, blue, alpha)
}

/// Burns or dodges the colors by increasing or decreasing the contrast, depending on the blend color.
/// If the blend color (light source) is lighter than 50% gray, the image is lightened by decreasing the contrast.
/// If the blend color is darker than 50% gray, the image is darkened by increasing the contrast.
pub fn vivid_light(a: RGBA, b: RGBA) -> RGBA {
  let b_burn = (
    (2.0 * b.0 as f32) as u8,
    (2.0 * b.1 as f32) as u8,
    (2.0 * b.2 as f32) as u8,
    (2.0 * b.3 as f32) as u8,
  );
  let b_dodge = (
    (2.0 * (b.0 as f32 - 128.0)) as u8,
    (2.0 * (b.1 as f32 - 128.0)) as u8,
    (2.0 * (b.2 as f32 - 128.0)) as u8,
    (2.0 * (b.3 as f32 - 128.0)) as u8,
  );
  let red = if b.0 < 128 {
    color_burn(a, b_burn).0
  } else {
    color_dodge(a, b_dodge).0
  };
  let green = if b.1 < 128 {
    color_burn(a, b_burn).1
  } else {
    color_dodge(a, b_dodge).1
  };
  let blue = if b.2 < 128 {
    color_burn(a, b_burn).2
  } else {
    color_dodge(a, b_dodge).2
  };
  let alpha = if b.3 < 128 {
    color_burn(a, b_burn).3
  } else {
    color_dodge(a, b_dodge).3
  };
  (red as u8, green as u8, blue as u8, alpha as u8)
}

/// Burns or dodges the colors by decreasing or increasing the brightness, depending on the blend color.
/// If the blend color (light source) is lighter than 50% gray, the image is lightened by increasing the brightness.
/// If the blend color is darker than 50% gray, the image is darkened by decreasing the brightness.
pub fn linear_light(a: RGBA, b: RGBA) -> RGBA {
  let b_burn = (
    (2.0 * b.0 as f32) as u8,
    (2.0 * b.1 as f32) as u8,
    (2.0 * b.2 as f32) as u8,
    (2.0 * b.3 as f32) as u8,
  );
  let b_dodge = (
    (2.0 * (b.0 as f32 - 128.0)) as u8,
    (2.0 * (b.1 as f32 - 128.0)) as u8,
    (2.0 * (b.2 as f32 - 128.0)) as u8,
    (2.0 * (b.3 as f32 - 128.0)) as u8,
  );
  let red = if b.0 < 128 {
    linear_burn(a, b_burn).0
  } else {
    linear_dodge(a, b_dodge).0
  };
  let green = if b.1 < 128 {
    linear_burn(a, b_burn).1
  } else {
    linear_dodge(a, b_dodge).1
  };
  let blue = if b.2 < 128 {
    linear_burn(a, b_burn).2
  } else {
    linear_dodge(a, b_dodge).2
  };
  let alpha = if b.3 < 128 {
    linear_burn(a, b_burn).3
  } else {
    linear_dodge(a, b_dodge).3
  };
  (red, green, blue, alpha)
}

/// Replaces the colors, depending on the blend color.
/// If the blend color (light source) is lighter than 50% gray, pixels darker than the blend color are replaced, and pixels lighter than the blend color do not change.
/// If the blend color is darker than 50% gray, pixels lighter than the blend color are replaced, and pixels darker than the blend color do not change.
/// This is useful for adding special effects to an image.
pub fn pin_light(a: RGBA, b: RGBA) -> RGBA {
  let red = if b.0 < 128 { darken(a, b).0 } else { lighten(a, b).0 };
  let green = if b.1 < 128 { darken(a, b).1 } else { lighten(a, b).1 };
  let blue = if b.2 < 128 { darken(a, b).2 } else { lighten(a, b).2 };
  let alpha = if b.3 < 128 { darken(a, b).3 } else { lighten(a, b).3 };
  (red, green, blue, alpha)
}

/// Adds the red, green and blue channel values of the blend color to the RGB values of the base color.
/// If the resulting sum for a channel is 255 or greater, it receives a value of 255; if less than 255, a value of 0. Therefore, all blended pixels have red, green, and blue channel values of either 0 or 255.
/// This changes all pixels to primary additive colors (red, green, or blue), white, or black.
pub fn hard_mix(a: RGBA, b: RGBA) -> RGBA {
  let blended = vivid_light(a, b);
  let red = if blended.0 < 128 { 0 } else { 255 };
  let green = if blended.1 < 128 { 0 } else { 255 };
  let blue = if blended.2 < 128 { 0 } else { 255 };
  let alpha = if blended.3 < 128 { 0 } else { 255 };
  (red, green, blue, alpha)
}

/// Looks at the color information in each channel and subtracts either the blend
/// color from the base color or the base color from the blend color, depending on which has the greater brightness value.
/// Blending with white inverts the base color values; blending with black produces no change.
pub fn difference(a: RGBA, b: RGBA) -> RGBA {
  let red = (a.0 as i32 - b.0 as i32).abs() as u8;
  let green = (a.1 as i32 - b.1 as i32).abs() as u8;
  let blue = (a.2 as i32 - b.2 as i32).abs() as u8;
  (red, green, blue, a.3)
}

/// Creates an effect similar to but lower in contrast than the Difference mode.
/// Blending with white inverts the base color values.
/// Blending with black produces no change.
pub fn exclusion(a: RGBA, b: RGBA) -> RGBA {
  let red = a.0 as i32 + b.0 as i32 - 2 * a.0 as i32 * b.0 as i32 / 255;
  let green = a.1 as i32 + b.1 as i32 - 2 * a.1 as i32 * b.1 as i32 / 255;
  let blue = a.2 as i32 + b.2 as i32 - 2 * a.2 as i32 * b.2 as i32 / 255;
  (red as u8, green as u8, blue as u8, a.3)
}

/// Looks at the color information in each channel and subtracts the blend color from the base color.
/// In 8- and 16-bit images, any resulting negative values are clipped to zero.
pub fn subtract(a: RGBA, b: RGBA) -> RGBA {
  let red = (a.0 as i32 - b.0 as i32).max(0) as u8;
  let green = (a.1 as i32 - b.1 as i32).max(0) as u8;
  let blue = (a.2 as i32 - b.2 as i32).max(0) as u8;
  (red, green, blue, a.3)
}

/// Looks at the color information in each channel and divides the blend color from the base color.
/// If the blend color channel is zero, the result for that channel will be zero.
pub fn divide(a: RGBA, b: RGBA) -> RGBA {
  let red = if b.0 == 0 {
    0
  } else {
    (a.0 as f32 / b.0 as f32 * 255.0).round() as u8
  };
  let green = if b.1 == 0 {
    0
  } else {
    (a.1 as f32 / b.1 as f32 * 255.0).round() as u8
  };
  let blue = if b.2 == 0 {
    0
  } else {
    (a.2 as f32 / b.2 as f32 * 255.0).round() as u8
  };
  (red, green, blue, a.3)
}

/// Creates a result color with the luminance and saturation of the base color and the hue of the blend color.
pub fn hue(a: RGBA, b: RGBA) -> RGBA {
  let (_, s1, l1) = rgb_to_hsl(a.0, a.1, a.2);
  let (h2, _, _) = rgb_to_hsl(b.0, b.1, b.2);
  let (r, g, b) = hsl_to_rgb(h2, s1, l1);
  (r, g, b, a.3)
}

/// Creates a result color with the luminance and hue of the base color and the saturation of the blend color.
///  Painting with this mode in an area with no (0) saturation (gray) causes no change.
pub fn saturation(a: RGBA, b: RGBA) -> RGBA {
  let (h1, _, l1) = rgb_to_hsl(a.0, a.1, a.2);
  let (_, s2, _) = rgb_to_hsl(b.0, b.1, b.2);
  let (r, g, b) = hsl_to_rgb(h1, s2, l1);
  (r, g, b, a.3)
}
/// Creates a result color with the luminance of the base color and the hue and saturation of the blend color.
/// This preserves the gray levels in the image and is useful for coloring monochrome images and for tinting color images.
pub fn color(a: RGBA, b: RGBA) -> RGBA {
  let (_, _, l1) = rgb_to_hsl(a.0, a.1, a.2);
  let (h2, s2, _) = rgb_to_hsl(b.0, b.1, b.2);
  let (r, g, b) = hsl_to_rgb(h2, s2, l1);
  (r, g, b, a.3)
}

/// Creates a result color with the hue and saturation of the base color and the luminance of the blend color.
/// This mode creates the inverse effect of Color mode.
pub fn luminosity(a: RGBA, b: RGBA) -> RGBA {
  let (h1, s1, _) = rgb_to_hsl(a.0, a.1, a.2);
  let (_, _, l2) = rgb_to_hsl(b.0, b.1, b.2);
  let (r, g, b) = hsl_to_rgb(h1, s1, l2);
  (r, g, b, a.3)
}
