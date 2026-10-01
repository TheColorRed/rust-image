@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> threshold: f32;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(input_tex);
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
  let pos = vec2<i32>(i32(gid.x), i32(gid.y));
  let p = textureLoad(input_tex, pos, 0);
  // The same 0-255 scale as the CPU and the uniform, so pixels at the threshold land on the same side.
  let rgb = round(p.rgb * 255.0);
  let avg = (rgb.r + rgb.g + rgb.b) / 3.0;
  let result = select(0.0, 1.0, avg > threshold);
  textureStore(output_tex, pos, vec4<f32>(vec3<f32>(result), p.a));
}