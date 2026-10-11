@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
fn decode(c: f32) -> f32 {
  if c <= 0.04045 { return c / 12.92; }
  return pow((c + 0.055) / 1.055, 2.4);
}
fn encode(c: f32) -> f32 {
  if c <= 0.0031308 { return 12.92 * c; }
  return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if gid.x >= dims.x || gid.y >= dims.y { return; }
  let pos = vec2<i32>(gid.xy);
  let p = textureLoad(input_tex, pos, 0);
  let gray = encode(0.2126 * decode(p.r) + 0.7152 * decode(p.g) + 0.0722 * decode(p.b));
  textureStore(output_tex, pos, vec4<f32>(gray, gray, 0.0, 1.0));
}
