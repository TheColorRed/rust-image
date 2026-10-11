@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
struct Parameters { highlights: f32, shadows: f32, highlight_tone: f32, shadow_tone: f32, midtone: f32, pad0: f32, pad1: f32, pad2: f32 }
@group(0) @binding(2) var<uniform> parameters: Parameters;
@group(0) @binding(4) var base_tex: texture_2d<f32>;

fn decode(channel: f32) -> f32 {
  if channel <= 0.04045 { return channel / 12.92; }
  return pow((channel + 0.055) / 1.055, 2.4);
}

fn encode(channel: f32) -> f32 {
  if channel <= 0.0031308 { return 12.92 * channel; }
  return 1.055 * pow(channel, 1.0 / 2.4) - 0.055;
}

fn weight(value: f32, tone: f32) -> f32 {
  if tone == 0.0 { return 0.0; }
  let across = clamp(value / tone, 0.0, 1.0);
  return 1.0 - across * across * (3.0 - 2.0 * across);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if gid.x >= dims.x || gid.y >= dims.y { return; }
  let pos = vec2<i32>(gid.xy);
  let pixel = textureLoad(base_tex, pos, 0);
  let neighborhood = textureLoad(input_tex, pos, 0);
  let rgb = vec3<f32>(decode(pixel.r), decode(pixel.g), decode(pixel.b));
  let luminance = 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
  if luminance == 0.0 {
    textureStore(output_tex, pos, pixel);
    return;
  }
  let value = encode(luminance);
  let inverse = 1.0 - value;
  let shadow_weight = min(weight(value, parameters.shadow_tone), weight(neighborhood.r, parameters.shadow_tone));
  let highlight_weight = min(weight(inverse, parameters.highlight_tone), weight(1.0 - neighborhood.g, parameters.highlight_tone));
  let adjusted = value + 0.9 * value * inverse * (parameters.shadows * shadow_weight * inverse * inverse - parameters.highlights * highlight_weight * value * value);
  let target_luminance = decode(adjusted + 0.9 * parameters.midtone * adjusted * (1.0 - adjusted) * (2.0 * adjusted - 1.0));
  let gain = min(target_luminance / luminance, 1.0 / max(rgb.r, max(rgb.g, rgb.b)));
  let result = rgb * gain;
  textureStore(output_tex, pos, vec4<f32>(encode(result.r), encode(result.g), encode(result.b), pixel.a));
}
