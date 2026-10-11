// Every blend mode of `BlendMode` (blend.rs), and the source-over step that puts the result on the destination. Shaders that
// blend something over an image start with this file (it is joined to them at compile time, since WGSL has no include) and
// call `composite`. The mode is its index in `BlendMode::ALL`.
//
// Channels are whole numbers 0-255 held in `u32`, and the arithmetic is the CPU's in whole numbers where the CPU's float
// math gives the same answer exactly (a quotient is floored or rounded the way the float version is, using integer
// division), so those modes match the CPU bit for bit. Divide and the hue, saturation, color, luminosity, darker color and
// lighter color modes keep the CPU's float steps and can differ by a level.

fn mode_multiply(a: u32, b: u32) -> u32 {
  return a * b / 255u;
}

// 255 - n / 255, truncated, for n = (255 - a) * (255 - b).
fn mode_screen(a: u32, b: u32) -> u32 {
  let n = (255u - a) * (255u - b);
  return 255u - (n + 254u) / 255u;
}

fn mode_color_burn(a: u32, b: u32) -> u32 {
  if (b == 0u) {
    return 0u;
  }
  let n = (255u - a) * 255u;
  let q = (n + b - 1u) / b;
  if (q >= 255u) {
    return 0u;
  }
  return 255u - q;
}

fn mode_color_dodge(a: u32, b: u32) -> u32 {
  if (b == 255u) {
    return 255u;
  }
  return min(a * 255u / (255u - b), 255u);
}

fn mode_linear_burn(a: u32, b: u32) -> u32 {
  if (a + b > 255u) {
    return a + b - 255u;
  }
  return 0u;
}

fn mode_linear_dodge(a: u32, b: u32) -> u32 {
  return min(a + b, 255u);
}

fn mode_vivid_light(a: u32, b: u32) -> u32 {
  if (b < 128u) {
    return mode_color_burn(a, 2u * b);
  }
  return mode_color_dodge(a, 2u * (b - 128u));
}

// The multiply half and the screen half of overlay, soft light and hard light, as the CPU's `multiply_or_screen` and
// `screen2`. `screen_numerator` is `255 * 255 - 2 * (255 - a) * (255 - b)`, which is never negative where it is used.
fn screen_numerator(a: u32, b: u32) -> u32 {
  return 65025u - 2u * (255u - a) * (255u - b);
}

fn mode_overlay(a: u32, b: u32) -> u32 {
  if (a < 128u) {
    return (4u * a * b + 255u) / 510u;
  }
  return (2u * screen_numerator(a, b) + 255u) / 510u;
}

fn mode_soft_light(a: u32, b: u32) -> u32 {
  if (b < 128u) {
    return (2u * a * b + 255u * a) / 510u;
  }
  return (screen_numerator(a, b) + 255u * a) / 510u;
}

fn mode_hard_light(a: u32, b: u32) -> u32 {
  if (b < 128u) {
    return 2u * a * b / 255u;
  }
  return screen_numerator(a, b) / 255u;
}

fn mode_linear_light(a: u32, b: u32) -> u32 {
  if (b < 128u) {
    return mode_linear_burn(a, 2u * b);
  }
  return mode_linear_dodge(a, 2u * (b - 128u));
}

fn mode_pin_light(a: u32, b: u32) -> u32 {
  if (b < 128u) {
    return min(a, b);
  }
  return max(a, b);
}

fn mode_hard_mix(a: u32, b: u32) -> u32 {
  if (mode_vivid_light(a, b) < 128u) {
    return 0u;
  }
  return 255u;
}

fn mode_reflect(a: u32, b: u32) -> u32 {
  if (b == 255u) {
    return 255u;
  }
  return min(a * a / (255u - b), 255u);
}

fn mode_difference(a: u32, b: u32) -> u32 {
  if (a > b) {
    return a - b;
  }
  return b - a;
}

fn mode_exclusion(a: u32, b: u32) -> u32 {
  return a + b - 2u * a * b / 255u;
}

fn mode_subtract(a: u32, b: u32) -> u32 {
  if (a > b) {
    return a - b;
  }
  return 0u;
}

fn mode_divide(a: u32, b: u32) -> u32 {
  if (b == 0u) {
    return 0u;
  }
  return min(u32(floor(f32(a) / f32(b) * 255.0 + 0.5)), 255u);
}

// a + b - 2ab/255, truncated.
fn mode_phoenix(a: u32, b: u32) -> u32 {
  let subtract = (2u * a * b + 254u) / 255u;
  return u32(clamp(i32(a + b) - i32(subtract), 0, 255));
}

fn mode_negation(a: u32, b: u32) -> u32 {
  return 255u - mode_difference(a, b);
}

fn mode_grain_extract(a: u32, b: u32) -> u32 {
  return u32(clamp((i32(a) + i32(b) - 255) / 2 + 128, 0, 255));
}

fn mode_grain_merge(a: u32, b: u32) -> u32 {
  return u32(clamp((i32(a) + i32(b) - 128) / 2, 0, 255));
}

// The channel modes, applied to red, green and blue. `mode` is the index in `BlendMode::ALL`.
fn mode_channel(mode: u32, a: u32, b: u32) -> u32 {
  switch (mode) {
    case 1u: { return min(a, b); }
    case 3u: { return (a + b) / 2u; }
    case 4u: { return mode_multiply(a, b); }
    case 5u: { return mode_color_burn(a, b); }
    case 6u: { return mode_linear_burn(a, b); }
    case 7u: { return max(a, b); }
    case 9u: { return mode_screen(a, b); }
    case 10u: { return mode_color_dodge(a, b); }
    case 11u: { return mode_linear_dodge(a, b); }
    case 12u: { return mode_overlay(a, b); }
    case 13u: { return mode_soft_light(a, b); }
    case 14u: { return mode_hard_light(a, b); }
    case 15u: { return mode_vivid_light(a, b); }
    case 16u: { return mode_linear_light(a, b); }
    case 17u: { return mode_pin_light(a, b); }
    case 18u: { return mode_hard_mix(a, b); }
    case 19u: { return mode_difference(a, b); }
    case 20u: { return mode_exclusion(a, b); }
    case 21u: { return mode_subtract(a, b); }
    case 22u: { return mode_divide(a, b); }
    case 27u: { return mode_reflect(a, b); }
    case 28u: { return mode_reflect(b, a); }
    case 29u: { return mode_phoenix(a, b); }
    case 30u: { return mode_negation(a, b); }
    case 31u: { return mode_grain_extract(a, b); }
    case 32u: { return mode_grain_merge(a, b); }
    default: { return b; }
  }
}

// RGB (0-255) to (hue in degrees, saturation, lightness), as `rgb_to_hsl` in the primitives crate.
fn rgb_to_hsl(c: vec3<u32>) -> vec3<f32> {
  let rf = f32(c.x) / 255.0;
  let gf = f32(c.y) / 255.0;
  let bf = f32(c.z) / 255.0;
  let high = max(rf, max(gf, bf));
  let low = min(rf, min(gf, bf));
  let d = high - low;
  var h = 0.0;
  if (d != 0.0) {
    if (high == rf) {
      h = (gf - bf) / d;
      if (gf < bf) {
        h = h + 6.0;
      }
    } else if (high == gf) {
      h = (bf - rf) / d + 2.0;
    } else {
      h = (rf - gf) / d + 4.0;
    }
    h = h * 60.0;
  }
  let l = (high + low) / 2.0;
  var s = 0.0;
  if (d != 0.0) {
    if (l > 0.5) {
      s = d / (2.0 - high - low);
    } else {
      s = d / (high + low);
    }
  }
  return vec3<f32>(h, s, l);
}

// (hue, saturation, lightness) to RGB (0-255), as `hsl_to_rgb` in the primitives crate.
fn hsl_to_rgb(hsl: vec3<f32>) -> vec3<u32> {
  let chroma = (1.0 - abs(2.0 * hsl.z - 1.0)) * hsl.y;
  let m = hsl.z - chroma / 2.0;
  let x = chroma * (1.0 - abs((hsl.x / 60.0) % 2.0 - 1.0));
  var rgb = vec3<f32>(chroma, 0.0, x);
  if (hsl.x < 60.0) {
    rgb = vec3<f32>(chroma, x, 0.0);
  } else if (hsl.x < 120.0) {
    rgb = vec3<f32>(x, chroma, 0.0);
  } else if (hsl.x < 180.0) {
    rgb = vec3<f32>(0.0, chroma, x);
  } else if (hsl.x < 240.0) {
    rgb = vec3<f32>(0.0, x, chroma);
  } else if (hsl.x < 300.0) {
    rgb = vec3<f32>(x, 0.0, chroma);
  }
  return vec3<u32>(floor(clamp((rgb + vec3<f32>(m)) * 255.0, vec3<f32>(0.0), vec3<f32>(255.0)) + vec3<f32>(0.5)));
}

// Hue, saturation and lightness taken from the source where `from_source` is set, from the destination elsewhere.
fn hsl_mix(a: vec3<u32>, b: vec3<u32>, from_source: vec3<bool>) -> vec3<u32> {
  return hsl_to_rgb(select(rgb_to_hsl(a), rgb_to_hsl(b), from_source));
}

// The blend color for the destination `a` and the source `b`, both red, green and blue as whole numbers 0-255.
fn blend_rgb(mode: u32, a: vec3<u32>, b: vec3<u32>) -> vec3<u32> {
  switch (mode) {
    case 0u: { return b; }
    case 2u: {
      if (rgb_to_hsl(a).z < rgb_to_hsl(b).z) { return a; }
      return b;
    }
    case 8u: {
      if (rgb_to_hsl(a).z > rgb_to_hsl(b).z) { return a; }
      return b;
    }
    case 23u: { return hsl_mix(a, b, vec3<bool>(true, false, false)); }
    case 24u: { return hsl_mix(a, b, vec3<bool>(false, true, false)); }
    case 25u: { return hsl_mix(a, b, vec3<bool>(true, true, false)); }
    case 26u: { return hsl_mix(a, b, vec3<bool>(false, false, true)); }
    default: {
      return vec3<u32>(mode_channel(mode, a.x, b.x), mode_channel(mode, a.y, b.y), mode_channel(mode, a.z, b.z));
    }
  }
}

// Puts `src` over `dst` with the blend mode, as `BlendImage::apply` does for one pixel: both are RGBA as whole numbers
// 0-255 held in floats, `opacity` is 0 to 1 on top of the source's own alpha, and the result is the same kind of value.
fn composite(dst: vec4<f32>, src: vec4<f32>, opacity: f32, mode: u32) -> vec4<f32> {
  let color = vec3<f32>(blend_rgb(mode, vec3<u32>(dst.rgb), vec3<u32>(src.rgb)));
  let source_alpha = src.a / 255.0 * opacity;
  let dst_alpha = dst.a / 255.0;
  let out_alpha = source_alpha + dst_alpha * (1.0 - source_alpha);
  if (out_alpha == 0.0) {
    return vec4<f32>(0.0);
  }
  let rgb = floor((color * source_alpha + dst.rgb * dst_alpha * (1.0 - source_alpha)) / out_alpha + 0.5);
  return vec4<f32>(rgb, floor(out_alpha * 255.0 + 0.5));
}
