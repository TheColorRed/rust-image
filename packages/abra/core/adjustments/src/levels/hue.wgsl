@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> amount: i32;

fn rgb_to_hsl(rgb: vec3<f32>) -> vec3<f32> {
  let maximum = max(rgb.r, max(rgb.g, rgb.b));
  let minimum = min(rgb.r, min(rgb.g, rgb.b));
  let delta = maximum - minimum;
  let lightness = (maximum + minimum) / 2.0;
  if (delta == 0.0) {
    return vec3<f32>(0.0, 0.0, lightness);
  }

  var hue: f32;
  if (maximum == rgb.r) {
    hue = (rgb.g - rgb.b) / delta + select(0.0, 6.0, rgb.g < rgb.b);
  } else if (maximum == rgb.g) {
    hue = (rgb.b - rgb.r) / delta + 2.0;
  } else {
    hue = (rgb.r - rgb.g) / delta + 4.0;
  }
  hue *= 60.0;

  var saturation: f32;
  if (lightness > 0.5) {
    saturation = delta / (2.0 - maximum - minimum);
  } else {
    saturation = delta / (maximum + minimum);
  }
  return vec3<f32>(hue, saturation, lightness);
}

fn hue_channel(p1: f32, q1: f32, hue_input: f32) -> f32 {
  var hue = hue_input;
  if (hue < 0.0) {
    hue += 1.0;
  }
  if (hue > 1.0) {
    hue -= 1.0;
  }
  if (hue < 1.0 / 6.0) {
    return p1 + (q1 - p1) * 6.0 * hue;
  }
  if (hue < 1.0 / 2.0) {
    return q1;
  }
  if (hue < 2.0 / 3.0) {
    return p1 + (q1 - p1) * (2.0 / 3.0 - hue) * 6.0;
  }
  return p1;
}

fn hsl_to_rgb(hsl: vec3<f32>) -> vec3<f32> {
  let hue = hsl.x / 360.0;
  let saturation = hsl.y;
  let lightness = hsl.z;
  if (saturation == 0.0) {
    return vec3<f32>(lightness);
  }
  var q = lightness * (1.0 + saturation);
  if (lightness >= 0.5) {
    q = lightness + saturation - lightness * saturation;
  }
  let p = 2.0 * lightness - q;
  return vec3<f32>(
    hue_channel(p, q, hue + 1.0 / 3.0),
    hue_channel(p, q, hue),
    hue_channel(p, q, hue - 1.0 / 3.0)
  );
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let p = textureLoad(input_tex, vec2<i32>(i32(gid.x), i32(gid.y)), 0);
  let hsl = rgb_to_hsl(p.rgb);
  let rotated_hue = (hsl.x + f32(amount)) % 360.0;
  let out_rgb = hsl_to_rgb(vec3<f32>(rotated_hue, hsl.y, hsl.z));
  textureStore(output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(out_rgb, p.a));
}
