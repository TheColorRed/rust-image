@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
// The mixing matrix. Each row is the weights, [red, green, blue, unused], of the source's channels that make one channel of
// the result. See `ChannelMix` in channel_mix.rs: this does the same sum in the same order, so the GPU and the CPU agree.
struct Weights {
  red: vec4<f32>,
  green: vec4<f32>,
  blue: vec4<f32>,
}
@group(0) @binding(2) var<uniform> weights: Weights;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) {
    return;
  }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let p = textureLoad(input_tex, pos, 0);
  let rgb = round(p.rgb * 255.0);
  let mixed = vec3<f32>(
    weights.red.x * rgb.r + weights.red.y * rgb.g + weights.red.z * rgb.b,
    weights.green.x * rgb.r + weights.green.y * rgb.g + weights.green.z * rgb.b,
    weights.blue.x * rgb.r + weights.blue.y * rgb.g + weights.blue.z * rgb.b,
  );
  // Clamp to a byte and round halves up, as the CPU does (WGSL `round` rounds halves to even).
  let out_rgb = floor(clamp(mixed, vec3<f32>(0.0), vec3<f32>(255.0)) + vec3<f32>(0.5)) / 255.0;
  textureStore(output_tex, pos, vec4<f32>(out_rgb, p.a));
}
