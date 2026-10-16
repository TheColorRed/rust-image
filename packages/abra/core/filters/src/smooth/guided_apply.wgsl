// The last pass of the guided filter. It reads the averaged line (a, b) of each pixel and the guide photo at binding 3, and writes
// the refined mask, a * brightness + b, as an ordinary opaque gray pixel.
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var guide_tex: texture_2d<f32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(input_tex));
  let pos = vec2<i32>(gid.xy);
  if (pos.x >= dims.x || pos.y >= dims.y) {
    return;
  }
  let line = textureLoad(input_tex, pos, 0);
  let guide = textureLoad(guide_tex, pos, 0).rgb;
  let brightness = dot(guide, vec3<f32>(0.299, 0.587, 0.114));
  let value = clamp(line.x * brightness + line.y, 0.0, 1.0);
  textureStore(output_tex, pos, vec4<f32>(value, value, value, 1.0));
}
