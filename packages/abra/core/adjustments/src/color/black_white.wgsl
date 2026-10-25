@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> settings: array<vec4<f32>, 3>;

fn range_weights(rgb: vec3<f32>) -> array<f32, 6> {
  let maximum = max(rgb.r, max(rgb.g, rgb.b));
  let minimum = min(rgb.r, min(rgb.g, rgb.b));
  let middle = rgb.r + rgb.g + rgb.b - maximum - minimum;
  var weights: array<f32, 6>;
  for (var i = 0u; i < 6u; i++) { weights[i] = 0.0; }
  if (rgb.r >= rgb.g && rgb.r >= rgb.b) { weights[0] = maximum - middle; }
  if (rgb.g >= rgb.r && rgb.g >= rgb.b) { weights[2] = maximum - middle; }
  if (rgb.b >= rgb.r && rgb.b >= rgb.g) { weights[4] = maximum - middle; }
  if (rgb.b <= rgb.r && rgb.b <= rgb.g) { weights[1] = middle - minimum; }
  if (rgb.r <= rgb.g && rgb.r <= rgb.b) { weights[3] = middle - minimum; }
  if (rgb.g <= rgb.r && rgb.g <= rgb.b) { weights[5] = middle - minimum; }
  return weights;
}

fn setting(index: u32) -> f32 {
  if (index < 4u) { return settings[0][index]; }
  return settings[1][index - 4u];
}

fn hue_channel(p: f32, q: f32, input_hue: f32) -> f32 {
  var hue = input_hue;
  if (hue < 0.0) { hue += 1.0; }
  if (hue > 1.0) { hue -= 1.0; }
  if (hue < 1.0 / 6.0) { return p + (q - p) * 6.0 * hue; }
  if (hue < 0.5) { return q; }
  if (hue < 2.0 / 3.0) { return p + (q - p) * (2.0 / 3.0 - hue) * 6.0; }
  return p;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let p = textureLoad(input_tex, pos, 0);
  let weights = range_weights(p.rgb);
  var shift = 0.0;
  for (var i = 0u; i < 6u; i++) { shift += weights[i] * setting(i); }
  let value = clamp(dot(p.rgb, vec3<f32>(0.299, 0.587, 0.114)) + shift / 100.0, 0.0, 1.0);
  var output_rgb = vec3<f32>(value);
  if (settings[2].z > 0.5) {
    let hue = ((settings[2].x % 360.0 + 360.0) % 360.0) / 360.0;
    let saturation = clamp(settings[2].y, 0.0, 1.0);
    if (saturation > 0.0) {
      var q = value * (1.0 + saturation);
      if (value >= 0.5) { q = value + saturation - value * saturation; }
      let low = 2.0 * value - q;
      output_rgb = vec3<f32>(hue_channel(low, q, hue + 1.0 / 3.0), hue_channel(low, q, hue), hue_channel(low, q, hue - 1.0 / 3.0));
    }
  }
  textureStore(output_tex, pos, vec4<f32>(output_rgb, p.a));
}
